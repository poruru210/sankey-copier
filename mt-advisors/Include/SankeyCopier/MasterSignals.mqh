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
   SPositionInfo positions[];
   int count = 0;

   #ifdef IS_MT5
      // MT5: Iterate through all positions
      int total = PositionsTotal();
      ArrayResize(positions, total); // Pre-allocate maximum size

      for(int i = 0; i < total; i++)
      {
         ulong ticket = PositionGetTicket(i);
         if(ticket > 0 && PositionSelectByTicket(ticket))
         {
            string raw_symbol = PositionGetString(POSITION_SYMBOL);

            // Master sends ALL positions - prefix/suffix is only used for symbol name cleaning
            string symbol = GetCleanSymbol(raw_symbol, symbol_prefix, symbol_suffix);

            long type = PositionGetInteger(POSITION_TYPE);
            double lots = PositionGetDouble(POSITION_VOLUME);
            double price = PositionGetDouble(POSITION_PRICE_OPEN);
            double sl = PositionGetDouble(POSITION_SL);
            double tp = PositionGetDouble(POSITION_TP);
            long magic = PositionGetInteger(POSITION_MAGIC);
            datetime open_time = (datetime)PositionGetInteger(POSITION_TIME);
            string comment = PositionGetString(POSITION_COMMENT);

            // Populate struct
            positions[count].ticket = (long)ticket;
            StringToCharArray(symbol, positions[count].symbol);
            positions[count].order_type = (int)type; // Assuming standard Enum mapping (MT5)
            positions[count].lots = lots;
            positions[count].open_price = price;
            positions[count].open_time = (long)open_time;
            positions[count].stop_loss = sl;
            positions[count].take_profit = tp;
            positions[count].magic_number = magic;
            StringToCharArray(comment, positions[count].comment);

            count++;
         }
      }
      ArrayResize(positions, count); // Resize to actual count
   #else
      // MT4: Iterate through all open orders (MODE_TRADES)
      int total = OrdersTotal();
      ArrayResize(positions, total); // Pre-allocate maximum

      for(int i = 0; i < total; i++)
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
            string symbol = GetCleanSymbol(raw_symbol, symbol_prefix, symbol_suffix);

            double lots = OrderLots();
            double price = OrderOpenPrice();
            double sl = OrderStopLoss();
            double tp = OrderTakeProfit();
            int magic = OrderMagicNumber();
            datetime open_time = OrderOpenTime();
            string comment = OrderComment();

            // Populate struct
            positions[count].ticket = (long)ticket;
            StringToCharArray(symbol, positions[count].symbol);
            positions[count].order_type = type; // Assuming standard Enum mapping (MT4)
            positions[count].lots = lots;
            positions[count].open_price = price;
            positions[count].open_time = (long)open_time;
            positions[count].stop_loss = sl;
            positions[count].take_profit = tp;
            positions[count].magic_number = (long)magic;
            StringToCharArray(comment, positions[count].comment);

            count++;
         }
      }
      ArrayResize(positions, count); // Resize to actual count
   #endif

   // Send via EaContext
   bool success = ea_context.SendPositionSnapshot(positions);

   if(success)
      LogInfo(CAT_SYNC, StringFormat("Snapshot sent: %d positions", count));
   else
      LogError(CAT_SYNC, "Failed to send position snapshot");

   return success;
}

#endif // SANKEY_COPIER_MASTER_SIGNALS_MQH
