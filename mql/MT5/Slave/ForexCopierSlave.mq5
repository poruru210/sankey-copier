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
   long   msgpack_parse(uchar &data[], int data_len);
   string config_get_string(long handle, string field_name);
   double config_get_double(long handle, string field_name);
   int    config_get_bool(long handle, string field_name);
   int    config_get_int(long handle, string field_name);
   void   config_free(long handle);
#import

//--- ZeroMQ socket types
#define ZMQ_PULL 7
#define ZMQ_PUSH 8
#define ZMQ_SUB 2

//--- Input parameters
input string   TradeServerAddress = "tcp://localhost:5556";  // Trade signal channel
input string   ConfigServerAddress = "tcp://localhost:5557"; // Configuration channel
input int      Slippage = 3;
input int      MaxRetries = 3;
input bool     AllowNewOrders = true;
input bool     AllowCloseOrders = true;

//--- Global variables
string      AccountID;                  // Auto-generated from broker + account number
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

//--- Configuration structures
struct SymbolMapping {
    string source_symbol;
    string target_symbol;
};

struct TradeFilters {
    string allowed_symbols[];
    string blocked_symbols[];
    int    allowed_magic_numbers[];
    int    blocked_magic_numbers[];
};

//--- Extended configuration variables (from ConfigMessage)
bool           g_config_enabled = true;          // Whether copying is enabled
double         g_config_lot_multiplier = 1.0;    // Lot multiplier (default 1.0)
bool           g_config_reverse_trade = false;   // Reverse trades (Buy<->Sell)
SymbolMapping  g_symbol_mappings[];              // Symbol mappings
TradeFilters   g_filters;                        // Trade filters
int            g_config_version = 0;             // Configuration version

//+------------------------------------------------------------------+
//| Expert initialization function                                     |
//+------------------------------------------------------------------+
int OnInit()
{
   Print("=== ForexCopier Slave EA (MT5) Starting ===");

   // Auto-generate AccountID from broker name and account number
   string broker = AccountInfoString(ACCOUNT_COMPANY);
   long account_number = AccountInfoInteger(ACCOUNT_LOGIN);

   // Replace spaces and special characters with underscores
   StringReplace(broker, " ", "_");
   StringReplace(broker, ".", "_");
   StringReplace(broker, "-", "_");

   // Format: broker_accountnumber
   AccountID = broker + "_" + IntegerToString(account_number);
   Print("Auto-generated AccountID: ", AccountID);

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

   // Initialize configuration arrays
   ArrayResize(g_symbol_mappings, 0);
   ArrayResize(g_filters.allowed_symbols, 0);
   ArrayResize(g_filters.blocked_symbols, 0);
   ArrayResize(g_filters.allowed_magic_numbers, 0);
   ArrayResize(g_filters.blocked_magic_numbers, 0);

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

   // Check for configuration messages (MessagePack format)
   uchar config_buffer[];
   ArrayResize(config_buffer, 4096);
   int config_bytes = zmq_socket_receive(g_zmq_config_socket, config_buffer, 4096);

   if(config_bytes > 0)
   {
      // Find the space separator between topic and MessagePack payload
      int space_pos = -1;
      for(int i = 0; i < config_bytes; i++)
      {
         if(config_buffer[i] == 32) // 32 = space
         {
            space_pos = i;
            break;
         }
      }

      if(space_pos > 0)
      {
         // Extract topic
         string topic = CharArrayToString(config_buffer, 0, space_pos);

         // Extract MessagePack payload
         int payload_start = space_pos + 1;
         int payload_len = config_bytes - payload_start;
         uchar msgpack_payload[];
         ArrayResize(msgpack_payload, payload_len);
         ArrayCopy(msgpack_payload, config_buffer, 0, payload_start, payload_len);

         Print("Received MessagePack config for topic '", topic, "' (", payload_len, " bytes)");
         ProcessConfigMessage(msgpack_payload, payload_len);
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
//| Check if trade should be processed based on filters             |
//+------------------------------------------------------------------+
bool ShouldProcessTrade(string symbol, int magic_number)
{
   // Check if copying is enabled
   if(!g_config_enabled)
   {
      Print("Trade filtering: Copying is disabled");
      return false;
   }

   // Check allowed symbols filter
   if(ArraySize(g_filters.allowed_symbols) > 0)
   {
      bool symbol_found = false;
      for(int i = 0; i < ArraySize(g_filters.allowed_symbols); i++)
      {
         if(g_filters.allowed_symbols[i] == symbol)
         {
            symbol_found = true;
            break;
         }
      }

      if(!symbol_found)
      {
         Print("Trade filtering: Symbol ", symbol, " not in allowed list");
         return false;
      }
   }

   // Check blocked symbols filter
   if(ArraySize(g_filters.blocked_symbols) > 0)
   {
      for(int i = 0; i < ArraySize(g_filters.blocked_symbols); i++)
      {
         if(g_filters.blocked_symbols[i] == symbol)
         {
            Print("Trade filtering: Symbol ", symbol, " is blocked");
            return false;
         }
      }
   }

   // Check allowed magic numbers filter
   if(ArraySize(g_filters.allowed_magic_numbers) > 0)
   {
      bool magic_found = false;
      for(int i = 0; i < ArraySize(g_filters.allowed_magic_numbers); i++)
      {
         if(g_filters.allowed_magic_numbers[i] == magic_number)
         {
            magic_found = true;
            break;
         }
      }

      if(!magic_found)
      {
         Print("Trade filtering: Magic number ", magic_number, " not in allowed list");
         return false;
      }
   }

   // Check blocked magic numbers filter
   if(ArraySize(g_filters.blocked_magic_numbers) > 0)
   {
      for(int i = 0; i < ArraySize(g_filters.blocked_magic_numbers); i++)
      {
         if(g_filters.blocked_magic_numbers[i] == magic_number)
         {
            Print("Trade filtering: Magic number ", magic_number, " is blocked");
            return false;
         }
      }
   }

   // All checks passed
   return true;
}

//+------------------------------------------------------------------+
//| Transform symbol using symbol mappings                           |
//+------------------------------------------------------------------+
string TransformSymbol(string source_symbol)
{
   // Check if there's a mapping for this symbol
   for(int i = 0; i < ArraySize(g_symbol_mappings); i++)
   {
      if(g_symbol_mappings[i].source_symbol == source_symbol)
      {
         Print("Symbol transformation: ", source_symbol, " -> ", g_symbol_mappings[i].target_symbol);
         return g_symbol_mappings[i].target_symbol;
      }
   }

   // No mapping found, return original symbol
   return source_symbol;
}

//+------------------------------------------------------------------+
//| Apply lot multiplier to lot size                                |
//+------------------------------------------------------------------+
double TransformLotSize(double source_lots)
{
   double transformed = source_lots * g_config_lot_multiplier;
   transformed = NormalizeDouble(transformed, 2);

   Print("Lot transformation: ", source_lots, " * ", g_config_lot_multiplier, " = ", transformed);

   return transformed;
}

//+------------------------------------------------------------------+
//| Reverse order type if configured                                |
//+------------------------------------------------------------------+
string ReverseOrderType(string order_type)
{
   if(!g_config_reverse_trade)
   {
      return order_type; // No reversal
   }

   // Reverse Buy <-> Sell
   if(order_type == "Buy")
   {
      Print("Order type reversed: Buy -> Sell");
      return "Sell";
   }
   else if(order_type == "Sell")
   {
      Print("Order type reversed: Sell -> Buy");
      return "Buy";
   }

   return order_type;
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

   // Get magic number (defaults to 0 if not present)
   string magic_str = GetJsonValue(json, "magic_number");
   int magic_number = (magic_str != "") ? (int)StringToInteger(magic_str) : 0;

   if(action == "Open" && AllowNewOrders)
   {
      // Apply filtering
      if(!ShouldProcessTrade(symbol, magic_number))
      {
         Print("Trade filtered out: ", symbol, " magic=", magic_number);
         return;
      }

      // Apply transformations
      string transformed_symbol = TransformSymbol(symbol);
      double transformed_lots = TransformLotSize(lots);
      string transformed_order_type = ReverseOrderType(order_type_str);

      // Open position with transformed values
      OpenPosition(master_ticket, transformed_symbol, transformed_order_type, transformed_lots, sl, tp);
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

   // Skip whitespace only (not quotes)
   int jsonLen = StringLen(json);
   while(start < jsonLen)
   {
      ushort c = StringGetCharacter(json, start);
      if(c != 32) break;  // 32 = space
      start++;
   }

   // Check if value starts with quote (string value)
   ushort firstChar = StringGetCharacter(json, start);
   bool isString = (firstChar == 34);  // 34 = double quote

   if(isString)
   {
      // Skip opening quote
      start++;

      // Find closing quote
      int end = start;
      while(end < jsonLen)
      {
         ushort c = StringGetCharacter(json, end);
         if(c == 34)  // Found closing quote
         {
            string value = StringSubstr(json, start, end - start);
            return value;
         }
         end++;
      }
      return "";  // No closing quote found
   }
   else
   {
      // Non-string value: find comma or closing brace
      int end = start;
      while(end < jsonLen)
      {
         ushort c = StringGetCharacter(json, end);
         if(c == 44 || c == 125) break;  // 44 = comma, 125 = }
         end++;
      }

      string value = StringSubstr(json, start, end - start);
      StringTrimLeft(value);
      StringTrimRight(value);
      return value;
   }
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
//| Parse symbol mappings from JSON array                            |
//+------------------------------------------------------------------+
void ParseSymbolMappings(string json)
{
   // Find "symbol_mappings": [ ... ]
   int start_pos = StringFind(json, "\"symbol_mappings\":");
   if(start_pos == -1) return;

   start_pos = StringFind(json, "[", start_pos);
   if(start_pos == -1) return;

   int end_pos = StringFind(json, "]", start_pos);
   if(end_pos == -1) return;

   string array_content = StringSubstr(json, start_pos + 1, end_pos - start_pos - 1);
   StringTrimLeft(array_content);
   StringTrimRight(array_content);

   if(array_content == "") return; // Empty array

   // Split by objects: { ... }, { ... }
   int obj_start = 0;
   while(true)
   {
      obj_start = StringFind(array_content, "{", obj_start);
      if(obj_start == -1) break;

      int obj_end = StringFind(array_content, "}", obj_start);
      if(obj_end == -1) break;

      string obj = StringSubstr(array_content, obj_start, obj_end - obj_start + 1);

      // Parse source_symbol and target_symbol
      string source = GetJsonValue(obj, "source_symbol");
      string target = GetJsonValue(obj, "target_symbol");

      if(source != "" && target != "")
      {
         int idx = ArraySize(g_symbol_mappings);
         ArrayResize(g_symbol_mappings, idx + 1);
         g_symbol_mappings[idx].source_symbol = source;
         g_symbol_mappings[idx].target_symbol = target;
      }

      obj_start = obj_end + 1;
   }
}

//+------------------------------------------------------------------+
//| Parse trade filters from JSON                                    |
//+------------------------------------------------------------------+
void ParseTradeFilters(string json)
{
   // Initialize all filter arrays
   ArrayResize(g_filters.allowed_symbols, 0);
   ArrayResize(g_filters.blocked_symbols, 0);
   ArrayResize(g_filters.allowed_magic_numbers, 0);
   ArrayResize(g_filters.blocked_magic_numbers, 0);

   // Find "filters": { ... }
   int filters_pos = StringFind(json, "\"filters\":");
   if(filters_pos == -1) return;

   int filters_start = StringFind(json, "{", filters_pos);
   if(filters_start == -1) return;

   int filters_end = StringFind(json, "}", filters_start);
   if(filters_end == -1) return;

   string filters_obj = StringSubstr(json, filters_start, filters_end - filters_start + 1);

   // Parse allowed_symbols
   ParseStringArray(filters_obj, "allowed_symbols", g_filters.allowed_symbols);

   // Parse blocked_symbols
   ParseStringArray(filters_obj, "blocked_symbols", g_filters.blocked_symbols);

   // Parse allowed_magic_numbers
   ParseIntArray(filters_obj, "allowed_magic_numbers", g_filters.allowed_magic_numbers);

   // Parse blocked_magic_numbers
   ParseIntArray(filters_obj, "blocked_magic_numbers", g_filters.blocked_magic_numbers);
}

//+------------------------------------------------------------------+
//| Parse JSON string array helper                                   |
//+------------------------------------------------------------------+
void ParseStringArray(string json, string key, string &output[])
{
   ArrayResize(output, 0);

   string search = "\"" + key + "\":";
   int key_pos = StringFind(json, search);
   if(key_pos == -1) return;

   int array_start = StringFind(json, "[", key_pos);
   if(array_start == -1) return;

   int array_end = StringFind(json, "]", array_start);
   if(array_end == -1) return;

   string array_content = StringSubstr(json, array_start + 1, array_end - array_start - 1);
   StringTrimLeft(array_content);
   StringTrimRight(array_content);

   // Check for null or empty
   if(array_content == "" || StringFind(array_content, "null") == 0) return;

   // Split by comma (simple approach - assumes no commas in strings)
   string items[];
   int count = StringSplit(array_content, ',', items);

   for(int i = 0; i < count; i++)
   {
      string item = items[i];
      StringTrimLeft(item);
      StringTrimRight(item);
      StringReplace(item, "\"", "");

      if(item != "")
      {
         int idx = ArraySize(output);
         ArrayResize(output, idx + 1);
         output[idx] = item;
      }
   }
}

//+------------------------------------------------------------------+
//| Parse JSON int array helper                                      |
//+------------------------------------------------------------------+
void ParseIntArray(string json, string key, int &output[])
{
   ArrayResize(output, 0);

   string search = "\"" + key + "\":";
   int key_pos = StringFind(json, search);
   if(key_pos == -1) return;

   int array_start = StringFind(json, "[", key_pos);
   if(array_start == -1) return;

   int array_end = StringFind(json, "]", array_start);
   if(array_end == -1) return;

   string array_content = StringSubstr(json, array_start + 1, array_end - array_start - 1);
   StringTrimLeft(array_content);
   StringTrimRight(array_content);

   // Check for null or empty
   if(array_content == "" || StringFind(array_content, "null") == 0) return;

   // Split by comma
   string items[];
   int count = StringSplit(array_content, ',', items);

   for(int i = 0; i < count; i++)
   {
      string item = items[i];
      StringTrimLeft(item);
      StringTrimRight(item);

      if(item != "")
      {
         int idx = ArraySize(output);
         ArrayResize(output, idx + 1);
         output[idx] = (int)StringToInteger(item);
      }
   }
}

//+------------------------------------------------------------------+
//| Process configuration message                                     |
//+------------------------------------------------------------------+
void ProcessConfigMessage(uchar &msgpack_data[], int data_len)
{
   Print("=== Processing Configuration Message ===");

   // Parse MessagePack once and get a handle to the config structure
   long config_handle = msgpack_parse(msgpack_data, data_len);
   if(config_handle == 0)
   {
      Print("ERROR: Failed to parse MessagePack config");
      return;
   }

   // Extract fields from the parsed config using the handle
   string new_master = config_get_string(config_handle, "master_account");
   string new_group = config_get_string(config_handle, "trade_group_id");

   if(new_master == "" || new_group == "")
   {
      Print("ERROR: Invalid config message received");
      config_free(config_handle);
      return;
   }

   // Extract extended configuration fields
   bool new_enabled = (config_get_bool(config_handle, "enabled") == 1);
   double new_lot_mult = config_get_double(config_handle, "lot_multiplier");
   bool new_reverse = (config_get_bool(config_handle, "reverse_trade") == 1);
   int new_version = config_get_int(config_handle, "config_version");

   // Log configuration values
   Print("Master Account: ", new_master);
   Print("Trade Group ID: ", new_group);
   Print("Enabled: ", new_enabled);
   Print("Lot Multiplier: ", new_lot_mult);
   Print("Reverse Trade: ", new_reverse);
   Print("Config Version: ", new_version);

   // TODO: Parse symbol mappings and filters from MessagePack
   // For now, skip arrays until we implement array support in DLL
   ArrayResize(g_symbol_mappings, 0);
   ArrayResize(g_filters.allowed_symbols, 0);
   ArrayResize(g_filters.blocked_symbols, 0);
   ArrayResize(g_filters.allowed_magic_numbers, 0);
   ArrayResize(g_filters.blocked_magic_numbers, 0);

   // Update global configuration
   g_config_enabled = new_enabled;
   g_config_lot_multiplier = new_lot_mult;
   g_config_reverse_trade = new_reverse;
   g_config_version = new_version;

   // Check if master/group changed
   if(new_master != g_current_master || new_group != g_trade_group_id)
   {
      Print("Master Account: ", g_current_master, " -> ", new_master);
      Print("Trade Group ID: ", g_trade_group_id, " -> ", new_group);

      // Update master and group
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
   }

   // Free the config handle
   config_free(config_handle);

   Print("=== Configuration Updated ===");
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
