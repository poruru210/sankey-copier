//+------------------------------------------------------------------+
//|                                        SankeyCopierSlave.mq4      |
//|                        Copyright 2025, SANKEY Copier Project      |
//|                                                                  |
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, SANKEY Copier Project"
#property link      ""
#property version   "2.00"
#property strict

//--- Include common headers
#include <SankeyCopier/SankeyCopierCommon.mqh>
#include <SankeyCopier/SankeyCopierMessages.mqh>
#include <SankeyCopier/SankeyCopierTrade.mqh>

//--- Input parameters
input string   TradeServerAddress = "tcp://localhost:5556";  // Trade signal channel
input string   ConfigServerAddress = "tcp://localhost:5557"; // Configuration channel
input int      Slippage = 3;                                 // Maximum slippage in points
input int      MaxRetries = 3;                               // Maximum order retries
input bool     AllowNewOrders = true;                        // Allow opening new orders
input bool     AllowCloseOrders = true;                      // Allow closing orders
input int      MaxSignalDelayMs = 5000;                      // Maximum allowed signal delay (milliseconds)
input bool     UsePendingOrderForDelayed = false;            // Use pending order for delayed signals

//--- Global variables
string      AccountID;                  // Auto-generated from broker + account number
HANDLE_TYPE g_zmq_context = -1;
HANDLE_TYPE g_zmq_trade_socket = -1;    // Socket for receiving trade signals
HANDLE_TYPE g_zmq_config_socket = -1;   // Socket for receiving configuration
bool        g_initialized = false;
datetime    g_last_heartbeat = 0;
string      g_current_master = "";      // Currently configured master account
string      g_trade_group_id = "";      // Current trade group subscription

struct OrderMapping {
    int master_ticket;
    int slave_ticket;
};
OrderMapping g_order_map[];

struct PendingOrderMapping {
    int master_ticket;
    int pending_ticket;
};
PendingOrderMapping g_pending_order_map[];

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
   Print("=== SankeyCopier Slave EA (MT4) Starting ===");

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

   ArrayResize(g_order_map, 0);
   ArrayResize(g_pending_order_map, 0);

   // Initialize configuration arrays
   ArrayResize(g_symbol_mappings, 0);
   ArrayResize(g_filters.allowed_symbols, 0);
   ArrayResize(g_filters.blocked_symbols, 0);
   ArrayResize(g_filters.allowed_magic_numbers, 0);
   ArrayResize(g_filters.blocked_magic_numbers, 0);

   g_initialized = true;

   // Send registration message to server
   SendRegistrationMessage(g_zmq_context, "tcp://localhost:5555", AccountID, "Slave", "MT4");

   // Set up timer for heartbeat and config messages (1 second interval)
   EventSetTimer(1);

   Print("=== SankeyCopier Slave EA Initialized ===");

   return INIT_SUCCEEDED;
}

//+------------------------------------------------------------------+
//| Expert deinitialization function                                  |
//+------------------------------------------------------------------+
void OnDeinit(const int reason)
{
   Print("=== SankeyCopier Slave EA (MT4) Stopping ===");

   // Send unregister message to server
   SendUnregistrationMessage(g_zmq_context, "tcp://localhost:5555", AccountID);

   // Kill timer
   EventKillTimer();

   if(g_zmq_config_socket >= 0) zmq_socket_destroy(g_zmq_config_socket);
   if(g_zmq_trade_socket >= 0) zmq_socket_destroy(g_zmq_trade_socket);
   if(g_zmq_context >= 0) zmq_context_destroy(g_zmq_context);

   Print("=== SankeyCopier Slave EA (MT4) Stopped ===");
}

//+------------------------------------------------------------------+
//| Timer function (called every 1 second)                            |
//+------------------------------------------------------------------+
void OnTimer()
{
   static int timer_call_count = 0;
   timer_call_count++;
   Print("[DEBUG] OnTimer() called (count=", timer_call_count, ")");

   if(!g_initialized)
   {
      Print("[DEBUG] Not initialized, returning");
      return;
   }

   // Send heartbeat every HEARTBEAT_INTERVAL_SECONDS
   datetime now = TimeLocal();
   Print("[DEBUG] Time check: now=", now, ", last_heartbeat=", g_last_heartbeat, ", elapsed=", (int)(now - g_last_heartbeat), " seconds");

   if(now - g_last_heartbeat >= HEARTBEAT_INTERVAL_SECONDS)
   {
      Print("[DEBUG] Sending heartbeat...");
      SendHeartbeatMessage(g_zmq_context, "tcp://localhost:5555", AccountID);
      g_last_heartbeat = TimeLocal();
      Print("[DEBUG] Heartbeat sent, updated last_heartbeat=", g_last_heartbeat);
   }

   // Check for configuration messages (MessagePack format)
   Print("[DEBUG] Checking for config messages...");
   uchar config_buffer[];
   ArrayResize(config_buffer, MESSAGE_BUFFER_SIZE);
   Print("[DEBUG] Buffer resized, calling zmq_socket_receive...");
   int config_bytes = zmq_socket_receive(g_zmq_config_socket, config_buffer, MESSAGE_BUFFER_SIZE);
   Print("[DEBUG] zmq_socket_receive returned: ", config_bytes);

   if(config_bytes > 0)
   {
      Print("[DEBUG] Processing config message (", config_bytes, " bytes)");
      // Find the space separator between topic and MessagePack payload
      int space_pos = -1;
      for(int i = 0; i < config_bytes; i++)
      {
         if(config_buffer[i] == SPACE_CHAR)
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

   Print("[DEBUG] OnTimer() completed");
}

//+------------------------------------------------------------------+
//| Expert tick function                                              |
//+------------------------------------------------------------------+
void OnTick()
{
   if(!g_initialized)
      return;

   // Check for trade signal messages (MessagePack format)
   uchar trade_buffer[];
   ArrayResize(trade_buffer, MESSAGE_BUFFER_SIZE);
   int trade_bytes = zmq_socket_receive(g_zmq_trade_socket, trade_buffer, MESSAGE_BUFFER_SIZE);

   if(trade_bytes > 0)
   {
      // PUB/SUB format: topic(trade_group_id) + space + MessagePack payload
      int space_pos = -1;
      for(int i = 0; i < trade_bytes; i++)
      {
         if(trade_buffer[i] == SPACE_CHAR)
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
//| Process incoming trade signal                                     |
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
   int master_ticket = (int)ticket_long;
   string symbol = trade_signal_get_string(handle, "symbol");
   string order_type_str = trade_signal_get_string(handle, "order_type");
   double lots = trade_signal_get_double(handle, "lots");
   double open_price = trade_signal_get_double(handle, "open_price");
   double stop_loss = trade_signal_get_double(handle, "stop_loss");
   double take_profit = trade_signal_get_double(handle, "take_profit");
   long magic_long = trade_signal_get_int(handle, "magic_number");
   int magic = (int)magic_long;
   string timestamp = trade_signal_get_string(handle, "timestamp");
   string source_account = trade_signal_get_string(handle, "source_account");

   Print("Processing ", action, " for master ticket #", master_ticket);

   if(action == "Open")
   {
      if(AllowNewOrders)
      {
         // Apply filtering
         if(!ShouldProcessTrade(symbol, magic, g_config_enabled, g_filters))
         {
            Print("Trade filtered out: ", symbol, " magic=", magic);
            trade_signal_free(handle);
            return;
         }

         // Apply transformations
         string transformed_symbol = TransformSymbol(symbol, g_symbol_mappings);
         double transformed_lots = TransformLotSize(lots, g_config_lot_multiplier);
         string transformed_order_type = ReverseOrderType(order_type_str, g_config_reverse_trade);

         // Open order with transformed values
         OpenOrder(master_ticket, transformed_symbol, transformed_order_type, transformed_lots, open_price, stop_loss, take_profit, magic, timestamp, source_account);
      }
   }
   else if(action == "Close")
   {
      if(AllowCloseOrders)
      {
         CloseOrder(master_ticket);
         CancelPendingOrder(master_ticket);  // Also cancel any pending orders
      }
   }
   else if(action == "Modify")
   {
      ModifyOrder(master_ticket, stop_loss, take_profit);
   }

   // Free the handle
   trade_signal_free(handle);
}

//+------------------------------------------------------------------+
//| Open new order                                                    |
//+------------------------------------------------------------------+
void OpenOrder(int master_ticket, string symbol, string order_type_str, double lots,
               double price, double sl, double tp, int magic, string timestamp, string source_account)
{
   // Check if already copied
   int slave_ticket = GetSlaveTicket(master_ticket);
   if(slave_ticket > 0)
   {
      Print("Order already copied: master #", master_ticket, " -> slave #", slave_ticket);
      return;
   }

   // Check signal delay
   datetime signal_time = ParseISO8601(timestamp);
   datetime current_time = TimeCurrent();
   int delay_ms = (int)((current_time - signal_time) * 1000);

   if(delay_ms > MaxSignalDelayMs)
   {
      if(!UsePendingOrderForDelayed)
      {
         Print("Signal too old (", delay_ms, "ms > ", MaxSignalDelayMs, "ms). Skipping master #", master_ticket);
         return;
      }
      else
      {
         Print("Signal delayed (", delay_ms, "ms). Using pending order at original price ", price);
         PlacePendingOrder(master_ticket, symbol, order_type_str, lots, price, sl, tp, magic, source_account, delay_ms);
         return;
      }
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

   // Extract account number and build traceable comment: "M12345#98765"
   string comment = "M" + IntegerToString(master_ticket) + "#" + ExtractAccountNumber(source_account);

   // Execute order
   int ticket = -1;
   for(int attempt = 0; attempt < MaxRetries; attempt++)
   {
      RefreshRates();

      if(order_type == OP_BUY || order_type == OP_SELL)
      {
         double exec_price = (order_type == OP_BUY) ? Ask : Bid;
         ticket = OrderSend(symbol, order_type, lots, exec_price, Slippage, sl, tp,
                           comment, magic, 0, clrGreen);
      }
      else
      {
         ticket = OrderSend(symbol, order_type, lots, price, Slippage, sl, tp,
                           comment, magic, 0, clrBlue);
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
//| Place pending order at original price                            |
//+------------------------------------------------------------------+
void PlacePendingOrder(int master_ticket, string symbol, string order_type_str,
                       double lots, double price, double sl, double tp, int magic, string source_account, int delay_ms)
{
   // Check if pending order already exists
   if(GetPendingTicket(master_ticket) > 0)
   {
      Print("Pending order already exists for master #", master_ticket);
      return;
   }

   int base_order_type = GetOrderType(order_type_str);
   if(base_order_type == -1)
   {
      Print("ERROR: Invalid order type: ", order_type_str);
      return;
   }

   // Convert to pending order type
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
      Print("ERROR: Cannot create pending order for type: ", order_type_str);
      return;
   }

   // Normalize values
   lots = NormalizeDouble(lots, 2);
   price = NormalizeDouble(price, Digits);
   sl = (sl > 0) ? NormalizeDouble(sl, Digits) : 0;
   tp = (tp > 0) ? NormalizeDouble(tp, Digits) : 0;

   // Extract account number and build traceable comment: "P12345#98765"
   string comment = "P" + IntegerToString(master_ticket) + "#" + ExtractAccountNumber(source_account);

   int ticket = OrderSend(symbol, pending_type, lots, price, Slippage, sl, tp,
                          comment, magic, 0, clrBlue);

   if(ticket > 0)
   {
      Print("Pending order placed: #", ticket, " for master #", master_ticket, " at price ", price);
      AddPendingOrderMapping(master_ticket, ticket);
   }
   else
   {
      Print("Failed to place pending order for master #", master_ticket, " Error: ", GetLastError());
   }
}

//+------------------------------------------------------------------+
//| Cancel pending order                                              |
//+------------------------------------------------------------------+
void CancelPendingOrder(int master_ticket)
{
   int pending_ticket = GetPendingTicket(master_ticket);
   if(pending_ticket <= 0)
      return;

   if(OrderDelete(pending_ticket))
   {
      Print("Pending order cancelled: #", pending_ticket, " for master #", master_ticket);
      RemovePendingOrderMapping(master_ticket);
   }
   else
   {
      Print("Failed to cancel pending order #", pending_ticket, " Error: ", GetLastError());
   }
}

//+------------------------------------------------------------------+
//| Order mapping functions                                           |
//+------------------------------------------------------------------+
void AddOrderMapping(int master_ticket, int slave_ticket)
{
   int size = ArraySize(g_order_map);
   ArrayResize(g_order_map, size + 1);
   g_order_map[size].master_ticket = master_ticket;
   g_order_map[size].slave_ticket = slave_ticket;
}

int GetSlaveTicket(int master_ticket)
{
   for(int i = 0; i < ArraySize(g_order_map); i++)
   {
      if(g_order_map[i].master_ticket == master_ticket)
         return g_order_map[i].slave_ticket;
   }
   return -1;
}

void RemoveOrderMapping(int master_ticket)
{
   for(int i = 0; i < ArraySize(g_order_map); i++)
   {
      if(g_order_map[i].master_ticket == master_ticket)
      {
         // Shift array
         for(int j = i; j < ArraySize(g_order_map) - 1; j++)
         {
            g_order_map[j] = g_order_map[j + 1];
         }
         ArrayResize(g_order_map, ArraySize(g_order_map) - 1);
         break;
      }
   }
}

//+------------------------------------------------------------------+
//| Pending order mapping functions                                   |
//+------------------------------------------------------------------+
void AddPendingOrderMapping(int master_ticket, int pending_ticket)
{
   int size = ArraySize(g_pending_order_map);
   ArrayResize(g_pending_order_map, size + 1);
   g_pending_order_map[size].master_ticket = master_ticket;
   g_pending_order_map[size].pending_ticket = pending_ticket;
}

int GetPendingTicket(int master_ticket)
{
   for(int i = 0; i < ArraySize(g_pending_order_map); i++)
   {
      if(g_pending_order_map[i].master_ticket == master_ticket)
         return g_pending_order_map[i].pending_ticket;
   }
   return -1;
}

void RemovePendingOrderMapping(int master_ticket)
{
   for(int i = 0; i < ArraySize(g_pending_order_map); i++)
   {
      if(g_pending_order_map[i].master_ticket == master_ticket)
      {
         // Shift array
         for(int j = i; j < ArraySize(g_pending_order_map) - 1; j++)
         {
            g_pending_order_map[j] = g_pending_order_map[j + 1];
         }
         ArrayResize(g_pending_order_map, ArraySize(g_pending_order_map) - 1);
         break;
      }
   }
}