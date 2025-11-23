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
#include <SankeyCopier/Common.mqh>
#include <SankeyCopier/Zmq.mqh>
#include <SankeyCopier/Mapping.mqh>
#include <SankeyCopier/GridPanel.mqh>
#include <SankeyCopier/Messages.mqh>
#include <SankeyCopier/Trade.mqh>

//--- Input parameters
input string   RelayServerAddress = DEFAULT_ADDR_PULL;       // Address to send heartbeats/requests (PULL)
input string   TradeSignalSourceAddress = DEFAULT_ADDR_PUB_TRADE; // Address to receive trade signals (SUB)
input string   ConfigSourceAddress = DEFAULT_ADDR_PUB_CONFIG;     // Address to receive configuration (SUB)
input int      Slippage = 3;                                 // Maximum slippage in points
input int      MaxRetries = 3;                               // Maximum order retries
input bool     AllowNewOrders = true;                        // Allow opening new orders
input bool     AllowCloseOrders = true;                      // Allow closing orders
input int      MaxSignalDelayMs = 5000;                      // Maximum allowed signal delay (milliseconds)
input bool     UsePendingOrderForDelayed = false;            // Use pending order for delayed signals
input string   SymbolPrefix = "";                       // Symbol prefix to add (e.g. "pro.")
input string   SymbolSuffix = "";                       // Symbol suffix to add (e.g. ".m")
input string   SymbolMap = "";                          // Symbol mapping (e.g. "XAUUSD=GOLD")
input bool     ShowConfigPanel = true;                       // Show configuration panel on chart
input int      PanelWidth = 280;                             // Configuration panel width (pixels)

//--- Global variables
string      AccountID;                  // Auto-generated from broker + account number
HANDLE_TYPE g_zmq_context = -1;
HANDLE_TYPE g_zmq_trade_socket = -1;    // Socket for receiving trade signals
HANDLE_TYPE g_zmq_config_socket = -1;   // Socket for receiving configuration
TicketMapping g_order_map[];
PendingTicketMapping g_pending_order_map[];
SymbolMapping g_local_mappings[];
bool        g_initialized = false;
datetime    g_last_heartbeat = 0;
bool        g_config_requested = false; // Track if config has been requested
bool        g_last_trade_allowed = false; // Track auto-trading state for change detection

//--- Extended configuration variables (from ConfigMessage)
CopyConfig     g_configs[];                      // Array of active configurations
bool           g_has_received_config = false;    // Track if we have received at least one config

//--- Configuration panel
CGridPanel     g_config_panel;                   // Grid panel for displaying configuration

int g_last_config_count = 0;

//+------------------------------------------------------------------+
//| Expert initialization function                                     |
//+------------------------------------------------------------------+
int OnInit()
{
   Print("=== SankeyCopier Slave EA (MT4) Starting ===");
   
   // Parse symbol mapping
   ParseSymbolMappingString(SymbolMap, g_local_mappings);
   Print("Parsed ", ArraySize(g_local_mappings), " local symbol mappings");

   // Auto-generate AccountID from broker name and account number
   AccountID = GenerateAccountID();
   Print("Auto-generated AccountID: ", AccountID);

   // Initialize ZMQ context
   g_zmq_context = InitializeZmqContext();
   if(g_zmq_context < 0)
      return INIT_FAILED;

   // Create and connect trade signal socket (SUB to port 5556)
   g_zmq_trade_socket = CreateAndConnectZmqSocket(g_zmq_context, ZMQ_SUB, TradeSignalSourceAddress, "Slave Trade SUB");
   if(g_zmq_trade_socket < 0)
   {
      CleanupZmqContext(g_zmq_context);
      return INIT_FAILED;
   }

   // Create and connect config socket (SUB to port 5557)
   g_zmq_config_socket = CreateAndConnectZmqSocket(g_zmq_context, ZMQ_SUB, ConfigSourceAddress, "Slave Config SUB");
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
   ArrayResize(g_configs, 0);

   g_initialized = true;

   // Set up timer for heartbeat and config messages (1 second interval)
   EventSetTimer(1);

   // Initialize configuration panel (Grid Panel)
   if(ShowConfigPanel)
   {
      g_config_panel.InitializeSlavePanel("SankeyCopierPanel_", PanelWidth);
      g_config_panel.UpdateStatusRow(STATUS_CONNECTED); // Initial state
      g_config_panel.UpdateServerRow(RelayServerAddress);
      g_config_panel.UpdateSymbolConfig(SymbolPrefix, SymbolSuffix, SymbolMap);
      // Show "Waiting" message initially
      g_config_panel.ShowMessage("Waiting for Web UI configuration...", clrYellow);
   }

   Print("=== SankeyCopier Slave EA Initialized ===");

   ChartRedraw();
   return INIT_SUCCEEDED;
}

//+------------------------------------------------------------------+
//| Expert deinitialization function                                  |
//+------------------------------------------------------------------+
void OnDeinit(const int reason)
{
   Print("=== SankeyCopier Slave EA (MT4) Stopping ===");

   // Send unregister message to server
   SendUnregistrationMessage(g_zmq_context, RelayServerAddress, AccountID);

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

   // Check for auto-trading state change (IsTradeAllowed)
   bool current_trade_allowed = (bool)TerminalInfoInteger(TERMINAL_TRADE_ALLOWED);
   bool trade_state_changed = (current_trade_allowed != g_last_trade_allowed);

   // Send heartbeat every HEARTBEAT_INTERVAL_SECONDS OR on trade state change
   datetime now = TimeLocal();
   bool should_send_heartbeat = (now - g_last_heartbeat >= HEARTBEAT_INTERVAL_SECONDS) || trade_state_changed;

   if(should_send_heartbeat)
   {
      bool heartbeat_sent = SendHeartbeatMessage(g_zmq_context, RelayServerAddress, AccountID, "Slave", "MT4", SymbolPrefix, SymbolSuffix, SymbolMap);

      if(heartbeat_sent)
      {
         g_last_heartbeat = TimeLocal();

         // If trade state changed, log it and update panel
         if(trade_state_changed)
         {
            Print("[INFO] Auto-trading state changed: ", g_last_trade_allowed, " -> ", current_trade_allowed);
            g_last_trade_allowed = current_trade_allowed;

            // Update panel to reflect auto-trading state
            // Only update if we have received config, otherwise keep showing "Waiting"
            if(ShowConfigPanel && g_has_received_config)
            {
               if(!current_trade_allowed)
               {
                  // Auto-trading OFF -> show ENABLED (yellow) as warning, like Web UI
                  g_config_panel.UpdateStatusRow(STATUS_ENABLED);
               }
               else
               {
                  // Auto-trading ON -> show actual config status
                  int status_to_show = STATUS_DISABLED;
                  if(ArraySize(g_configs) > 0) status_to_show = STATUS_ENABLED; 
                  
                  for(int i=0; i<ArraySize(g_configs); i++)
                  {
                     if(g_configs[i].status == STATUS_CONNECTED)
                     {
                        status_to_show = STATUS_CONNECTED;
                        break;
                     }
                  }
                  g_config_panel.UpdateStatusRow(status_to_show);
               }
            }

            // If auto-trading was just enabled, request configuration
            if(current_trade_allowed && !g_config_requested)
            {
               Print("[INFO] Auto-trading enabled, requesting configuration...");
               if(SendRequestConfigMessage(g_zmq_context, RelayServerAddress, AccountID, "Slave"))
               {
                  g_config_requested = true;
                  Print("[INFO] Configuration request sent successfully");
               }
               else
               {
                  Print("[ERROR] Failed to send configuration request, will retry on next state change");
               }
            }
         }
         else
         {
            // On first successful heartbeat (normal interval), request configuration if not yet requested
            if(!g_config_requested)
            {
               Print("[INFO] First heartbeat successful, requesting configuration...");
               if(SendRequestConfigMessage(g_zmq_context, RelayServerAddress, AccountID, "Slave"))
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
         ProcessConfigMessage(msgpack_payload, payload_len, g_configs, g_zmq_trade_socket);

         g_has_received_config = true;
         
         if(ArraySize(g_configs) != g_last_config_count)
         {
            Print("DEBUG: Config count changed: ", g_last_config_count, " -> ", ArraySize(g_configs));
            g_last_config_count = ArraySize(g_configs);
         }

         // Update configuration panel
         if(ShowConfigPanel)
         {
            g_config_panel.HideMessage(); // Switch to grid view
            
            // Check local auto-trading state - if OFF, show ENABLED (yellow) as warning
            bool local_trade_allowed = (bool)TerminalInfoInteger(TERMINAL_TRADE_ALLOWED);
            if(!local_trade_allowed)
            {
               // Local auto-trading OFF -> show ENABLED (yellow) warning, like Web UI
               g_config_panel.UpdateStatusRow(STATUS_ENABLED);
            }
            else
            {
               // Local auto-trading ON -> show actual config status from server
               // If we have at least one connected master, show CONNECTED
               bool any_connected = false;
               for(int i=0; i<ArraySize(g_configs); i++)
               {
                  if(g_configs[i].status == STATUS_CONNECTED)
                  {
                     any_connected = true;
                     break;
                  }
               }
               
               if(any_connected)
                  g_config_panel.UpdateStatusRow(STATUS_CONNECTED);
               else if(ArraySize(g_configs) > 0)
                  g_config_panel.UpdateStatusRow(STATUS_ENABLED); // Has configs but none connected?
               else
                  g_config_panel.UpdateStatusRow(STATUS_DISABLED);
            }
            
            // Update dynamic config list
            g_config_panel.UpdateConfigList(g_configs);
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

   Print("Processing ", action, " for master ticket #", master_ticket, " from ", source_account);

   // Find matching config for this master
   int config_index = -1;
   for(int i=0; i<ArraySize(g_configs); i++)
   {
      if(g_configs[i].master_account == source_account)
      {
         config_index = i;
         break;
      }
   }

   if(config_index == -1)
   {
      Print("Trade signal rejected: No active configuration for master ", source_account);
      trade_signal_free(handle);
      return;
   }

   // Check if connected to Master before processing any trades
   if(g_configs[config_index].status != STATUS_CONNECTED)
   {
      Print("Trade signal rejected: Not connected to Master (status=", g_configs[config_index].status, "). Master ticket #", master_ticket);
      trade_signal_free(handle);
      return;
   }

   if(action == "Open")
   {
      if(AllowNewOrders)
      {
         // Apply filtering
         if(!ShouldProcessTrade(symbol, magic, g_configs[config_index]))
         {
            Print("Trade filtered out: ", symbol, " magic=", magic);
            trade_signal_free(handle);
            return; // Do not proceed if filtered
         }

         // Apply transformations
         string mapped_symbol = TransformSymbol(symbol, g_configs[config_index].symbol_mappings);
         mapped_symbol = TransformSymbol(mapped_symbol, g_local_mappings); // Apply local mapping
         string transformed_symbol = GetLocalSymbol(mapped_symbol, SymbolPrefix, SymbolSuffix);
         
         // Transform lot size
         double transformed_lots = TransformLotSize(lots, g_configs[config_index].lot_multiplier, transformed_symbol);
         string transformed_order_type = ReverseOrderType(order_type_str, g_configs[config_index].reverse_trade);

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
   datetime current_time = TimeGMT();
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