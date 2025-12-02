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

//--- Forward declaration for SlaveTrade.mqh (must be before include)
bool g_received_via_timer = false; // Track if signal was received via OnTimer (for latency tracing)

//--- Include common headers
#include "../Include/SankeyCopier/Common.mqh"
#include "../Include/SankeyCopier/Zmq.mqh"
#include "../Include/SankeyCopier/Mapping.mqh"
#include "../Include/SankeyCopier/GridPanel.mqh"
#include "../Include/SankeyCopier/Messages.mqh"
#include "../Include/SankeyCopier/Trade.mqh"
#include "../Include/SankeyCopier/SlaveTrade.mqh"
#include "../Include/SankeyCopier/MessageParsing.mqh"
#include "../Include/SankeyCopier/Logging.mqh"

//--- Input parameters
// Note: Most trade settings (Slippage, MaxRetries, AllowNewOrders, etc.) are now
// configured via Web-UI and received through the config message from relay-server.
// ZMQ addresses are loaded from sankey_copier.ini (no input override)
input bool     ShowConfigPanel = true;              // Show configuration panel on chart
input int      PanelWidth = 280;                    // Configuration panel width (pixels)
input int      SignalPollingIntervalMs = 1000;      // Signal polling interval in ms [1000-5000] (MT4: 1s minimum)

//--- Resolved addresses (from sankey_copier.ini config file)
// 2-port architecture: PUSH (EA->Server) and SUB (Server->EA, unified for trades+configs)
string g_RelayAddress = "";
string g_TradeAddress = "";  // Unified SUB address for trades and configs

//--- Default values for trade execution (used before config is received)
#define DEFAULT_SLIPPAGE              30     // Default slippage in points
#define DEFAULT_MAX_RETRIES           3      // Default retry attempts
#define DEFAULT_MAX_SIGNAL_DELAY_MS   5000   // Default max signal delay

//--- Global variables
string      AccountID;                  // Auto-generated from broker + account number
HANDLE_TYPE g_zmq_context = -1;
HANDLE_TYPE g_zmq_socket = -1;
HANDLE_TYPE g_zmq_trade_socket = -1;    // Socket for receiving trade signals
HANDLE_TYPE g_zmq_config_socket = -1;   // Socket for receiving configuration
TicketMapping g_order_map[];
PendingTicketMapping g_pending_order_map[];
bool        g_initialized = false;
datetime    g_last_heartbeat = 0;
bool        g_config_requested = false;   // Track if config request has been sent
bool        g_last_trade_allowed = false; // Track auto-trading state for change detection

//--- Extended configuration variables (from ConfigMessage)
CopyConfig     g_configs[];                      // Array of active configurations
bool           g_has_received_config = false;    // Track if we have received at least one config

//--- Topic strings (generated via FFI)
string g_config_topic = "";
string g_vlogs_topic = "";

//--- Configuration panel
CGridPanel     g_config_panel;

int g_last_config_count = 0;

//+------------------------------------------------------------------+
//| Expert initialization function                                     |
//+------------------------------------------------------------------+
int OnInit()
{
   Print("=== SankeyCopier Slave EA (MT4) Starting ===");

   // Auto-generate AccountID from broker name and account number
   AccountID = GenerateAccountID();
   Print("Auto-generated AccountID: ", AccountID);

   // Generate topics using FFI
   ushort topic_buffer[256];
   int len = build_config_topic(AccountID, topic_buffer, 256);
   if(len > 0) 
   {
      g_config_topic = ShortArrayToString(topic_buffer);
   }
   else
   {
      Print("ERROR: Failed to build config topic");
      return INIT_FAILED;
   }

   len = get_global_config_topic(topic_buffer, 256);
   if(len > 0)
   {
      g_vlogs_topic = ShortArrayToString(topic_buffer);
   }
   else
   {
      Print("ERROR: Failed to build global config topic");
      return INIT_FAILED;
   }

   Print("Generated topics: Config=", g_config_topic, ", VLogs=", g_vlogs_topic);

   // Resolve addresses from sankey_copier.ini config file
   // 2-port architecture: PUSH (EA->Server) and SUB (Server->EA, unified)
   g_RelayAddress = GetPushAddress();
   g_TradeAddress = GetTradeSubAddress();

   Print("Resolved addresses: PUSH=", g_RelayAddress, ", SUB=", g_TradeAddress, " (unified)");

   // Initialize ZMQ context
   g_zmq_context = InitializeZmqContext();
   if(g_zmq_context < 0)
      return INIT_FAILED;

   // Create and connect trade signal socket (SUB) - uses unified PUB address
   g_zmq_trade_socket = CreateAndConnectZmqSocket(g_zmq_context, ZMQ_SUB, g_TradeAddress, "Slave Trade SUB");
   if(g_zmq_trade_socket < 0)
   {
      CleanupZmqContext(g_zmq_context);
      return INIT_FAILED;
   }

   // Create and connect config socket (SUB) - uses same unified PUB address
   g_zmq_config_socket = CreateAndConnectZmqSocket(g_zmq_context, ZMQ_SUB, g_TradeAddress, "Slave Config SUB");
   if(g_zmq_config_socket < 0)
   {
      CleanupZmqSocket(g_zmq_trade_socket, "Slave Trade SUB");
      CleanupZmqContext(g_zmq_context);
      return INIT_FAILED;
   }

   // Subscribe to config messages for this account ID
   if(!SubscribeToTopic(g_zmq_config_socket, g_config_topic))
   {
      CleanupZmqMultiSocket(g_zmq_trade_socket, g_zmq_config_socket, g_zmq_context, "Slave Trade SUB", "Slave Config SUB");
      return INIT_FAILED;
   }

   // Subscribe to VictoriaLogs config (global broadcast)
   if(!SubscribeToTopic(g_zmq_config_socket, g_vlogs_topic))
   {
      Print("WARNING: Failed to subscribe to vlogs_config topic");
   }

   // Recover ticket mappings from existing positions (restart recovery)
   int recovered = RecoverMappingsFromPositions(g_order_map, g_pending_order_map);
   if(recovered > 0)
   {
      Print("Recovered ", recovered, " position mappings from previous session");
   }
   else
   {
      Print("No previous position mappings to recover (fresh start)");
   }

   // Initialize configuration arrays
   ArrayResize(g_configs, 0);

   g_initialized = true;

   // Set up timer for signal polling (MT4: seconds only, minimum 1 second)
   // Also handles heartbeat (every HEARTBEAT_INTERVAL_SECONDS) and config messages
   int interval_sec = MathMax(1, SignalPollingIntervalMs / 1000);
   EventSetTimer(interval_sec);
   Print("Signal polling interval: ", interval_sec, " second(s)");

   // Initialize configuration panel (Grid Panel)
   if(ShowConfigPanel)
   {
      g_config_panel.InitializeSlavePanel("SankeyCopierPanel_", PanelWidth);
      // Show NO_CONFIGURATION status initially (no config received yet)
      g_config_panel.UpdateStatusRow(STATUS_NO_CONFIG);
      g_config_panel.UpdateServerRow(g_RelayAddress);
      // Symbol config is now per-Master from Web-UI, will be shown in carousel
      g_config_panel.UpdateSymbolConfig("", "", "");
   }

   Print("=== SankeyCopier Slave EA Initialized ===");

   // VictoriaLogs is configured via server-pushed vlogs_config message
   // (no local endpoint parameter needed)

   ChartRedraw();
   return INIT_SUCCEEDED;
}

//+------------------------------------------------------------------+
//| Expert deinitialization function                                  |
//+------------------------------------------------------------------+
void OnDeinit(const int reason)
{
   Print("=== SankeyCopier Slave EA (MT4) Stopping ===");

   // Flush VictoriaLogs before shutdown
   VLogsFlush();

   // Send unregister message to server
   SendUnregistrationMessage(g_zmq_context, g_RelayAddress, AccountID);

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
//| Timer function (called at SignalPollingIntervalMs interval)       |
//| Handles: signal polling, heartbeat, config messages               |
//+------------------------------------------------------------------+
void OnTimer()
{
   if(!g_initialized)
      return;

   // Poll for trade signals (ensures reception even without ticks)
   // This is the key improvement for reducing latency on low-activity symbols
   g_received_via_timer = true;  // Mark that signal will be received via OnTimer
   ProcessTradeSignals();

   // Check for auto-trading state change (IsTradeAllowed)
   bool current_trade_allowed = (bool)TerminalInfoInteger(TERMINAL_TRADE_ALLOWED);
   bool trade_state_changed = (current_trade_allowed != g_last_trade_allowed);

   // Send heartbeat every HEARTBEAT_INTERVAL_SECONDS OR on trade state change
   datetime now = TimeLocal();
   bool should_send_heartbeat = (now - g_last_heartbeat >= HEARTBEAT_INTERVAL_SECONDS) || trade_state_changed;

   if(should_send_heartbeat)
   {
      // Slave doesn't send symbol settings in heartbeat - those are managed per-Master config by relay server
      bool heartbeat_sent = SendHeartbeatMessage(g_zmq_context, g_RelayAddress, AccountID, "Slave", "MT4");

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
                   // Auto-trading OFF -> show DISABLED (Slave cannot trade)
                   g_config_panel.UpdateStatusRow(STATUS_DISABLED);
                }
               else
               {
                  // Auto-trading ON -> show actual config status
                  g_config_panel.UpdatePanelStatusFromConfigs(g_configs);
               }
               ChartRedraw();
            }

            // If auto-trading was just enabled, request configuration
            if(current_trade_allowed && !g_config_requested)
            {
               Print("[INFO] Auto-trading enabled, requesting configuration...");
               if(SendRequestConfigMessage(g_zmq_context, g_RelayAddress, AccountID, "Slave"))
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
               if(SendRequestConfigMessage(g_zmq_context, g_RelayAddress, AccountID, "Slave"))
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

         // Check for VLogs config message first (global broadcast)
         if(topic == g_vlogs_topic)
         {
            HANDLE_TYPE vlogs_handle = parse_vlogs_config(msgpack_payload, payload_len);
            if(vlogs_handle != 0 && vlogs_handle != -1)
            {
               VLogsApplyConfig(vlogs_handle, "slave", AccountID);
               vlogs_config_free(vlogs_handle);
            }
         }
         // Try to parse as SlaveConfig first
         else if(topic == g_config_topic)
         {
            HANDLE_TYPE config_handle = parse_slave_config(msgpack_payload, payload_len);
            if(config_handle > 0)
            {
               string master_account = slave_config_get_string(config_handle, "master_account");
               if(master_account != "")
               {
                  // Valid SlaveConfig - process it
                  slave_config_free(config_handle);
                  ProcessConfigMessage(msgpack_payload, payload_len, g_configs, g_zmq_trade_socket,
                                       g_zmq_context, g_RelayAddress, AccountID);
                  g_has_received_config = true;
               }
               else
               {
                  // Not a SlaveConfig - try PositionSnapshot
                  slave_config_free(config_handle);
                  ProcessPositionSnapshot(msgpack_payload, payload_len);
               }
            }
            else
            {
               // Failed to parse as SlaveConfig - try PositionSnapshot
               ProcessPositionSnapshot(msgpack_payload, payload_len);
            }
         }
         
         
         if(ArraySize(g_configs) != g_last_config_count)
         {
            g_last_config_count = ArraySize(g_configs);
         }

         // Update configuration panel
         if(ShowConfigPanel)
         {
            // Check local auto-trading state - if OFF, show ENABLED (yellow) as warning
            bool local_trade_allowed = (bool)TerminalInfoInteger(TERMINAL_TRADE_ALLOWED);
            if(!local_trade_allowed)
            {
               // Local auto-trading OFF -> show DISABLED (Slave cannot trade)
               g_config_panel.UpdateStatusRow(STATUS_DISABLED);
            }
            else
            {
               // Local auto-trading ON -> show actual config status from server
               g_config_panel.UpdatePanelStatusFromConfigs(g_configs);
            }

            // Update carousel display with detailed copy settings
            g_config_panel.UpdateCarouselConfigs(g_configs);
            ChartRedraw();
         }
      }
   }
}

//+------------------------------------------------------------------+
//| Process trade signals from ZeroMQ socket                          |
//| Called from both OnTick() and OnTimer() for low-latency reception |
//+------------------------------------------------------------------+
void ProcessTradeSignals()
{
   // Check for trade signal messages (MessagePack format)
   // Note: PositionSnapshot is received via config socket in OnTimer
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

         // Trade socket receives messages on master_account topic
         // This includes both TradeSignal and MasterConfigMessage
         // We need to filter out non-TradeSignal messages
         
         // Try to parse as TradeSignal first
         HANDLE_TYPE test_handle = parse_trade_signal(msgpack_payload, payload_len);
         if(test_handle > 0)
         {
            // Valid TradeSignal - free test handle and process normally
            trade_signal_free(test_handle);
            ProcessTradeSignal(msgpack_payload, payload_len);
         }
         else
         {
            // Not a TradeSignal (likely MasterConfigMessage or other message type)
            // Silently ignore - these messages are handled by config_socket
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

   // Check if any pending orders have been filled
   CheckPendingOrderFills(g_pending_order_map, g_order_map);

   // Process trade signals (also called from OnTimer for polling)
   g_received_via_timer = false;  // Mark that signal will be received via OnTick
   ProcessTradeSignals();

   // Flush VictoriaLogs periodically
   VLogsFlushIfNeeded();
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

   // Log trade signal receipt with key details for traceability
   Print("[SIGNAL] ", action, " master:#", master_ticket, " ", symbol, " ", order_type_str, " ", lots, " lots @ ", open_price, " from ", source_account);

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

   // Get trade settings from config (use defaults as fallback)
   int trade_slippage = (g_configs[config_index].max_slippage > 0)
                        ? g_configs[config_index].max_slippage
                        : DEFAULT_SLIPPAGE;
   int max_retries = (g_configs[config_index].max_retries > 0)
                     ? g_configs[config_index].max_retries
                     : DEFAULT_MAX_RETRIES;
   int max_signal_delay = (g_configs[config_index].max_signal_delay_ms > 0)
                          ? g_configs[config_index].max_signal_delay_ms
                          : DEFAULT_MAX_SIGNAL_DELAY_MS;
   bool use_pending_for_delayed = g_configs[config_index].use_pending_order_for_delayed;
   bool allow_new_orders = g_configs[config_index].allow_new_orders;

   if(action == "Open")
   {
      if(!allow_new_orders)
      {
         Print("Open signal rejected: allow_new_orders=false (status=", g_configs[config_index].status, ") for master #", master_ticket);
         trade_signal_free(handle);
         return;
      }

      // Filtering (Symbol, Magic, Lot) is already handled by Relay Server
      // We process all signals received here

      // Symbol is already transformed by Relay Server (mapping + prefix/suffix applied)
      string transformed_symbol = symbol;

      // Transform lot size (supports multiplier and margin_ratio modes)
      double transformed_lots = TransformLotSize(lots, g_configs[config_index], transformed_symbol);
      string transformed_order_type = ReverseOrderType(order_type_str, g_configs[config_index].reverse_trade);

      // Open order with transformed values (pass config settings)
      ExecuteOpenTrade(g_order_map, g_pending_order_map, master_ticket, transformed_symbol,
                       transformed_order_type, transformed_lots, open_price, stop_loss, take_profit,
                       timestamp, source_account, magic, trade_slippage,
                       max_signal_delay, use_pending_for_delayed, max_retries, DEFAULT_SLIPPAGE);
   }
   else if(action == "Close")
   {
      // Close trades are always allowed (to close existing positions)
      double close_ratio = trade_signal_get_double(handle, "close_ratio");
      ExecuteCloseTrade(g_order_map, master_ticket, close_ratio, trade_slippage, DEFAULT_SLIPPAGE);
      ExecuteCancelPendingOrder(g_pending_order_map, master_ticket);
   }
   else if(action == "Modify")
   {
      ExecuteModifyTrade(g_order_map, master_ticket, stop_loss, take_profit);
   }

   // Free the handle
   trade_signal_free(handle);
}

//+------------------------------------------------------------------+
//| Process position snapshot for sync (MT4)                          |
//| Called when Slave receives PositionSnapshot from Master           |
//+------------------------------------------------------------------+
void ProcessPositionSnapshot(uchar &data[], int data_len)
{
   Print("=== Processing Position Snapshot ===");

   // Parse the PositionSnapshot message
   HANDLE_TYPE handle = parse_position_snapshot(data, data_len);
   if(handle == 0 || handle == -1)
   {
      Print("ERROR: Failed to parse PositionSnapshot message");
      return;
   }

   // Get source account (master)
   string source_account = position_snapshot_get_string(handle, "source_account");
   if(source_account == "")
   {
      Print("ERROR: PositionSnapshot has empty source_account");
      position_snapshot_free(handle);
      return;
   }

   Print("PositionSnapshot from master: ", source_account);

   // Find matching config for this master
   int config_index = -1;
   for(int i = 0; i < ArraySize(g_configs); i++)
   {
      if(g_configs[i].master_account == source_account)
      {
         config_index = i;
         break;
      }
   }

   if(config_index == -1)
   {
      Print("PositionSnapshot rejected: No config for master ", source_account);
      position_snapshot_free(handle);
      return;
   }

   // Check sync_mode - should not be SKIP
   int sync_mode = g_configs[config_index].sync_mode;
   if(sync_mode == SYNC_MODE_SKIP)
   {
      Print("PositionSnapshot ignored: sync_mode is SKIP");
      position_snapshot_free(handle);
      return;
   }

   // Get sync parameters from config
   int limit_order_expiry = g_configs[config_index].limit_order_expiry_min;
   double market_sync_max_pips = g_configs[config_index].market_sync_max_pips;
   int trade_slippage = (g_configs[config_index].max_slippage > 0)
                        ? g_configs[config_index].max_slippage
                        : DEFAULT_SLIPPAGE;

   Print("Sync mode: ", (sync_mode == SYNC_MODE_LIMIT_ORDER) ? "LIMIT_ORDER" : "MARKET_ORDER");

   // Get position count
   int position_count = position_snapshot_get_positions_count(handle);
   Print("Positions to sync: ", position_count);

   int synced_count = 0;
   int skipped_count = 0;

   // Process each position
   for(int i = 0; i < position_count; i++)
   {
      // Extract position data
      long master_ticket_long = position_snapshot_get_position_int(handle, i, "ticket");
      int master_ticket = (int)master_ticket_long;
      string symbol = position_snapshot_get_position_string(handle, i, "symbol");
      string order_type_str = position_snapshot_get_position_string(handle, i, "order_type");
      double lots = position_snapshot_get_position_double(handle, i, "lots");
      double open_price = position_snapshot_get_position_double(handle, i, "open_price");
      double sl = position_snapshot_get_position_double(handle, i, "stop_loss");
      double tp = position_snapshot_get_position_double(handle, i, "take_profit");
      long magic_long = position_snapshot_get_position_int(handle, i, "magic_number");
      int magic_number = (int)magic_long;

      Print("Position ", i + 1, "/", position_count, ": #", master_ticket,
            " ", symbol, " ", order_type_str, " ", lots, " lots @ ", open_price);

      // Check if we already have this position mapped
      if(GetSlaveTicketFromMapping(g_order_map, master_ticket) > 0)
      {
         Print("  -> Already mapped, skipping");
         skipped_count++;
         continue;
      }

      // Symbol is already transformed by Relay Server (mapping + prefix/suffix applied)
      string transformed_symbol = symbol;

      // Transform lot size
      double transformed_lots = TransformLotSize(lots, g_configs[config_index], transformed_symbol);

      // Reverse order type if configured
      string transformed_order_type = ReverseOrderType(order_type_str, g_configs[config_index].reverse_trade);

      // Execute sync based on mode
      if(sync_mode == SYNC_MODE_LIMIT_ORDER)
      {
         SyncWithLimitOrder(g_pending_order_map, master_ticket, transformed_symbol,
                            transformed_order_type, transformed_lots, open_price, sl, tp,
                            source_account, magic_number, limit_order_expiry);
         synced_count++;
      }
      else if(sync_mode == SYNC_MODE_MARKET_ORDER)
      {
         if(SyncWithMarketOrder(g_order_map, master_ticket, transformed_symbol,
                                transformed_order_type, transformed_lots, open_price, sl, tp,
                                source_account, magic_number, trade_slippage,
                                market_sync_max_pips, DEFAULT_SLIPPAGE))
         {
            synced_count++;
         }
         else
         {
            Print("  -> Price deviation too large, skipped");
            skipped_count++;
         }
      }
   }

   Print("=== Position Sync Complete: ", synced_count, " synced, ", skipped_count, " skipped ===");
   position_snapshot_free(handle);
}

// Trade functions are now provided by SlaveTrade.mqh

//+------------------------------------------------------------------+
//| Chart event handler (for panel click navigation)                  |
//+------------------------------------------------------------------+
void OnChartEvent(const int id, const long &lparam, const double &dparam, const string &sparam)
{
   // Handle mouse click events for carousel navigation
   if(id == CHARTEVENT_CLICK && ShowConfigPanel)
   {
      int x = (int)lparam;
      int y = (int)dparam;

      // Check if click is on carousel navigation
      if(g_config_panel.HandleChartClick(x, y))
      {
         // Click was handled by panel
         return;
      }
   }
}