//+------------------------------------------------------------------+
//|                                        ForexCopierSlave.mq4      |
//|                        Copyright 2025, Forex Copier Project      |
//|                                                                  |
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, Forex Copier Project"
#property link      ""
#property version   "1.00"
#property strict

//--- Import Rust ZeroMQ DLL
#import "forex_copier_zmq.dll"
   int    zmq_context_create();
   void   zmq_context_destroy(int context);
   int    zmq_socket_create(int context, int socket_type);
   void   zmq_socket_destroy(int socket);
   int    zmq_socket_bind(int socket, string address);
   int    zmq_socket_send(int socket, string message);
   int    zmq_socket_receive(int socket, uchar &buffer[], int buffer_size);
   int    zmq_socket_subscribe_all(int socket);
   int    zmq_socket_connect(int socket, string address);
#import

//--- ZeroMQ socket types
#define ZMQ_PULL 7
#define ZMQ_PUSH 8

//--- Input parameters
input string   ServerAddress = "tcp://localhost:5556";  // Server ZMQ address for slaves
input string   AccountID = "SLAVE_001";                 // Slave account identifier
input int      Slippage = 3;                            // Maximum slippage in points
input int      MaxRetries = 3;                          // Maximum order retries
input bool     AllowNewOrders = true;                   // Allow opening new orders
input bool     AllowCloseOrders = true;                 // Allow closing orders

//--- Global variables
int         g_zmq_context = -1;
int         g_zmq_socket = -1;
bool        g_initialized = false;
int         g_order_map[][2]; // [master_ticket][slave_ticket]
datetime    g_last_heartbeat = 0;

//+------------------------------------------------------------------+
//| Expert initialization function                                     |
//+------------------------------------------------------------------+
int OnInit()
{
   Print("=== ForexCopier Slave EA (MT4) Starting ===");
   Print("Server Address: ", ServerAddress);
   Print("Account ID: ", AccountID);

   g_zmq_context = zmq_context_create();
   if(g_zmq_context < 0)
   {
      Print("ERROR: Failed to create ZMQ context");
      return INIT_FAILED;
   }

   g_zmq_socket = zmq_socket_create(g_zmq_context, ZMQ_PULL);
   if(g_zmq_socket < 0)
   {
      Print("ERROR: Failed to create ZMQ socket");
      zmq_context_destroy(g_zmq_context);
      return INIT_FAILED;
   }

   if(zmq_socket_bind(g_zmq_socket, ServerAddress) == 0)
   {
      Print("ERROR: Failed to bind to ", ServerAddress);
      zmq_socket_destroy(g_zmq_socket);
      zmq_context_destroy(g_zmq_context);
      return INIT_FAILED;
   }

   Print("Listening for signals on ", ServerAddress);

   ArrayResize(g_order_map, 0);

   g_initialized = true;

   // Send registration message to server
   SendRegisterMessage();

   Print("=== ForexCopier Slave EA Initialized ===");

   return INIT_SUCCEEDED;
}

//+------------------------------------------------------------------+
//| Expert deinitialization function                                  |
//+------------------------------------------------------------------+
void OnDeinit(const int reason)
{
   Print("=== ForexCopier Slave EA (MT4) Stopping ===");

   // Send unregister message to server
   SendUnregisterMessage();

   if(g_zmq_socket >= 0) zmq_socket_destroy(g_zmq_socket);
   if(g_zmq_context >= 0) zmq_context_destroy(g_zmq_context);

   Print("=== ForexCopier Slave EA (MT4) Stopped ===");
}

//+------------------------------------------------------------------+
//| Expert tick function                                              |
//+------------------------------------------------------------------+
void OnTick()
{
   if(!g_initialized)
      return;

   // Send heartbeat every 30 seconds
   if(TimeCurrent() - g_last_heartbeat >= 30)
   {
      SendHeartbeat();
      g_last_heartbeat = TimeCurrent();
   }

   // Try to receive message (non-blocking)
   uchar buffer[];
   ArrayResize(buffer, 4096);
   int bytes = zmq_socket_receive(g_zmq_socket, buffer, 4096);

   if(bytes > 0)
   {
      string message = CharArrayToString(buffer, 0, bytes);
      Print("Received message: ", message);
      ProcessTradeSignal(message);
   }
}

//+------------------------------------------------------------------+
//| Process incoming trade signal                                     |
//+------------------------------------------------------------------+
void ProcessTradeSignal(string json)
{
   // Parse JSON (simplified - in production use proper JSON parser)
   string action = GetJsonValue(json, "action");
   int master_ticket = (int)StringToInteger(GetJsonValue(json, "ticket"));
   string symbol = GetJsonValue(json, "symbol");
   string order_type_str = GetJsonValue(json, "order_type");
   double lots = StringToDouble(GetJsonValue(json, "lots"));
   double open_price = StringToDouble(GetJsonValue(json, "open_price"));
   string sl_str = GetJsonValue(json, "stop_loss");
   string tp_str = GetJsonValue(json, "take_profit");
   double stop_loss = (sl_str != "null") ? StringToDouble(sl_str) : 0;
   double take_profit = (tp_str != "null") ? StringToDouble(tp_str) : 0;
   int magic = (int)StringToInteger(GetJsonValue(json, "magic_number"));

   Print("Processing ", action, " for master ticket #", master_ticket);

   if(action == "Open")
   {
      if(AllowNewOrders)
         OpenOrder(master_ticket, symbol, order_type_str, lots, open_price, stop_loss, take_profit, magic);
   }
   else if(action == "Close")
   {
      if(AllowCloseOrders)
         CloseOrder(master_ticket);
   }
   else if(action == "Modify")
   {
      ModifyOrder(master_ticket, stop_loss, take_profit);
   }
}

//+------------------------------------------------------------------+
//| Open new order                                                    |
//+------------------------------------------------------------------+
void OpenOrder(int master_ticket, string symbol, string order_type_str, double lots,
               double price, double sl, double tp, int magic)
{
   // Check if already copied
   int slave_ticket = GetSlaveTicket(master_ticket);
   if(slave_ticket > 0)
   {
      Print("Order already copied: master #", master_ticket, " -> slave #", slave_ticket);
      return;
   }

   int order_type = GetOrderType(order_type_str);
   if(order_type == -1)
   {
      Print("ERROR: Invalid order type: ", order_type_str);
      return;
   }

   // Normalize values
   lots = NormalizeDouble(lots, 2);
   price = NormalizeDouble(price, Digits);
   sl = (sl > 0) ? NormalizeDouble(sl, Digits) : 0;
   tp = (tp > 0) ? NormalizeDouble(tp, Digits) : 0;

   // Execute order
   int ticket = -1;
   for(int attempt = 0; attempt < MaxRetries; attempt++)
   {
      RefreshRates();

      if(order_type == OP_BUY || order_type == OP_SELL)
      {
         double exec_price = (order_type == OP_BUY) ? Ask : Bid;
         ticket = OrderSend(symbol, order_type, lots, exec_price, Slippage, sl, tp,
                           "Copied from #" + IntegerToString(master_ticket), magic, 0, clrGreen);
      }
      else
      {
         ticket = OrderSend(symbol, order_type, lots, price, Slippage, sl, tp,
                           "Copied from #" + IntegerToString(master_ticket), magic, 0, clrBlue);
      }

      if(ticket > 0)
      {
         Print("Order opened successfully: slave #", ticket, " from master #", master_ticket);
         AddOrderMapping(master_ticket, ticket);
         break;
      }
      else
      {
         Print("ERROR: Failed to open order, attempt ", attempt + 1, "/", MaxRetries,
               ", Error: ", GetLastError());
         Sleep(1000);
      }
   }
}

//+------------------------------------------------------------------+
//| Close order                                                       |
//+------------------------------------------------------------------+
void CloseOrder(int master_ticket)
{
   int slave_ticket = GetSlaveTicket(master_ticket);
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

   bool result = OrderClose(slave_ticket, OrderLots(), close_price, Slippage, clrRed);

   if(result)
   {
      Print("Order closed successfully: slave #", slave_ticket);
      RemoveOrderMapping(master_ticket);
   }
   else
   {
      Print("ERROR: Failed to close order #", slave_ticket, ", Error: ", GetLastError());
   }
}

//+------------------------------------------------------------------+
//| Modify order                                                      |
//+------------------------------------------------------------------+
void ModifyOrder(int master_ticket, double sl, double tp)
{
   int slave_ticket = GetSlaveTicket(master_ticket);
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
//| Get order type from string                                        |
//+------------------------------------------------------------------+
int GetOrderType(string type_str)
{
   if(type_str == "Buy")       return OP_BUY;
   if(type_str == "Sell")      return OP_SELL;
   if(type_str == "BuyLimit")  return OP_BUYLIMIT;
   if(type_str == "SellLimit") return OP_SELLLIMIT;
   if(type_str == "BuyStop")   return OP_BUYSTOP;
   if(type_str == "SellStop")  return OP_SELLSTOP;
   return -1;
}

//+------------------------------------------------------------------+
//| Simple JSON value extractor                                       |
//+------------------------------------------------------------------+
string GetJsonValue(string json, string key)
{
   string search = "\"" + key + "\":";
   int start = StringFind(json, search);
   if(start == -1)
      return "";

   start += StringLen(search);

   // Skip whitespace and quotes
   while(start < StringLen(json) && (StringGetCharacter(json, start) == ' ' ||
                                     StringGetCharacter(json, start) == '"'))
      start++;

   int end = start;
   bool in_string = false;

   // Find end of value
   while(end < StringLen(json))
   {
      ushort ch = StringGetCharacter(json, end);
      if(ch == '"')
         in_string = !in_string;
      else if(!in_string && (ch == ',' || ch == '}'))
         break;
      end++;
   }

   string value = StringSubstr(json, start, end - start);
   StringReplace(value, "\"", "");
   StringTrimLeft(value);
   StringTrimRight(value);

   return value;
}

//+------------------------------------------------------------------+
//| Order mapping functions                                           |
//+------------------------------------------------------------------+
void AddOrderMapping(int master_ticket, int slave_ticket)
{
   int size = ArrayRange(g_order_map, 0);
   ArrayResize(g_order_map, size + 1);
   g_order_map[size][0] = master_ticket;
   g_order_map[size][1] = slave_ticket;
}

int GetSlaveTicket(int master_ticket)
{
   for(int i = 0; i < ArrayRange(g_order_map, 0); i++)
   {
      if(g_order_map[i][0] == master_ticket)
         return g_order_map[i][1];
   }
   return -1;
}

void RemoveOrderMapping(int master_ticket)
{
   for(int i = 0; i < ArrayRange(g_order_map, 0); i++)
   {
      if(g_order_map[i][0] == master_ticket)
      {
         // Shift array
         for(int j = i; j < ArrayRange(g_order_map, 0) - 1; j++)
         {
            g_order_map[j][0] = g_order_map[j + 1][0];
            g_order_map[j][1] = g_order_map[j + 1][1];
         }
         ArrayResize(g_order_map, ArrayRange(g_order_map, 0) - 1);
         break;
      }
   }
}

//+------------------------------------------------------------------+
//| Send registration message to server                              |
//+------------------------------------------------------------------+
void SendRegisterMessage()
{
   // Create temporary PUSH socket to send registration
   int push_socket = zmq_socket_create(g_zmq_context, ZMQ_PUSH);
   if(push_socket < 0)
   {
      Print("ERROR: Failed to create registration socket");
      return;
   }

   if(zmq_socket_connect(push_socket, "tcp://localhost:5555") == 0)
   {
      Print("ERROR: Failed to connect to registration server");
      zmq_socket_destroy(push_socket);
      return;
   }

   // Get current timestamp in ISO 8601 format
   string timestamp = TimeToString(TimeCurrent(), TIME_DATE | TIME_SECONDS);
   StringReplace(timestamp, ".", "-");
   StringReplace(timestamp, " ", "T");
   timestamp += "Z";

   // Build JSON message
   string json = "{";
   json += "\"message_type\":\"Register\",";
   json += "\"account_id\":\"" + AccountID + "\",";
   json += "\"ea_type\":\"Slave\",";
   json += "\"platform\":\"MT4\",";
   json += "\"account_number\":" + IntegerToString(AccountNumber()) + ",";
   json += "\"broker\":\"" + AccountCompany() + "\",";
   json += "\"account_name\":\"" + AccountName() + "\",";
   json += "\"server\":\"" + AccountServer() + "\",";
   json += "\"balance\":" + DoubleToString(AccountBalance(), 2) + ",";
   json += "\"equity\":" + DoubleToString(AccountEquity(), 2) + ",";
   json += "\"currency\":\"" + AccountCurrency() + "\",";
   json += "\"leverage\":" + IntegerToString(AccountLeverage()) + ",";
   json += "\"timestamp\":\"" + timestamp + "\"";
   json += "}";

   zmq_socket_send(push_socket, json);
   Print("Registration message sent to server");

   zmq_socket_destroy(push_socket);
}

//+------------------------------------------------------------------+
//| Send unregister message to server                                |
//+------------------------------------------------------------------+
void SendUnregisterMessage()
{
   int push_socket = zmq_socket_create(g_zmq_context, ZMQ_PUSH);
   if(push_socket < 0)
   {
      Print("ERROR: Failed to create unregister socket");
      return;
   }

   if(zmq_socket_connect(push_socket, "tcp://localhost:5555") == 0)
   {
      Print("ERROR: Failed to connect to registration server");
      zmq_socket_destroy(push_socket);
      return;
   }

   // Get current timestamp
   string timestamp = TimeToString(TimeCurrent(), TIME_DATE | TIME_SECONDS);
   StringReplace(timestamp, ".", "-");
   StringReplace(timestamp, " ", "T");
   timestamp += "Z";

   string json = "{";
   json += "\"message_type\":\"Unregister\",";
   json += "\"account_id\":\"" + AccountID + "\",";
   json += "\"timestamp\":\"" + timestamp + "\"";
   json += "}";

   zmq_socket_send(push_socket, json);
   Print("Unregister message sent to server");

   zmq_socket_destroy(push_socket);
}

//+------------------------------------------------------------------+
//| Send heartbeat message to server                                 |
//+------------------------------------------------------------------+
void SendHeartbeat()
{
   int push_socket = zmq_socket_create(g_zmq_context, ZMQ_PUSH);
   if(push_socket < 0)
   {
      Print("ERROR: Failed to create heartbeat socket");
      return;
   }

   if(zmq_socket_connect(push_socket, "tcp://localhost:5555") == 0)
   {
      Print("ERROR: Failed to connect to heartbeat server");
      zmq_socket_destroy(push_socket);
      return;
   }

   // Get current timestamp
   string timestamp = TimeToString(TimeCurrent(), TIME_DATE | TIME_SECONDS);
   StringReplace(timestamp, ".", "-");
   StringReplace(timestamp, " ", "T");
   timestamp += "Z";

   // Count open positions
   int open_positions = 0;
   for(int i = 0; i < OrdersTotal(); i++)
   {
      if(OrderSelect(i, SELECT_BY_POS) && OrderSymbol() == Symbol())
         open_positions++;
   }

   string json = "{";
   json += "\"message_type\":\"Heartbeat\",";
   json += "\"account_id\":\"" + AccountID + "\",";
   json += "\"balance\":" + DoubleToString(AccountBalance(), 2) + ",";
   json += "\"equity\":" + DoubleToString(AccountEquity(), 2) + ",";
   json += "\"open_positions\":" + IntegerToString(open_positions) + ",";
   json += "\"timestamp\":\"" + timestamp + "\"";
   json += "}";

   zmq_socket_send(push_socket, json);

   zmq_socket_destroy(push_socket);
}
