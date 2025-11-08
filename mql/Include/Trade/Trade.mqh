//+------------------------------------------------------------------+
//|                                                  Trade/Trade.mqh |
//|                        Copyright 2025, Forex Copier Project      |
//|           Minimal CTrade implementation for MQL5 compilation     |
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, Forex Copier Project"
#property strict

//+------------------------------------------------------------------+
//| CTrade Class - Minimal implementation                            |
//| Wraps native MQL5 trading functions                              |
//+------------------------------------------------------------------+
class CTrade
{
private:
   ulong             m_magic;              // Expert magic number
   ulong             m_deviation;          // Slippage in points
   ENUM_ORDER_TYPE_FILLING m_type_filling; // Order filling type
   ulong             m_result_order;       // Last order ticket
   uint              m_result_retcode;     // Last operation return code

public:
                     CTrade(void);
                    ~CTrade(void) {}

   // Configuration methods
   void              SetExpertMagicNumber(ulong magic) { m_magic = magic; }
   void              SetDeviationInPoints(ulong deviation) { m_deviation = deviation; }
   void              SetTypeFilling(ENUM_ORDER_TYPE_FILLING filling) { m_type_filling = filling; }

   // Result methods
   ulong             ResultOrder(void) const { return m_result_order; }
   uint              ResultRetcode(void) const { return m_result_retcode; }

   // Trading operations
   bool              Buy(double volume, string symbol, double price, double sl, double tp, string comment);
   bool              Sell(double volume, string symbol, double price, double sl, double tp, string comment);
   bool              PositionClose(ulong ticket, ulong deviation = ULONG_MAX);
   bool              PositionModify(ulong ticket, double sl, double tp);
   bool              OrderOpen(string symbol, ENUM_ORDER_TYPE order_type, double volume,
                              double limit_price, double price, double sl, double tp,
                              ENUM_ORDER_TYPE_TIME type_time, datetime expiration, string comment);
   bool              OrderDelete(ulong ticket);

private:
   bool              OrderSend(MqlTradeRequest &request, MqlTradeResult &result);
};

//+------------------------------------------------------------------+
//| Constructor                                                       |
//+------------------------------------------------------------------+
CTrade::CTrade(void)
{
   m_magic = 0;
   m_deviation = 10;
   m_type_filling = ORDER_FILLING_FOK;
   m_result_order = 0;
   m_result_retcode = 0;
}

//+------------------------------------------------------------------+
//| Send trade request                                                |
//+------------------------------------------------------------------+
bool CTrade::OrderSend(MqlTradeRequest &request, MqlTradeResult &result)
{
   ResetLastError();

   if(!::OrderSend(request, result))
   {
      m_result_retcode = result.retcode;
      return false;
   }

   m_result_order = result.order;
   m_result_retcode = result.retcode;

   return result.retcode == TRADE_RETCODE_DONE ||
          result.retcode == TRADE_RETCODE_PLACED ||
          result.retcode == TRADE_RETCODE_DONE_PARTIAL;
}

//+------------------------------------------------------------------+
//| Open Buy position                                                 |
//+------------------------------------------------------------------+
bool CTrade::Buy(double volume, string symbol, double price, double sl, double tp, string comment)
{
   MqlTradeRequest request = {};
   MqlTradeResult result = {};

   request.action = TRADE_ACTION_DEAL;
   request.symbol = symbol;
   request.volume = volume;
   request.type = ORDER_TYPE_BUY;
   request.price = (price == 0) ? SymbolInfoDouble(symbol, SYMBOL_ASK) : price;
   request.sl = sl;
   request.tp = tp;
   request.deviation = m_deviation;
   request.magic = m_magic;
   request.comment = comment;
   request.type_filling = m_type_filling;

   return OrderSend(request, result);
}

//+------------------------------------------------------------------+
//| Open Sell position                                                |
//+------------------------------------------------------------------+
bool CTrade::Sell(double volume, string symbol, double price, double sl, double tp, string comment)
{
   MqlTradeRequest request = {};
   MqlTradeResult result = {};

   request.action = TRADE_ACTION_DEAL;
   request.symbol = symbol;
   request.volume = volume;
   request.type = ORDER_TYPE_SELL;
   request.price = (price == 0) ? SymbolInfoDouble(symbol, SYMBOL_BID) : price;
   request.sl = sl;
   request.tp = tp;
   request.deviation = m_deviation;
   request.magic = m_magic;
   request.comment = comment;
   request.type_filling = m_type_filling;

   return OrderSend(request, result);
}

//+------------------------------------------------------------------+
//| Close position                                                    |
//+------------------------------------------------------------------+
bool CTrade::PositionClose(ulong ticket, ulong deviation = ULONG_MAX)
{
   if(!PositionSelectByTicket(ticket))
      return false;

   string symbol = PositionGetString(POSITION_SYMBOL);
   ENUM_POSITION_TYPE type = (ENUM_POSITION_TYPE)PositionGetInteger(POSITION_TYPE);
   double volume = PositionGetDouble(POSITION_VOLUME);

   MqlTradeRequest request = {};
   MqlTradeResult result = {};

   request.action = TRADE_ACTION_DEAL;
   request.position = ticket;
   request.symbol = symbol;
   request.volume = volume;
   request.type = (type == POSITION_TYPE_BUY) ? ORDER_TYPE_SELL : ORDER_TYPE_BUY;
   request.price = (type == POSITION_TYPE_BUY) ? SymbolInfoDouble(symbol, SYMBOL_BID) : SymbolInfoDouble(symbol, SYMBOL_ASK);
   request.deviation = (deviation == ULONG_MAX) ? m_deviation : deviation;
   request.magic = m_magic;
   request.type_filling = m_type_filling;

   return OrderSend(request, result);
}

//+------------------------------------------------------------------+
//| Modify position                                                   |
//+------------------------------------------------------------------+
bool CTrade::PositionModify(ulong ticket, double sl, double tp)
{
   if(!PositionSelectByTicket(ticket))
      return false;

   string symbol = PositionGetString(POSITION_SYMBOL);

   MqlTradeRequest request = {};
   MqlTradeResult result = {};

   request.action = TRADE_ACTION_SLTP;
   request.position = ticket;
   request.symbol = symbol;
   request.sl = sl;
   request.tp = tp;
   request.magic = m_magic;

   return OrderSend(request, result);
}

//+------------------------------------------------------------------+
//| Open pending order                                                |
//+------------------------------------------------------------------+
bool CTrade::OrderOpen(string symbol, ENUM_ORDER_TYPE order_type, double volume,
                       double limit_price, double price, double sl, double tp,
                       ENUM_ORDER_TYPE_TIME type_time, datetime expiration, string comment)
{
   MqlTradeRequest request = {};
   MqlTradeResult result = {};

   request.action = TRADE_ACTION_PENDING;
   request.symbol = symbol;
   request.volume = volume;
   request.type = order_type;
   request.price = price;
   request.stoplimit = limit_price;
   request.sl = sl;
   request.tp = tp;
   request.type_time = type_time;
   request.expiration = expiration;
   request.deviation = m_deviation;
   request.magic = m_magic;
   request.comment = comment;
   request.type_filling = m_type_filling;

   return OrderSend(request, result);
}

//+------------------------------------------------------------------+
//| Delete pending order                                              |
//+------------------------------------------------------------------+
bool CTrade::OrderDelete(ulong ticket)
{
   MqlTradeRequest request = {};
   MqlTradeResult result = {};

   request.action = TRADE_ACTION_REMOVE;
   request.order = ticket;

   return OrderSend(request, result);
}
