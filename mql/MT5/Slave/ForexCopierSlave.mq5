//+------------------------------------------------------------------+
//|                                        ForexCopierSlave.mq5      |
//|                        Copyright 2025, Forex Copier Project      |
//|                                                                  |
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, Forex Copier Project"
#property link      ""
#property version   "1.00"

#include <Trade/Trade.mqh>

//--- Include common headers
#include <ForexCopierCommon.mqh>
#include <ForexCopierJson.mqh>
#include <ForexCopierMessages.mqh>
#include <ForexCopierTrade.mqh>

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
   AccountID = GenerateAccountID();
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
   SendRegistrationMessage(g_zmq_context, "tcp://localhost:5555", AccountID, "Slave", "MT5");

   return INIT_SUCCEEDED;
}

//+------------------------------------------------------------------+
//| Expert deinitialization function                                  |
//+------------------------------------------------------------------+
void OnDeinit(const int reason)
{
   // Send unregister message to server
   SendUnregistrationMessage(g_zmq_context, "tcp://localhost:5555", AccountID);

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
      SendHeartbeatMessage(g_zmq_context, "tcp://localhost:5555", AccountID);
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
         ProcessConfigMessage(msgpack_payload, payload_len, g_current_master, g_trade_group_id,
                             g_config_enabled, g_config_lot_multiplier, g_config_reverse_trade,
                             g_config_version, g_symbol_mappings, g_filters, g_zmq_trade_socket);
      }
   }

   // Check for trade signal messages (MessagePack format)
   uchar trade_buffer[];
   ArrayResize(trade_buffer, 4096);
   int trade_bytes = zmq_socket_receive(g_zmq_trade_socket, trade_buffer, 4096);

   if(trade_bytes > 0)
   {
      // PUB/SUB format: topic(trade_group_id) + space + MessagePack payload
      int space_pos = -1;
      for(int i = 0; i < trade_bytes; i++)
      {
         if(trade_buffer[i] == 32) // 32 = space
         {
            space_pos = i;
            break;
         }
      }

      if(space_pos > 0)
      {
         // Extract topic
         string topic = CharArrayToString(trade_buffer, 0, space_pos);

         // Extract MessagePack payload
         int payload_start = space_pos + 1;
         int payload_len = trade_bytes - payload_start;
         uchar msgpack_payload[];
         ArrayResize(msgpack_payload, payload_len);
         ArrayCopy(msgpack_payload, trade_buffer, 0, payload_start, payload_len);

         Print("Received MessagePack trade signal for topic '", topic, "' (", payload_len, " bytes)");
         ProcessTradeSignal(msgpack_payload, payload_len);
      }
   }
}

//+------------------------------------------------------------------+
//| Process trade signal                                              |
//+------------------------------------------------------------------+
void ProcessTradeSignal(uchar &data[], int data_len)
{
   // Parse MessagePack trade signal
   HANDLE_TYPE handle = parse_trade_signal(data, data_len);
   if(handle == 0 || handle == -1)
   {
      Print("ERROR: Failed to parse MessagePack trade signal");
      return;
   }

   // Extract fields from MessagePack
   string action = trade_signal_get_string(handle, "action");
   long ticket_long = trade_signal_get_int(handle, "ticket");
   ulong master_ticket = (ulong)ticket_long;
   string symbol = trade_signal_get_string(handle, "symbol");
   string order_type_str = trade_signal_get_string(handle, "order_type");
   double lots = trade_signal_get_double(handle, "lots");
   double price = trade_signal_get_double(handle, "open_price");
   double sl = trade_signal_get_double(handle, "stop_loss");
   double tp = trade_signal_get_double(handle, "take_profit");
   long magic_long = trade_signal_get_int(handle, "magic_number");
   int magic_number = (int)magic_long;

   if(action == "Open" && AllowNewOrders)
   {
      // Apply filtering
      if(!ShouldProcessTrade(symbol, magic_number, g_config_enabled, g_filters))
      {
         Print("Trade filtered out: ", symbol, " magic=", magic_number);
         trade_signal_free(handle);
         return;
      }

      // Apply transformations
      string transformed_symbol = TransformSymbol(symbol, g_symbol_mappings);
      double transformed_lots = TransformLotSize(lots, g_config_lot_multiplier);
      string transformed_order_type = ReverseOrderType(order_type_str, g_config_reverse_trade);

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

   // Free the handle
   trade_signal_free(handle);
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

   ENUM_ORDER_TYPE order_type = GetOrderTypeEnum(type_str);
   if((int)order_type == -1) return;

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

