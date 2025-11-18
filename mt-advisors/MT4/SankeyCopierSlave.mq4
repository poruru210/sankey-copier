//+------------------------------------------------------------------+
//|                                        SankeyCopierSlave.mq4      |
//|                        Copyright 2025, SANKEY Copier Project      |
//|                                                                  |
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, SANKEY Copier Project"
#property link      ""
#property version   "1.00"  // VERSION_PLACEHOLDER
#property icon      "app.ico"
#property strict

//--- Include common headers
#include <SankeyCopier/SankeyCopierCommon.mqh>
#include <SankeyCopier/SankeyCopierZmq.mqh>
#include <SankeyCopier/SankeyCopierMapping.mqh>
#include <SankeyCopier/SankeyCopierGridPanel.mqh>
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
input bool     ShowConfigPanel = true;                       // Show configuration panel on chart

//--- Global variables
string      AccountID;                  // Auto-generated from broker + account number
HANDLE_TYPE g_zmq_context = -1;
HANDLE_TYPE g_zmq_trade_socket = -1;    // Socket for receiving trade signals
HANDLE_TYPE g_zmq_config_socket = -1;   // Socket for receiving configuration
bool        g_initialized = false;
datetime    g_last_heartbeat = 0;
string      g_current_master = "";      // Currently configured master account
string      g_trade_group_id = "";      // Current trade group subscription
bool        g_config_requested = false; // Track if config has been requested

// Ticket mapping arrays (structures defined in SankeyCopierMapping.mqh)
TicketMapping g_order_map[];
PendingTicketMapping g_pending_order_map[];

//--- Extended configuration variables (from ConfigMessage)
bool           g_config_enabled = true;          // Whether copying is enabled
double         g_config_lot_multiplier = 1.0;    // Lot multiplier (default 1.0)
bool           g_config_reverse_trade = false;   // Reverse trades (Buy<->Sell)
SymbolMapping  g_symbol_mappings[];              // Symbol mappings

//--- Configuration panel
CGridPanel     g_config_panel;                   // Grid panel for displaying configuration
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

   // Initialize ZMQ context
   g_zmq_context = InitializeZmqContext();
   if(g_zmq_context < 0)
      return INIT_FAILED;

   // Create and connect trade signal socket (SUB to port 5556)
   g_zmq_trade_socket = CreateAndConnectZmqSocket(g_zmq_context, ZMQ_SUB, TradeServerAddress, "Slave Trade SUB");
   if(g_zmq_trade_socket < 0)
   {
      CleanupZmqContext(g_zmq_context);
      return INIT_FAILED;
   }

   // Create and connect config socket (SUB to port 5557)
   g_zmq_config_socket = CreateAndConnectZmqSocket(g_zmq_context, ZMQ_SUB, ConfigServerAddress, "Slave Config SUB");
   if(g_zmq_config_socket < 0)
   {
      CleanupZmqSocket(g_zmq_trade_socket, "Slave Trade SUB");
      CleanupZmqContext(g_zmq_context);
      return INIT_FAILED;
   }

   // Subscribe to config messages for this account ID
   if(!SubscribeToTopic(g_zmq_config_socket, AccountID))
   {
      CleanupZmqMultiSocket(g_zmq_trade_socket, g_zmq_config_socket, g_zmq_context, "Slave Trade SUB", "Slave Config SUB");
      return INIT_FAILED;
   }

   ArrayResize(g_order_map, 0);
   ArrayResize(g_pending_order_map, 0);

   // Initialize configuration arrays
   ArrayResize(g_symbol_mappings, 0);
   ArrayResize(g_filters.allowed_symbols, 0);
   ArrayResize(g_filters.blocked_symbols, 0);
   ArrayResize(g_filters.allowed_magic_numbers, 0);
   ArrayResize(g_filters.blocked_magic_numbers, 0);

   g_initialized = true;

   // Set up timer for heartbeat and config messages (1 second interval)
   EventSetTimer(1);

   // Initialize configuration panel (Grid Panel)
   if(ShowConfigPanel)
   {
      g_config_panel.Initialize("SankeyCopierPanel_", 10, 20, 200, 15);
      g_config_panel.SetTitle("Sankey Copier - Slave", PANEL_COLOR_TITLE);

      // Add initial rows
      string status_vals[] = {"Status:", "DISABLED"};
      color status_cols[] = {PANEL_COLOR_LABEL, PANEL_COLOR_DISABLED};
      g_config_panel.AddRow("status", status_vals, status_cols);

      string master_vals[] = {"Master:", "N/A"};
      color master_cols[] = {PANEL_COLOR_LABEL, PANEL_COLOR_VALUE};
      g_config_panel.AddRow("master", master_vals, master_cols);

      string lot_vals[] = {"Lot Mult:", "1.00x"};
      color lot_cols[] = {PANEL_COLOR_LABEL, PANEL_COLOR_VALUE};
      g_config_panel.AddRow("lot", lot_vals, lot_cols);

      string reverse_vals[] = {"Reverse:", "OFF"};
      color reverse_cols[] = {PANEL_COLOR_LABEL, PANEL_COLOR_VALUE};
      g_config_panel.AddRow("reverse", reverse_vals, reverse_cols);

      string version_vals[] = {"Config Ver:", "0"};
      color version_cols[] = {PANEL_COLOR_LABEL, PANEL_COLOR_VALUE};
      g_config_panel.AddRow("version", version_vals, version_cols);

      string symbols_vals[] = {"Symbols:", "0"};
      color symbols_cols[] = {PANEL_COLOR_LABEL, PANEL_COLOR_VALUE};
      g_config_panel.AddRow("symbols", symbols_vals, symbols_cols);
   }

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

   // Delete configuration panel
   if(ShowConfigPanel)
      g_config_panel.Delete();

   // Cleanup ZMQ resources
   CleanupZmqMultiSocket(g_zmq_trade_socket, g_zmq_config_socket, g_zmq_context, "Slave Trade SUB", "Slave Config SUB");

   Print("=== SankeyCopier Slave EA (MT4) Stopped ===");
}

//+------------------------------------------------------------------+
//| Timer function (called every 1 second)                            |
//+------------------------------------------------------------------+
void OnTimer()
{
   if(!g_initialized)
      return;

   // Send heartbeat every HEARTBEAT_INTERVAL_SECONDS
   datetime now = TimeLocal();

   if(now - g_last_heartbeat >= HEARTBEAT_INTERVAL_SECONDS)
   {
      bool heartbeat_sent = SendHeartbeatMessage(g_zmq_context, "tcp://localhost:5555", AccountID, "Slave", "MT4");

      if(heartbeat_sent)
      {
         g_last_heartbeat = TimeLocal();

         // On first successful heartbeat, request configuration from server
         if(!g_config_requested)
         {
            Print("[INFO] First heartbeat successful, requesting configuration...");
            if(SendRequestConfigMessage(g_zmq_context, "tcp://localhost:5555", AccountID))
            {
               g_config_requested = true;
               Print("[INFO] Configuration request sent successfully");
            }
            else
            {
               Print("[ERROR] Failed to send configuration request, will retry on next heartbeat");
            }
         }
      }
   }

   // Check for configuration messages (MessagePack format)
   uchar config_buffer[];
   ArrayResize(config_buffer, MESSAGE_BUFFER_SIZE);
   int config_bytes = zmq_socket_receive(g_zmq_config_socket, config_buffer, MESSAGE_BUFFER_SIZE);

   if(config_bytes > 0)
   {
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

         // Update configuration panel
         if(ShowConfigPanel)
         {
            // Update status row
            string status_vals[] = {"Status:", g_config_enabled ? "ENABLED" : "DISABLED"};
            color status_cols[] = {PANEL_COLOR_LABEL, g_config_enabled ? PANEL_COLOR_ENABLED : PANEL_COLOR_DISABLED};
            g_config_panel.UpdateRow("status", status_vals, status_cols);

            // Update master row
            string master_vals[] = {"Master:", g_current_master == "" ? "N/A" : g_current_master};
            color master_cols[] = {PANEL_COLOR_LABEL, PANEL_COLOR_VALUE};
            g_config_panel.UpdateRow("master", master_vals, master_cols);

            // Update lot multiplier row
            string lot_vals[] = {"Lot Mult:", StringFormat("%.2fx", g_config_lot_multiplier)};
            color lot_cols[] = {PANEL_COLOR_LABEL, PANEL_COLOR_VALUE};
            g_config_panel.UpdateRow("lot", lot_vals, lot_cols);

            // Update reverse trade row
            string reverse_vals[] = {"Reverse:", g_config_reverse_trade ? "ON" : "OFF"};
            color reverse_cols[] = {PANEL_COLOR_LABEL, PANEL_COLOR_VALUE};
            g_config_panel.UpdateRow("reverse", reverse_vals, reverse_cols);

            // Update config version row
            string version_vals[] = {"Config Ver:", IntegerToString(g_config_version)};
            color version_cols[] = {PANEL_COLOR_LABEL, PANEL_COLOR_VALUE};
            g_config_panel.UpdateRow("version", version_vals, version_cols);

            // Update symbols row
            string symbols_vals[] = {"Symbols:", IntegerToString(ArraySize(g_symbol_mappings))};
            color symbols_cols[] = {PANEL_COLOR_LABEL, PANEL_COLOR_VALUE};
            g_config_panel.UpdateRow("symbols", symbols_vals, symbols_cols);
         }
      }
   }
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
   int slave_ticket = GetSlaveTicketFromMapping(g_order_map, master_ticket);
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
         AddTicketMapping(g_order_map, master_ticket, ticket);
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
   int slave_ticket = GetSlaveTicketFromMapping(g_order_map, master_ticket);
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
      RemoveTicketMapping(g_order_map, master_ticket);
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
   int slave_ticket = GetSlaveTicketFromMapping(g_order_map, master_ticket);
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
   if(GetPendingTicketFromMapping(g_pending_order_map, master_ticket) > 0)
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
      AddPendingTicketMapping(g_pending_order_map, master_ticket, ticket);
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
   int pending_ticket = GetPendingTicketFromMapping(g_pending_order_map, master_ticket);
   if(pending_ticket <= 0)
      return;

   if(OrderDelete(pending_ticket))
   {
      Print("Pending order cancelled: #", pending_ticket, " for master #", master_ticket);
      RemovePendingTicketMapping(g_pending_order_map, master_ticket);
   }
   else
   {
      Print("Failed to cancel pending order #", pending_ticket, " Error: ", GetLastError());
   }
}

// Ticket mapping functions are now provided by SankeyCopierMapping.mqh