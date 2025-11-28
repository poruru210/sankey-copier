//+------------------------------------------------------------------+
//|                                         SankeyCopierSlaveTrade.mqh |
//|                        Copyright 2025, SANKEY Copier Project        |
//|                                                                    |
//| Purpose: Platform-agnostic trade execution functions for Slave EA  |
//| Why: Eliminates ~400 LOC duplication between MT4 and MT5 Slave EAs |
//|      by providing unified interface with platform-specific impl.    |
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, SANKEY Copier Project"
#property strict

#ifndef SANKEY_COPIER_SLAVE_TRADE_MQH
#define SANKEY_COPIER_SLAVE_TRADE_MQH

#include "Common.mqh"
#include "Mapping.mqh"

// =============================================================================
// External References
// =============================================================================
// These globals must be defined in the EA file that includes this header:
//   - g_order_map[]           : TicketMapping array for active positions
//   - g_pending_order_map[]   : PendingTicketMapping array for pending orders
//   - Slippage                : Default slippage in points (input parameter)
//   - MaxRetries              : Max retry attempts for order operations
//   - MaxSignalDelayMs        : Max acceptable signal delay in milliseconds
//   - UsePendingOrderForDelayed: Use pending order for delayed signals
// For MT5 only:
//   - g_trade                 : CTrade object for trade operations

// =============================================================================
// Platform-Specific Order Type Conversion
// =============================================================================

#ifdef IS_MT4
//+------------------------------------------------------------------+
//| Get order type from string (MT4)                                  |
//| Accepts PascalCase format matching relay-server's OrderType enum |
//| Returns: OP_* constant or -1 if invalid                          |
//+------------------------------------------------------------------+
int GetOrderTypeFromString(string type_str)
{
   if(type_str == "Buy")       return OP_BUY;
   if(type_str == "Sell")      return OP_SELL;
   if(type_str == "BuyLimit")  return OP_BUYLIMIT;
   if(type_str == "SellLimit") return OP_SELLLIMIT;
   if(type_str == "BuyStop")   return OP_BUYSTOP;
   if(type_str == "SellStop")  return OP_SELLSTOP;
   return -1;
}
#endif

#ifdef IS_MT5
//+------------------------------------------------------------------+
//| Get order type from string (MT5)                                  |
//| Accepts PascalCase format matching relay-server's OrderType enum |
//| Returns: ENUM_ORDER_TYPE or -1 cast if invalid                   |
//+------------------------------------------------------------------+
ENUM_ORDER_TYPE GetOrderTypeFromString(string type_str)
{
   if(type_str == "Buy")       return ORDER_TYPE_BUY;
   if(type_str == "Sell")      return ORDER_TYPE_SELL;
   if(type_str == "BuyLimit")  return ORDER_TYPE_BUY_LIMIT;
   if(type_str == "SellLimit") return ORDER_TYPE_SELL_LIMIT;
   if(type_str == "BuyStop")   return ORDER_TYPE_BUY_STOP;
   if(type_str == "SellStop")  return ORDER_TYPE_SELL_STOP;
   return (ENUM_ORDER_TYPE)-1;
}
#endif

// Note: NormalizeLotSize is provided by Trade.mqh

// =============================================================================
// Trade Execution Functions - MT5 Implementation
// =============================================================================

#ifdef IS_MT5

//+------------------------------------------------------------------+
//| Open position (MT5)                                               |
//+------------------------------------------------------------------+
void ExecuteOpenTrade(CTrade &trade, TicketMapping &order_map[], PendingTicketMapping &pending_map[],
                      ulong master_ticket, string symbol, string type_str,
                      double lots, double price, double sl, double tp, string timestamp,
                      string source_account, int magic, int slippage_points,
                      int max_signal_delay_ms, bool use_pending_for_delayed, int max_retries, int default_slippage)
{
   if(GetSlaveTicketFromMapping(order_map, master_ticket) > 0)
   {
      Print("Already copied master #", master_ticket);
      return;
   }

   // Check signal delay
   datetime signal_time = ParseISO8601(timestamp);
   datetime current_time = TimeGMT();
   int delay_ms = (int)((current_time - signal_time) * 1000);

   if(delay_ms > max_signal_delay_ms)
   {
      if(!use_pending_for_delayed)
      {
         Print("Signal too old (", delay_ms, "ms > ", max_signal_delay_ms, "ms). Skipping master #", master_ticket);
         return;
      }
      else
      {
         Print("Signal delayed (", delay_ms, "ms). Using pending order at original price ", price);
         ExecutePendingOrder(trade, pending_map, master_ticket, symbol, type_str, lots, price, sl, tp,
                            source_account, delay_ms, magic);
         return;
      }
   }

   ENUM_ORDER_TYPE order_type = GetOrderTypeFromString(type_str);
   if((int)order_type == -1) return;

   lots = NormalizeDouble(lots, 2);
   price = NormalizeDouble(price, _Digits);
   sl = (sl > 0) ? NormalizeDouble(sl, _Digits) : 0;
   tp = (tp > 0) ? NormalizeDouble(tp, _Digits) : 0;

   string comment = BuildMarketComment(master_ticket);

   trade.SetExpertMagicNumber(magic);
   int effective_slippage = (slippage_points > 0) ? slippage_points : default_slippage;
   trade.SetDeviationInPoints(effective_slippage);

   bool result = false;

   for(int i = 0; i < max_retries; i++)
   {
      if(order_type == ORDER_TYPE_BUY)
         result = trade.Buy(lots, symbol, 0, sl, tp, comment);
      else if(order_type == ORDER_TYPE_SELL)
         result = trade.Sell(lots, symbol, 0, sl, tp, comment);

      if(result)
      {
         ulong ticket = trade.ResultOrder();
         Print("Position opened: #", ticket, " from master #", master_ticket,
               " (delay: ", delay_ms, "ms, slippage: ", effective_slippage, " pts)");
         AddTicketMapping(order_map, master_ticket, ticket);
         break;
      }
      else
      {
         Print("Failed to open position, attempt ", i+1, "/", max_retries);
         Sleep(1000);
      }
   }
}

//+------------------------------------------------------------------+
//| Close position (MT5)                                              |
//| close_ratio: 0 or >= 1.0 = full close, 0 < ratio < 1.0 = partial |
//+------------------------------------------------------------------+
void ExecuteCloseTrade(CTrade &trade, TicketMapping &order_map[],
                       ulong master_ticket, double close_ratio,
                       int slippage_points, int default_slippage)
{
   ulong slave_ticket = GetSlaveTicketFromMapping(order_map, master_ticket);
   if(slave_ticket == 0)
   {
      Print("No slave position for master #", master_ticket);
      return;
   }

   if(!PositionSelectByTicket(slave_ticket))
   {
      Print("Position #", slave_ticket, " not found");
      RemoveTicketMapping(order_map, master_ticket);
      return;
   }

   int effective_slippage = (slippage_points > 0) ? slippage_points : default_slippage;
   trade.SetDeviationInPoints(effective_slippage);

   string symbol = PositionGetString(POSITION_SYMBOL);
   double current_lots = PositionGetDouble(POSITION_VOLUME);

   // Determine if this is a partial close or full close
   bool is_partial_close = (close_ratio > 0.0 && close_ratio < 1.0);

   if(is_partial_close)
   {
      // Partial close: apply close_ratio to current position volume
      double close_lots = NormalizeLotSize(current_lots * close_ratio, symbol);

      // Ensure close_lots is valid (at least minimum lot size)
      if(close_lots <= 0.0)
      {
         Print("Partial close lots too small, skipping. Ratio: ", close_ratio, " Current: ", current_lots);
         return;
      }

      if(trade.PositionClosePartial(slave_ticket, close_lots))
      {
         Print("Partial close: #", slave_ticket, " closed ", close_lots, " lots (",
               (close_ratio * 100.0), "%), remaining: ", (current_lots - close_lots), " lots");
         // Keep mapping - position still open with remaining lots
      }
      else
      {
         Print("Failed to partial close position #", slave_ticket, ", lots: ", close_lots);
      }
   }
   else
   {
      // Full close
      if(trade.PositionClose(slave_ticket))
      {
         Print("Position closed: #", slave_ticket, " (slippage: ", effective_slippage, " pts)");
         RemoveTicketMapping(order_map, master_ticket);
      }
      else
      {
         Print("Failed to close position #", slave_ticket);
      }
   }
}

//+------------------------------------------------------------------+
//| Modify position (MT5)                                             |
//+------------------------------------------------------------------+
void ExecuteModifyTrade(CTrade &trade, TicketMapping &order_map[],
                        ulong master_ticket, double sl, double tp)
{
   ulong slave_ticket = GetSlaveTicketFromMapping(order_map, master_ticket);
   if(slave_ticket == 0) return;

   if(!PositionSelectByTicket(slave_ticket)) return;

   if(trade.PositionModify(slave_ticket, sl, tp))
   {
      Print("Position modified: #", slave_ticket);
   }
}

//+------------------------------------------------------------------+
//| Place pending order (MT5)                                         |
//+------------------------------------------------------------------+
void ExecutePendingOrder(CTrade &trade, PendingTicketMapping &pending_map[],
                         ulong master_ticket, string symbol, string type_str,
                         double lots, double price, double sl, double tp,
                         string source_account, int delay_ms, int magic)
{
   if(GetPendingTicketFromMapping(pending_map, master_ticket) > 0)
   {
      Print("Pending order already exists for master #", master_ticket);
      return;
   }

   ENUM_ORDER_TYPE order_type = GetOrderTypeFromString(type_str);
   if((int)order_type == -1) return;

   ENUM_ORDER_TYPE pending_type;
   double current_price;

   if(order_type == ORDER_TYPE_BUY)
   {
      current_price = SymbolInfoDouble(symbol, SYMBOL_ASK);
      pending_type = (price < current_price) ? ORDER_TYPE_BUY_LIMIT : ORDER_TYPE_BUY_STOP;
   }
   else
   {
      current_price = SymbolInfoDouble(symbol, SYMBOL_BID);
      pending_type = (price > current_price) ? ORDER_TYPE_SELL_LIMIT : ORDER_TYPE_SELL_STOP;
   }

   lots = NormalizeDouble(lots, 2);
   string comment = BuildPendingComment(master_ticket);

   trade.SetExpertMagicNumber(magic);

   bool result = trade.OrderOpen(symbol, pending_type, lots, 0, price, sl, tp,
                                  ORDER_TIME_GTC, 0, comment);

   if(result)
   {
      ulong ticket = trade.ResultOrder();
      Print("Pending order placed: #", ticket, " for master #", master_ticket, " at price ", price);
      AddPendingTicketMapping(pending_map, master_ticket, ticket);
   }
   else
   {
      Print("Failed to place pending order for master #", master_ticket);
   }
}

//+------------------------------------------------------------------+
//| Cancel pending order (MT5)                                        |
//+------------------------------------------------------------------+
void ExecuteCancelPendingOrder(CTrade &trade, PendingTicketMapping &pending_map[], ulong master_ticket)
{
   ulong pending_ticket = GetPendingTicketFromMapping(pending_map, master_ticket);
   if(pending_ticket == 0) return;

   if(trade.OrderDelete(pending_ticket))
   {
      Print("Pending order cancelled: #", pending_ticket, " for master #", master_ticket);
      RemovePendingTicketMapping(pending_map, master_ticket);
   }
   else
   {
      Print("Failed to cancel pending order #", pending_ticket);
   }
}

//+------------------------------------------------------------------+
//| Sync position using limit order (MT5)                             |
//+------------------------------------------------------------------+
void SyncWithLimitOrder(CTrade &trade, PendingTicketMapping &pending_map[],
                        ulong master_ticket, string symbol, string type_str,
                        double lots, double price, double sl, double tp,
                        string source_account, int magic, int expiry_minutes)
{
   ENUM_ORDER_TYPE base_type = GetOrderTypeFromString(type_str);
   if((int)base_type == -1) return;

   ENUM_ORDER_TYPE limit_type;
   double current_price;

   if(base_type == ORDER_TYPE_BUY)
   {
      current_price = SymbolInfoDouble(symbol, SYMBOL_ASK);
      limit_type = (price < current_price) ? ORDER_TYPE_BUY_LIMIT : ORDER_TYPE_BUY_STOP;
   }
   else if(base_type == ORDER_TYPE_SELL)
   {
      current_price = SymbolInfoDouble(symbol, SYMBOL_BID);
      limit_type = (price > current_price) ? ORDER_TYPE_SELL_LIMIT : ORDER_TYPE_SELL_STOP;
   }
   else
   {
      Print("ERROR: Cannot sync pending order type: ", type_str);
      return;
   }

   lots = NormalizeLotSize(lots, symbol);

   datetime expiration = 0;
   if(expiry_minutes > 0)
   {
      expiration = TimeGMT() + expiry_minutes * 60;
   }

   string comment = "S" + IntegerToString(master_ticket);

   trade.SetExpertMagicNumber(magic);

   bool result = trade.OrderOpen(symbol, limit_type, lots, 0, price, sl, tp,
                                  (expiry_minutes > 0) ? ORDER_TIME_SPECIFIED : ORDER_TIME_GTC,
                                  expiration, comment);

   if(result)
   {
      ulong ticket = trade.ResultOrder();
      Print("Sync limit order placed: #", ticket, " for master #", master_ticket,
            " ", EnumToString(limit_type), " @ ", price);
      AddPendingTicketMapping(pending_map, master_ticket, ticket);
   }
   else
   {
      Print("ERROR: Failed to place sync limit order for master #", master_ticket,
            " Error: ", GetLastError());
   }
}

//+------------------------------------------------------------------+
//| Sync position using market order (MT5)                            |
//| Returns: true if executed, false if price deviation exceeded     |
//+------------------------------------------------------------------+
bool SyncWithMarketOrder(CTrade &trade, TicketMapping &order_map[],
                         ulong master_ticket, string symbol, string type_str,
                         double lots, double master_price, double sl, double tp,
                         string source_account, int magic, int slippage_points,
                         double max_pips, int default_slippage)
{
   ENUM_ORDER_TYPE order_type = GetOrderTypeFromString(type_str);
   if((int)order_type == -1) return false;

   double current_price;
   if(order_type == ORDER_TYPE_BUY)
      current_price = SymbolInfoDouble(symbol, SYMBOL_ASK);
   else if(order_type == ORDER_TYPE_SELL)
      current_price = SymbolInfoDouble(symbol, SYMBOL_BID);
   else
      return false;

   double point = SymbolInfoDouble(symbol, SYMBOL_POINT);
   int digits = (int)SymbolInfoInteger(symbol, SYMBOL_DIGITS);
   double pip_size = (digits == 3 || digits == 5) ? point * 10 : point;
   double deviation_pips = MathAbs(current_price - master_price) / pip_size;

   Print("  Price deviation: ", DoubleToString(deviation_pips, 1), " pips (max: ", max_pips, ")");

   if(deviation_pips > max_pips)
   {
      Print("  -> Price deviation ", deviation_pips, " exceeds max ", max_pips, " pips");
      return false;
   }

   lots = NormalizeLotSize(lots, symbol);
   string comment = "S" + IntegerToString(master_ticket);

   trade.SetExpertMagicNumber(magic);
   int effective_slippage = (slippage_points > 0) ? slippage_points : default_slippage;
   trade.SetDeviationInPoints(effective_slippage);

   bool result = false;
   if(order_type == ORDER_TYPE_BUY)
      result = trade.Buy(lots, symbol, 0, sl, tp, comment);
   else if(order_type == ORDER_TYPE_SELL)
      result = trade.Sell(lots, symbol, 0, sl, tp, comment);

   if(result)
   {
      ulong ticket = trade.ResultOrder();
      Print("  -> Market sync executed: #", ticket, " (deviation: ", deviation_pips, " pips)");
      AddTicketMapping(order_map, master_ticket, ticket);
      return true;
   }
   else
   {
      Print("  -> Market sync failed, Error: ", GetLastError());
      return false;
   }
}

// MT5 has OnTradeTransaction for pending order fill detection, no polling needed
void CheckPendingOrderFills(PendingTicketMapping &pending_map[], TicketMapping &order_map[])
{
   // No-op on MT5: OnTradeTransaction handles this
}

#endif // IS_MT5

// =============================================================================
// Trade Execution Functions - MT4 Implementation
// =============================================================================

#ifdef IS_MT4

//+------------------------------------------------------------------+
//| Open order (MT4)                                                  |
//+------------------------------------------------------------------+
void ExecuteOpenTrade(TicketMapping &order_map[], PendingTicketMapping &pending_map[],
                      int master_ticket, string symbol, string type_str,
                      double lots, double price, double sl, double tp, string timestamp,
                      string source_account, int magic, int slippage_points,
                      int max_signal_delay_ms, bool use_pending_for_delayed, int max_retries, int default_slippage)
{
   int slave_ticket = GetSlaveTicketFromMapping(order_map, master_ticket);
   if(slave_ticket > 0)
   {
      Print("Order already copied: master #", master_ticket, " -> slave #", slave_ticket);
      return;
   }

   // Check signal delay
   datetime signal_time = ParseISO8601(timestamp);
   datetime current_time = TimeGMT();
   int delay_ms = (int)((current_time - signal_time) * 1000);

   if(delay_ms > max_signal_delay_ms)
   {
      if(!use_pending_for_delayed)
      {
         Print("Signal too old (", delay_ms, "ms > ", max_signal_delay_ms, "ms). Skipping master #", master_ticket);
         return;
      }
      else
      {
         Print("Signal delayed (", delay_ms, "ms). Using pending order at original price ", price);
         ExecutePendingOrder(pending_map, master_ticket, symbol, type_str, lots, price, sl, tp,
                            source_account, delay_ms, magic, default_slippage);
         return;
      }
   }

   int order_type = GetOrderTypeFromString(type_str);
   if(order_type == -1)
   {
      Print("ERROR: Invalid order type: ", type_str);
      return;
   }

   lots = NormalizeDouble(lots, 2);
   price = NormalizeDouble(price, Digits);
   sl = (sl > 0) ? NormalizeDouble(sl, Digits) : 0;
   tp = (tp > 0) ? NormalizeDouble(tp, Digits) : 0;

   string comment = BuildMarketComment(master_ticket);
   int effective_slippage = (slippage_points > 0) ? slippage_points : default_slippage;

   int ticket = -1;
   for(int attempt = 0; attempt < max_retries; attempt++)
   {
      RefreshRates();

      if(order_type == OP_BUY || order_type == OP_SELL)
      {
         double exec_price = (order_type == OP_BUY) ? Ask : Bid;
         ticket = OrderSend(symbol, order_type, lots, exec_price, effective_slippage, sl, tp,
                           comment, magic, 0, clrGreen);
      }
      else
      {
         ticket = OrderSend(symbol, order_type, lots, price, effective_slippage, sl, tp,
                           comment, magic, 0, clrBlue);
      }

      if(ticket > 0)
      {
         Print("Order opened successfully: slave #", ticket, " from master #", master_ticket,
               " (delay: ", delay_ms, "ms, slippage: ", effective_slippage, " pts)");
         AddTicketMapping(order_map, master_ticket, ticket);
         break;
      }
      else
      {
         Print("ERROR: Failed to open order, attempt ", attempt + 1, "/", max_retries,
               ", Error: ", GetLastError());
         Sleep(1000);
      }
   }
}

//+------------------------------------------------------------------+
//| Close order (MT4)                                                 |
//| close_ratio: 0 or >= 1.0 = full close, 0 < ratio < 1.0 = partial |
//+------------------------------------------------------------------+
void ExecuteCloseTrade(TicketMapping &order_map[],
                       int master_ticket, double close_ratio,
                       int slippage_points, int default_slippage)
{
   int slave_ticket = GetSlaveTicketFromMapping(order_map, master_ticket);
   if(slave_ticket <= 0)
   {
      Print("No slave order found for master #", master_ticket);
      return;
   }

   if(!OrderSelect(slave_ticket, SELECT_BY_TICKET))
   {
      Print("ERROR: Cannot select slave order #", slave_ticket);
      return;
   }

   RefreshRates();
   double close_price = (OrderType() == OP_BUY) ? Bid : Ask;
   int effective_slippage = (slippage_points > 0) ? slippage_points : default_slippage;
   string symbol = OrderSymbol();
   double current_lots = OrderLots();

   // Determine if this is a partial close or full close
   bool is_partial_close = (close_ratio > 0.0 && close_ratio < 1.0);
   double close_lots;

   if(is_partial_close)
   {
      // Partial close: apply close_ratio to current order volume
      close_lots = NormalizeLotSize(current_lots * close_ratio, symbol);

      // Ensure close_lots is valid (at least minimum lot size)
      if(close_lots <= 0.0)
      {
         Print("Partial close lots too small, skipping. Ratio: ", close_ratio, " Current: ", current_lots);
         return;
      }
   }
   else
   {
      // Full close
      close_lots = current_lots;
   }

   bool result = OrderClose(slave_ticket, close_lots, close_price, effective_slippage, clrRed);

   if(result)
   {
      if(is_partial_close)
      {
         Print("Partial close: #", slave_ticket, " closed ", close_lots, " lots (",
               (close_ratio * 100.0), "%), remaining: ", (current_lots - close_lots), " lots");
         // Keep mapping - order still open with remaining lots (MT4 may create new ticket)
         // Note: MT4 may create a new order ticket for remaining lots, mapping may need update
      }
      else
      {
         Print("Order closed successfully: slave #", slave_ticket, " (slippage: ", effective_slippage, " pts)");
         RemoveTicketMapping(order_map, master_ticket);
      }
   }
   else
   {
      Print("ERROR: Failed to close order #", slave_ticket, ", Error: ", GetLastError());
   }
}

//+------------------------------------------------------------------+
//| Modify order (MT4)                                                |
//+------------------------------------------------------------------+
void ExecuteModifyTrade(TicketMapping &order_map[],
                        int master_ticket, double sl, double tp)
{
   int slave_ticket = GetSlaveTicketFromMapping(order_map, master_ticket);
   if(slave_ticket <= 0)
   {
      Print("No slave order found for master #", master_ticket);
      return;
   }

   if(!OrderSelect(slave_ticket, SELECT_BY_TICKET))
   {
      Print("ERROR: Cannot select slave order #", slave_ticket);
      return;
   }

   sl = (sl > 0) ? NormalizeDouble(sl, Digits) : OrderStopLoss();
   tp = (tp > 0) ? NormalizeDouble(tp, Digits) : OrderTakeProfit();

   bool result = OrderModify(slave_ticket, OrderOpenPrice(), sl, tp, 0, clrYellow);

   if(result)
   {
      Print("Order modified successfully: slave #", slave_ticket);
   }
   else
   {
      Print("ERROR: Failed to modify order #", slave_ticket, ", Error: ", GetLastError());
   }
}

//+------------------------------------------------------------------+
//| Place pending order (MT4)                                         |
//+------------------------------------------------------------------+
void ExecutePendingOrder(PendingTicketMapping &pending_map[],
                         int master_ticket, string symbol, string type_str,
                         double lots, double price, double sl, double tp,
                         string source_account, int delay_ms, int magic, int default_slippage)
{
   if(GetPendingTicketFromMapping(pending_map, master_ticket) > 0)
   {
      Print("Pending order already exists for master #", master_ticket);
      return;
   }

   int base_order_type = GetOrderTypeFromString(type_str);
   if(base_order_type == -1)
   {
      Print("ERROR: Invalid order type: ", type_str);
      return;
   }

   RefreshRates();
   int pending_type;

   if(base_order_type == OP_BUY)
   {
      double current_price = Ask;
      pending_type = (price < current_price) ? OP_BUYLIMIT : OP_BUYSTOP;
   }
   else if(base_order_type == OP_SELL)
   {
      double current_price = Bid;
      pending_type = (price > current_price) ? OP_SELLLIMIT : OP_SELLSTOP;
   }
   else
   {
      Print("ERROR: Cannot create pending order for type: ", type_str);
      return;
   }

   lots = NormalizeDouble(lots, 2);
   price = NormalizeDouble(price, Digits);
   sl = (sl > 0) ? NormalizeDouble(sl, Digits) : 0;
   tp = (tp > 0) ? NormalizeDouble(tp, Digits) : 0;

   string comment = BuildPendingComment(master_ticket);

   int ticket = OrderSend(symbol, pending_type, lots, price, default_slippage, sl, tp,
                          comment, magic, 0, clrBlue);

   if(ticket > 0)
   {
      Print("Pending order placed: #", ticket, " for master #", master_ticket, " at price ", price);
      AddPendingTicketMapping(pending_map, master_ticket, ticket);
   }
   else
   {
      Print("Failed to place pending order for master #", master_ticket, " Error: ", GetLastError());
   }
}

//+------------------------------------------------------------------+
//| Cancel pending order (MT4)                                        |
//+------------------------------------------------------------------+
void ExecuteCancelPendingOrder(PendingTicketMapping &pending_map[], int master_ticket)
{
   int pending_ticket = GetPendingTicketFromMapping(pending_map, master_ticket);
   if(pending_ticket <= 0) return;

   if(OrderDelete(pending_ticket))
   {
      Print("Pending order cancelled: #", pending_ticket, " for master #", master_ticket);
      RemovePendingTicketMapping(pending_map, master_ticket);
   }
   else
   {
      Print("Failed to cancel pending order #", pending_ticket, " Error: ", GetLastError());
   }
}

//+------------------------------------------------------------------+
//| Sync position using limit order (MT4)                             |
//+------------------------------------------------------------------+
void SyncWithLimitOrder(PendingTicketMapping &pending_map[],
                        int master_ticket, string symbol, string type_str,
                        double lots, double price, double sl, double tp,
                        string source_account, int magic, int expiry_minutes)
{
   int base_type = GetOrderTypeFromString(type_str);
   if(base_type == -1) return;

   int limit_type;
   double current_price;

   if(base_type == OP_BUY)
   {
      current_price = MarketInfo(symbol, MODE_ASK);
      limit_type = (price < current_price) ? OP_BUYLIMIT : OP_BUYSTOP;
   }
   else if(base_type == OP_SELL)
   {
      current_price = MarketInfo(symbol, MODE_BID);
      limit_type = (price > current_price) ? OP_SELLLIMIT : OP_SELLSTOP;
   }
   else
   {
      Print("ERROR: Cannot sync pending order type: ", type_str);
      return;
   }

   lots = NormalizeLotSize(lots, symbol);

   datetime expiration = 0;
   if(expiry_minutes > 0)
   {
      expiration = TimeGMT() + expiry_minutes * 60;
   }

   string comment = "S" + IntegerToString(master_ticket);

   int ticket = OrderSend(symbol, limit_type, lots, price, 0, sl, tp, comment, magic, expiration);

   if(ticket > 0)
   {
      Print("Sync limit order placed: #", ticket, " for master #", master_ticket,
            " type=", limit_type, " @ ", price);
      AddPendingTicketMapping(pending_map, master_ticket, ticket);
   }
   else
   {
      Print("ERROR: Failed to place sync limit order for master #", master_ticket,
            " Error: ", GetLastError());
   }
}

//+------------------------------------------------------------------+
//| Sync position using market order (MT4)                            |
//| Returns: true if executed, false if price deviation exceeded     |
//+------------------------------------------------------------------+
bool SyncWithMarketOrder(TicketMapping &order_map[],
                         int master_ticket, string symbol, string type_str,
                         double lots, double master_price, double sl, double tp,
                         string source_account, int magic, int slippage_points,
                         double max_pips, int default_slippage)
{
   int order_type = GetOrderTypeFromString(type_str);
   if(order_type == -1 || (order_type != OP_BUY && order_type != OP_SELL))
      return false;

   double current_price;
   if(order_type == OP_BUY)
      current_price = MarketInfo(symbol, MODE_ASK);
   else
      current_price = MarketInfo(symbol, MODE_BID);

   double point = MarketInfo(symbol, MODE_POINT);
   int digits = (int)MarketInfo(symbol, MODE_DIGITS);
   double pip_size = (digits == 3 || digits == 5) ? point * 10 : point;
   double deviation_pips = MathAbs(current_price - master_price) / pip_size;

   Print("  Price deviation: ", DoubleToString(deviation_pips, 1), " pips (max: ", max_pips, ")");

   if(deviation_pips > max_pips)
   {
      Print("  -> Price deviation ", deviation_pips, " exceeds max ", max_pips, " pips");
      return false;
   }

   lots = NormalizeLotSize(lots, symbol);
   string comment = "S" + IntegerToString(master_ticket);
   int effective_slippage = (slippage_points > 0) ? slippage_points : default_slippage;

   int ticket = OrderSend(symbol, order_type, lots, current_price, effective_slippage,
                          sl, tp, comment, magic, 0);

   if(ticket > 0)
   {
      Print("  -> Market sync executed: #", ticket, " (deviation: ", deviation_pips, " pips)");
      AddTicketMapping(order_map, master_ticket, ticket);
      return true;
   }
   else
   {
      Print("  -> Market sync failed, Error: ", GetLastError());
      return false;
   }
}

//+------------------------------------------------------------------+
//| Check pending order fills (MT4 polling)                           |
//| MT4 doesn't have OnTradeTransaction, so we must poll              |
//+------------------------------------------------------------------+
void CheckPendingOrderFills(PendingTicketMapping &pending_map[], TicketMapping &order_map[])
{
   for(int i = ArraySize(pending_map) - 1; i >= 0; i--)
   {
      int master_ticket = pending_map[i].master_ticket;
      int pending_ticket = pending_map[i].pending_ticket;

      if(OrderSelect(pending_ticket, SELECT_BY_TICKET))
      {
         int order_type = OrderType();

         // Check if order is still pending (type >= 2 means pending order)
         if(order_type >= OP_BUYLIMIT)
         {
            continue;
         }

         // Order has been filled (converted to market order)
         RemovePendingTicketMapping(pending_map, master_ticket);
         AddTicketMapping(order_map, master_ticket, pending_ticket);

         Print("[PENDING FILL] Order #", pending_ticket, " filled (master:#", master_ticket, ")");
      }
      else
      {
         // Order not found - check history
         if(OrderSelect(pending_ticket, SELECT_BY_TICKET, MODE_HISTORY))
         {
            int close_time = (int)OrderCloseTime();
            if(close_time > 0)
            {
               Print("Pending order #", pending_ticket, " was cancelled/deleted for master #", master_ticket);
               RemovePendingTicketMapping(pending_map, master_ticket);
            }
         }
         else
         {
            Print("Pending order #", pending_ticket, " not found, removing mapping for master #", master_ticket);
            RemovePendingTicketMapping(pending_map, master_ticket);
         }
      }
   }
}

#endif // IS_MT4

#endif // SANKEY_COPIER_SLAVE_TRADE_MQH
