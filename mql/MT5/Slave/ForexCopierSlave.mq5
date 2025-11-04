//+------------------------------------------------------------------+
//|                                        ForexCopierSlave.mq5      |
//|                        Copyright 2025, Forex Copier Project      |
//|                                                                  |
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, Forex Copier Project"
#property link      ""
#property version   "1.00"

#include <Trade/Trade.mqh>

//--- Import Rust ZeroMQ DLL
#import "forex_copier_zmq.dll"
   int    zmq_context_create();
   void   zmq_context_destroy(int context);
   int    zmq_socket_create(int context, int socket_type);
   void   zmq_socket_destroy(int socket);
   int    zmq_socket_bind(int socket, string address);
   int    zmq_socket_connect(int socket, string address);
   int    zmq_socket_send(int socket, string message);
   int    zmq_socket_receive(int socket, uchar &buffer[], int buffer_size);
   int    zmq_socket_subscribe_all(int socket);
   int    zmq_socket_subscribe(int socket, string topic);
#import

//--- ZeroMQ socket types
#define ZMQ_PULL 7
#define ZMQ_PUSH 8
#define ZMQ_SUB 2

//--- Input parameters
input string   TradeServerAddress = "tcp://localhost:5556";  // Trade signal channel
input string   ConfigServerAddress = "tcp://localhost:5557"; // Configuration channel
input string   AccountID = "SLAVE_001";
input int      Slippage = 3;
input int      MaxRetries = 3;
input bool     AllowNewOrders = true;
input bool     AllowCloseOrders = true;

//--- Global variables
int         g_zmq_context = -1;
int         g_zmq_trade_socket = -1;    // Socket for receiving trade signals
int         g_zmq_config_socket = -1;   // Socket for receiving configuration
CTrade      g_trade;
bool        g_initialized = false;
datetime    g_last_heartbeat = 0;
string      g_current_master = "";      // Currently configured master account
string      g_trade_group_id = "";      // Current trade group subscription

struct OrderMapping {
    ulong master_ticket;
    ulong slave_ticket;
};
OrderMapping g_order_map[];

//+------------------------------------------------------------------+
//| Expert initialization function                                     |
//+------------------------------------------------------------------+
int OnInit()
{
   Print("=== ForexCopier Slave EA (MT5) Starting ===");

   g_zmq_context = zmq_context_create();
   if(g_zmq_context < 0)
   {
      Print("ERROR: Failed to create ZMQ context");
      return INIT_FAILED;
   }

   // Create trade signal socket (SUB to port 5556)
   g_zmq_trade_socket = zmq_socket_create(g_zmq_context, ZMQ_SUB);
   if(g_zmq_trade_socket < 0)
   {
      Print("ERROR: Failed to create ZMQ trade socket");
      zmq_context_destroy(g_zmq_context);
      return INIT_FAILED;
   }

   if(zmq_socket_connect(g_zmq_trade_socket, TradeServerAddress) == 0)
   {
      Print("ERROR: Failed to connect to ", TradeServerAddress);
      zmq_socket_destroy(g_zmq_trade_socket);
      zmq_context_destroy(g_zmq_context);
      return INIT_FAILED;
   }

   Print("Connected to trade channel: ", TradeServerAddress);

   // Create config socket (SUB to port 5557)
   g_zmq_config_socket = zmq_socket_create(g_zmq_context, ZMQ_SUB);
   if(g_zmq_config_socket < 0)
   {
      Print("ERROR: Failed to create ZMQ config socket");
      zmq_socket_destroy(g_zmq_trade_socket);
      zmq_context_destroy(g_zmq_context);
      return INIT_FAILED;
   }

   if(zmq_socket_connect(g_zmq_config_socket, ConfigServerAddress) == 0)
   {
      Print("ERROR: Failed to connect to ", ConfigServerAddress);
      zmq_socket_destroy(g_zmq_config_socket);
      zmq_socket_destroy(g_zmq_trade_socket);
      zmq_context_destroy(g_zmq_context);
      return INIT_FAILED;
   }

   // Subscribe to config messages for this account ID
   if(zmq_socket_subscribe(g_zmq_config_socket, AccountID) == 0)
   {
      Print("ERROR: Failed to subscribe to config topic: ", AccountID);
      zmq_socket_destroy(g_zmq_config_socket);
      zmq_socket_destroy(g_zmq_trade_socket);
      zmq_context_destroy(g_zmq_context);
      return INIT_FAILED;
   }

   Print("Connected to config channel: ", ConfigServerAddress, " and subscribed to: ", AccountID);

   g_trade.SetExpertMagicNumber(0);
   g_trade.SetDeviationInPoints(Slippage);
   g_trade.SetTypeFilling(ORDER_FILLING_IOC);

   ArrayResize(g_order_map, 0);
   g_initialized = true;

   // Send registration message to server
   SendRegisterMessage();

   return INIT_SUCCEEDED;
}

//+------------------------------------------------------------------+
//| Expert deinitialization function                                  |
//+------------------------------------------------------------------+
void OnDeinit(const int reason)
{
   // Send unregister message to server
   SendUnregisterMessage();

   if(g_zmq_config_socket >= 0) zmq_socket_destroy(g_zmq_config_socket);
   if(g_zmq_trade_socket >= 0) zmq_socket_destroy(g_zmq_trade_socket);
   if(g_zmq_context >= 0) zmq_context_destroy(g_zmq_context);
}

//+------------------------------------------------------------------+
//| Expert tick function                                              |
//+------------------------------------------------------------------+
void OnTick()
{
   if(!g_initialized) return;

   // Send heartbeat every 30 seconds
   if(TimeCurrent() - g_last_heartbeat >= 30)
   {
      SendHeartbeat();
      g_last_heartbeat = TimeCurrent();
   }

   // Check for configuration messages
   uchar config_buffer[];
   ArrayResize(config_buffer, 4096);
   int config_bytes = zmq_socket_receive(g_zmq_config_socket, config_buffer, 4096);

   if(config_bytes > 0)
   {
      string config_message = CharArrayToString(config_buffer, 0, config_bytes);

      // Parse config message: topic + space + JSON
      int space_pos = StringFind(config_message, " ");
      if(space_pos > 0)
      {
         string topic = StringSubstr(config_message, 0, space_pos);
         string json = StringSubstr(config_message, space_pos + 1);

         Print("Received config for topic '", topic, "': ", json);
         ProcessConfigMessage(json);
      }
   }

   // Check for trade signal messages
   uchar trade_buffer[];
   ArrayResize(trade_buffer, 4096);
   int trade_bytes = zmq_socket_receive(g_zmq_trade_socket, trade_buffer, 4096);

   if(trade_bytes > 0)
   {
      string trade_message = CharArrayToString(trade_buffer, 0, trade_bytes);

      // PUB/SUB形式: トピック(trade_group_id) + スペース + JSON
      int space_pos = StringFind(trade_message, " ");
      if(space_pos > 0)
      {
         string topic = StringSubstr(trade_message, 0, space_pos);
         string json = StringSubstr(trade_message, space_pos + 1);

         Print("Received trade signal for topic '", topic, "': ", json);
         ProcessTradeSignal(json);
      }
      else
      {
         // 互換性: スペースがない場合はそのまま処理
         Print("Received trade: ", trade_message);
         ProcessTradeSignal(trade_message);
      }
   }
}

//+------------------------------------------------------------------+
//| Process trade signal                                              |
//+------------------------------------------------------------------+
void ProcessTradeSignal(string json)
{
   string action = GetJsonValue(json, "action");
   ulong master_ticket = (ulong)StringToInteger(GetJsonValue(json, "ticket"));
   string symbol = GetJsonValue(json, "symbol");
   string order_type_str = GetJsonValue(json, "order_type");
   double lots = StringToDouble(GetJsonValue(json, "lots"));
   double price = StringToDouble(GetJsonValue(json, "open_price"));
   string sl_str = GetJsonValue(json, "stop_loss");
   string tp_str = GetJsonValue(json, "take_profit");
   double sl = (sl_str != "null") ? StringToDouble(sl_str) : 0;
   double tp = (tp_str != "null") ? StringToDouble(tp_str) : 0;

   if(action == "Open" && AllowNewOrders)
   {
      OpenPosition(master_ticket, symbol, order_type_str, lots, sl, tp);
   }
   else if(action == "Close" && AllowCloseOrders)
   {
      ClosePosition(master_ticket);
   }
   else if(action == "Modify")
   {
      ModifyPosition(master_ticket, sl, tp);
   }
}

//+------------------------------------------------------------------+
//| Open position                                                     |
//+------------------------------------------------------------------+
void OpenPosition(ulong master_ticket, string symbol, string type_str,
                  double lots, double sl, double tp)
{
   if(GetSlaveTicket(master_ticket) > 0)
   {
      Print("Already copied master #", master_ticket);
      return;
   }

   ENUM_ORDER_TYPE order_type = GetOrderType(type_str);
   if(order_type == -1) return;

   lots = NormalizeDouble(lots, 2);

   g_trade.SetExpertMagicNumber(0);
   bool result = false;

   for(int i = 0; i < MaxRetries; i++)
   {
      if(order_type == ORDER_TYPE_BUY)
         result = g_trade.Buy(lots, symbol, 0, sl, tp, "Copy from #" + IntegerToString(master_ticket));
      else if(order_type == ORDER_TYPE_SELL)
         result = g_trade.Sell(lots, symbol, 0, sl, tp, "Copy from #" + IntegerToString(master_ticket));

      if(result)
      {
         ulong ticket = g_trade.ResultOrder();
         Print("Position opened: #", ticket, " from master #", master_ticket);
         AddOrderMapping(master_ticket, ticket);
         break;
      }
      else
      {
         Print("Failed to open position, attempt ", i+1, "/", MaxRetries);
         Sleep(1000);
      }
   }
}

//+------------------------------------------------------------------+
//| Close position                                                    |
//+------------------------------------------------------------------+
void ClosePosition(ulong master_ticket)
{
   ulong slave_ticket = GetSlaveTicket(master_ticket);
   if(slave_ticket == 0)
   {
      Print("No slave position for master #", master_ticket);
      return;
   }

   if(!PositionSelectByTicket(slave_ticket))
   {
      Print("Position #", slave_ticket, " not found");
      RemoveOrderMapping(master_ticket);
      return;
   }

   if(g_trade.PositionClose(slave_ticket))
   {
      Print("Position closed: #", slave_ticket);
      RemoveOrderMapping(master_ticket);
   }
   else
   {
      Print("Failed to close position #", slave_ticket);
   }
}

//+------------------------------------------------------------------+
//| Modify position                                                   |
//+------------------------------------------------------------------+
void ModifyPosition(ulong master_ticket, double sl, double tp)
{
   ulong slave_ticket = GetSlaveTicket(master_ticket);
   if(slave_ticket == 0) return;

   if(!PositionSelectByTicket(slave_ticket)) return;

   if(g_trade.PositionModify(slave_ticket, sl, tp))
   {
      Print("Position modified: #", slave_ticket);
   }
}

//+------------------------------------------------------------------+
//| Get order type from string                                        |
//+------------------------------------------------------------------+
ENUM_ORDER_TYPE GetOrderType(string type_str)
{
   if(type_str == "Buy") return ORDER_TYPE_BUY;
   if(type_str == "Sell") return ORDER_TYPE_SELL;
   return (ENUM_ORDER_TYPE)-1;
}

//+------------------------------------------------------------------+
//| Simple JSON parser                                                |
//+------------------------------------------------------------------+
string GetJsonValue(string json, string key)
{
   string search = "\"" + key + "\":";
   int start = StringFind(json, search);
   if(start == -1) return "";

   start += StringLen(search);

   // Skip whitespace and quotes
   int jsonLen = StringLen(json);
   while(start < jsonLen)
   {
      ushort c = StringGetCharacter(json, start);
      if(c != 32 && c != 34) break;  // 32 = space, 34 = double quote
      start++;
   }

   int end = start;
   bool in_string = false;

   // Find end of value
   while(end < jsonLen)
   {
      ushort c = StringGetCharacter(json, end);
      if(c == 34) in_string = !in_string;  // 34 = double quote
      else if(!in_string && (c == 44 || c == 125)) break;  // 44 = comma, 125 = }
      end++;
   }

   string value = StringSubstr(json, start, end - start);
   StringReplace(value, "\"", "");
   StringTrimLeft(value);
   StringTrimRight(value);
   return value;
}

//+------------------------------------------------------------------+
//| Order mapping helpers                                             |
//+------------------------------------------------------------------+
void AddOrderMapping(ulong master, ulong slave)
{
   int size = ArraySize(g_order_map);
   ArrayResize(g_order_map, size + 1);
   g_order_map[size].master_ticket = master;
   g_order_map[size].slave_ticket = slave;
}

ulong GetSlaveTicket(ulong master)
{
   for(int i = 0; i < ArraySize(g_order_map); i++)
      if(g_order_map[i].master_ticket == master)
         return g_order_map[i].slave_ticket;
   return 0;
}

void RemoveOrderMapping(ulong master)
{
   for(int i = 0; i < ArraySize(g_order_map); i++)
   {
      if(g_order_map[i].master_ticket == master)
      {
         for(int j = i; j < ArraySize(g_order_map) - 1; j++)
            g_order_map[j] = g_order_map[j + 1];
         ArrayResize(g_order_map, ArraySize(g_order_map) - 1);
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
   json += "\"platform\":\"MT5\",";
   json += "\"account_number\":" + IntegerToString(AccountInfoInteger(ACCOUNT_LOGIN)) + ",";
   json += "\"broker\":\"" + AccountInfoString(ACCOUNT_COMPANY) + "\",";
   json += "\"account_name\":\"" + AccountInfoString(ACCOUNT_NAME) + "\",";
   json += "\"server\":\"" + AccountInfoString(ACCOUNT_SERVER) + "\",";
   json += "\"balance\":" + DoubleToString(AccountInfoDouble(ACCOUNT_BALANCE), 2) + ",";
   json += "\"equity\":" + DoubleToString(AccountInfoDouble(ACCOUNT_EQUITY), 2) + ",";
   json += "\"currency\":\"" + AccountInfoString(ACCOUNT_CURRENCY) + "\",";
   json += "\"leverage\":" + IntegerToString(AccountInfoInteger(ACCOUNT_LEVERAGE)) + ",";
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
//| Process configuration message                                     |
//+------------------------------------------------------------------+
void ProcessConfigMessage(string json)
{
   string new_master = GetJsonValue(json, "master_account");
   string new_group = GetJsonValue(json, "trade_group_id");

   if(new_master == "" || new_group == "")
   {
      Print("ERROR: Invalid config message received");
      return;
   }

   // Check if configuration changed
   if(new_master != g_current_master || new_group != g_trade_group_id)
   {
      Print("=== Configuration Update ===");
      Print("Master Account: ", g_current_master, " -> ", new_master);
      Print("Trade Group ID: ", g_trade_group_id, " -> ", new_group);

      // Update configuration
      g_current_master = new_master;
      g_trade_group_id = new_group;

      // Subscribe to new trade group
      if(zmq_socket_subscribe(g_zmq_trade_socket, g_trade_group_id) == 0)
      {
         Print("ERROR: Failed to subscribe to trade group: ", g_trade_group_id);
      }
      else
      {
         Print("Successfully subscribed to trade group: ", g_trade_group_id);
      }

      Print("=== Configuration Updated ===");
   }
   else
   {
      Print("Configuration unchanged - same master and group");
   }
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
   int open_positions = PositionsTotal();

   string json = "{";
   json += "\"message_type\":\"Heartbeat\",";
   json += "\"account_id\":\"" + AccountID + "\",";
   json += "\"balance\":" + DoubleToString(AccountInfoDouble(ACCOUNT_BALANCE), 2) + ",";
   json += "\"equity\":" + DoubleToString(AccountInfoDouble(ACCOUNT_EQUITY), 2) + ",";
   json += "\"open_positions\":" + IntegerToString(open_positions) + ",";
   json += "\"timestamp\":\"" + timestamp + "\"";
   json += "}";

   zmq_socket_send(push_socket, json);

   zmq_socket_destroy(push_socket);
}
