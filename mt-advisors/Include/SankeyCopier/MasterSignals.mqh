//+------------------------------------------------------------------+
//|                                  SankeyCopierMasterSignals.mqh   |
//|                        Copyright 2025, SANKEY Copier Project      |
//|                     Master EA specific signal functions           |
//+------------------------------------------------------------------+
// Purpose: Contains functions for Master EA to send trade signals
// Why: Separates Master-specific code from common shared code
//      to improve code organization and maintainability
#property copyright "Copyright 2025, SANKEY Copier Project"
#property strict

#ifndef SANKEY_COPIER_MASTER_SIGNALS_MQH
#define SANKEY_COPIER_MASTER_SIGNALS_MQH

#include "Common.mqh"

//+------------------------------------------------------------------+
//| Send open position signal message (Master)                       |
//| Called when Master EA opens a new position to notify Slaves      |
//+------------------------------------------------------------------+
bool SendOpenSignal(HANDLE_TYPE zmq_socket, TICKET_TYPE ticket, string symbol,
                    string order_type, double lots, double price, double sl, double tp,
                    long magic, string comment, string account_id)
{
   // Serialize open signal message using MessagePack
   // close_ratio = 0.0 for Open signals (not a partial close)
   int len = serialize_trade_signal("Open", (long)ticket, symbol, order_type,
                                            lots, price, sl, tp, magic, comment,
                                            FormatTimestampISO8601(TimeGMT()), account_id, 0.0);

   if(len <= 0)
   {
      Print("[ERROR] Failed to serialize open signal message");
      return false;
   }

   // Copy serialized data to buffer
   uchar buffer[];
   ArrayResize(buffer, len);
   int copied = copy_serialized_buffer(buffer, len);

   if(copied != len)
   {
      Print("[ERROR] Failed to copy open signal message buffer");
      return false;
   }

   // Send binary MessagePack data
   return (zmq_socket_send_binary(zmq_socket, buffer, len) == 1);
}

//+------------------------------------------------------------------+
//| Send close signal message (Master)                               |
//| Called when Master EA closes a position to notify Slaves         |
//| close_ratio: 0 or >= 1.0 = full close, 0 < ratio < 1.0 = partial |
//+------------------------------------------------------------------+
bool SendCloseSignal(HANDLE_TYPE zmq_socket, TICKET_TYPE ticket, double close_ratio, string account_id)
{
   // For close signals, we send a trade signal with action="Close"
   // close_ratio indicates what portion was closed (0 = full close for backward compat)
   int len = serialize_trade_signal("Close", (long)ticket, "", "", 0.0, 0.0, 0.0, 0.0,
                                            0, "", FormatTimestampISO8601(TimeGMT()), account_id, close_ratio);

   if(len <= 0)
   {
      Print("[ERROR] Failed to serialize close signal message");
      return false;
   }

   // Copy serialized data to buffer
   uchar buffer[];
   ArrayResize(buffer, len);
   int copied = copy_serialized_buffer(buffer, len);

   if(copied != len)
   {
      Print("[ERROR] Failed to copy close signal message buffer");
      return false;
   }

   // Send binary MessagePack data
   return (zmq_socket_send_binary(zmq_socket, buffer, len) == 1);
}

//+------------------------------------------------------------------+
//| Send modify signal message (Master)                             |
//| Called when Master EA modifies SL/TP to notify Slaves            |
//+------------------------------------------------------------------+
bool SendModifySignal(HANDLE_TYPE zmq_socket, TICKET_TYPE ticket, double sl, double tp, string account_id)
{
   // For modify signals, we send a trade signal with action="Modify"
   // Only ticket, stop_loss, take_profit, timestamp, and source_account are needed
   // close_ratio = 0.0 for Modify signals (not a close operation)
   int len = serialize_trade_signal("Modify", (long)ticket, "", "", 0.0, 0.0, sl, tp,
                                            0, "", FormatTimestampISO8601(TimeGMT()), account_id, 0.0);

   if(len <= 0)
   {
      Print("[ERROR] Failed to serialize modify signal message");
      return false;
   }

   // Copy serialized data to buffer
   uchar buffer[];
   ArrayResize(buffer, len);
   int copied = copy_serialized_buffer(buffer, len);

   if(copied != len)
   {
      Print("[ERROR] Failed to copy modify signal message buffer");
      return false;
   }

   // Send binary MessagePack data
   return (zmq_socket_send_binary(zmq_socket, buffer, len) == 1);
}

//+------------------------------------------------------------------+
//| Send position snapshot message (Master)                          |
//| Called when Master receives SyncRequest from Slave               |
//| Collects all current positions and sends as PositionSnapshot      |
//+------------------------------------------------------------------+
bool SendPositionSnapshot(HANDLE_TYPE zmq_socket, string account_id, string symbol_prefix, string symbol_suffix)
{
   // Create snapshot builder
   HANDLE_TYPE builder = create_position_snapshot_builder(account_id);
   if(builder == 0 || builder == -1)
   {
      Print("[ERROR] Failed to create position snapshot builder");
      return false;
   }

   int position_count = 0;

   #ifdef IS_MT5
      // MT5: Iterate through all positions
      for(int i = 0; i < PositionsTotal(); i++)
      {
         ulong ticket = PositionGetTicket(i);
         if(ticket > 0 && PositionSelectByTicket(ticket))
         {
            string raw_symbol = PositionGetString(POSITION_SYMBOL);

            // Master sends ALL positions - prefix/suffix is only used for symbol name cleaning
            // Clean symbol (remove prefix/suffix if present)
            string symbol = GetCleanSymbol(raw_symbol, symbol_prefix, symbol_suffix);

            long type = PositionGetInteger(POSITION_TYPE);
            double lots = PositionGetDouble(POSITION_VOLUME);
            double price = PositionGetDouble(POSITION_PRICE_OPEN);
            double sl = PositionGetDouble(POSITION_SL);
            double tp = PositionGetDouble(POSITION_TP);
            long magic = PositionGetInteger(POSITION_MAGIC);
            datetime open_time = (datetime)PositionGetInteger(POSITION_TIME);

            string order_type = GetOrderTypeString((ENUM_POSITION_TYPE)type);
            string open_time_str = FormatTimestampISO8601(open_time);

            int result = position_snapshot_builder_add_position(builder, (long)ticket, symbol, order_type,
                                                                  lots, price, sl, tp, magic, open_time_str);
            if(result == 1)
               position_count++;
         }
      }
   #else
      // MT4: Iterate through all open orders (MODE_TRADES)
      for(int i = 0; i < OrdersTotal(); i++)
      {
         if(OrderSelect(i, SELECT_BY_POS, MODE_TRADES))
         {
            int type = OrderType();
            // Only include market orders (OP_BUY, OP_SELL)
            if(type != OP_BUY && type != OP_SELL)
               continue;

            int ticket = OrderTicket();
            string raw_symbol = OrderSymbol();

            // Master sends ALL positions - prefix/suffix is only used for symbol name cleaning
            // Clean symbol (remove prefix/suffix if present)
            string symbol = GetCleanSymbol(raw_symbol, symbol_prefix, symbol_suffix);

            double lots = OrderLots();
            double price = OrderOpenPrice();
            double sl = OrderStopLoss();
            double tp = OrderTakeProfit();
            int magic = OrderMagicNumber();
            datetime open_time = OrderOpenTime();

            string order_type = GetOrderTypeString(type);
            string open_time_str = FormatTimestampISO8601(open_time);

            int result = position_snapshot_builder_add_position(builder, (long)ticket, symbol, order_type,
                                                                  lots, price, sl, tp, magic, open_time_str);
            if(result == 1)
               position_count++;
         }
      }
   #endif

   // Serialize the snapshot
   uchar buffer[];
   ArrayResize(buffer, MESSAGE_BUFFER_SIZE);
   int len = position_snapshot_builder_serialize(builder, buffer, MESSAGE_BUFFER_SIZE);

   // Free the builder
   position_snapshot_builder_free(builder);

   if(len <= 0)
   {
      Print("[ERROR] Failed to serialize position snapshot");
      return false;
   }

   // Resize to actual length
   ArrayResize(buffer, len);

   // Send binary MessagePack data
   bool success = (zmq_socket_send_binary(zmq_socket, buffer, len) == 1);

   if(success)
      Print("[SYNC] Snapshot sent: ", position_count, " positions (", len, " bytes)");
   else
      Print("[ERROR] Failed to send position snapshot");

   return success;
}

#endif // SANKEY_COPIER_MASTER_SIGNALS_MQH
