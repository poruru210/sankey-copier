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
#include "Logging.mqh"

//+------------------------------------------------------------------+
//| Send open position signal message (Master)                       |
//| Called when Master EA opens a new position to notify Slaves      |
//+------------------------------------------------------------------+
bool SendOpenSignal(EaContextWrapper &ea_context, TICKET_TYPE ticket, string symbol,
                    string order_type, double lots, double price, double sl, double tp,
                    long magic, string comment, string account_id)
{
   return ea_context.SendOpenSignal((long)ticket, symbol, order_type, lots, price, sl, tp, magic, comment);
}

//+------------------------------------------------------------------+
//| Send close signal message (Master)                               |
//| Called when Master EA closes a position to notify Slaves         |
//| close_ratio: 0 or >= 1.0 = full close, 0 < ratio < 1.0 = partial |
//+------------------------------------------------------------------+
bool SendCloseSignal(EaContextWrapper &ea_context, TICKET_TYPE ticket, double close_ratio, string account_id)
{
   return ea_context.SendCloseSignal((long)ticket, close_ratio);
}

//+------------------------------------------------------------------+
//| Send modify signal message (Master)                             |
//| Called when Master EA modifies SL/TP to notify Slaves            |
//+------------------------------------------------------------------+
bool SendModifySignal(EaContextWrapper &ea_context, TICKET_TYPE ticket, double sl, double tp, string account_id)
{
   return ea_context.SendModifySignal((long)ticket, sl, tp);
}

//+------------------------------------------------------------------+
//| Send position snapshot message (Master)                          |
//| Called when Master receives SyncRequest from Slave               |
//| Collects all current positions and sends as PositionSnapshot      |
//+------------------------------------------------------------------+
bool SendPositionSnapshot(EaContextWrapper &ea_context, string account_id, string symbol_prefix, string symbol_suffix)
{
   // Create snapshot builder
   HANDLE_TYPE builder = create_position_snapshot_builder(account_id);
   if(builder == 0 || builder == -1)
   {
      LogError(CAT_SYNC, "Failed to create position snapshot builder");
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
      LogError(CAT_SYNC, "Failed to serialize position snapshot");
      return false;
   }

   // Resize to actual length
   ArrayResize(buffer, len);

   // Send via EaContext
   bool success = ea_context.SendPush(buffer, len);

   if(success)
      LogInfo(CAT_SYNC, StringFormat("Snapshot sent: %d positions (%d bytes)", position_count, len));
   else
      LogError(CAT_SYNC, "Failed to send position snapshot");

   return success;
}

#endif // SANKEY_COPIER_MASTER_SIGNALS_MQH
