//+------------------------------------------------------------------+
//|                                        SankeyCopierSlave.mq5      |
//|                        Copyright 2025, SANKEY Copier Project      |
//|                                                                  |
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, SANKEY Copier Project"
#property link      ""
#property version   "1.00"  // VERSION_PLACEHOLDER
#property icon      "app.ico"

//--- Forward declaration for SlaveTrade.mqh (must be before include)
bool g_received_via_timer = false; // Track if signal was received via OnTimer (for latency tracing)

#include "../Include/Trade/Trade.mqh"

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
input int      SignalPollingIntervalMs = 100;       // Signal polling interval in ms [100-5000]

//--- Resolved addresses (from sankey_copier.ini config file)
// 2-port architecture: PUSH (EA->Server) and SUB (Server->EA, unified for trades+configs)
string g_RelayAddress = "";
string g_SubAddress = "";  // Unified SUB address for trades and configs

//--- Default values for trade execution (used before config is received)
#define DEFAULT_SLIPPAGE              30     // Default slippage in points
#define DEFAULT_MAX_RETRIES           3      // Default retry attempts
#define DEFAULT_MAX_SIGNAL_DELAY_MS   5000   // Default max signal delay

//--- Global variables
string      AccountID;                  // Auto-generated from broker + account number
HANDLE_TYPE g_zmq_context = -1;
HANDLE_TYPE g_zmq_trade_socket = -1;    // Socket for receiving trade signals
HANDLE_TYPE g_zmq_config_socket = -1;   // Socket for receiving configuration
CTrade      g_trade;
TicketMapping g_order_map[];
PendingTicketMapping g_pending_order_map[];
// g_local_mappings removed - all symbol transformation now handled by Relay Server
bool        g_initialized = false;
datetime    g_last_heartbeat = 0;
bool        g_config_requested = false; // Track if config has been requested
bool        g_last_trade_allowed = false; // Track auto-trading state for change detection
HANDLE_TYPE g_ea_state = 0;             // Rust EA State manager
// g_received_via_timer is defined before includes (required for SlaveTrade.mqh)

//--- Extended configuration variables (from ConfigMessage)
CopyConfig     g_configs[];                      // Array of active configurations
bool           g_has_received_config = false;    // Track if we have received at least one config
string         g_config_topic = "";              // Config topic (generated via FFI)
string         g_vlogs_topic = "";               // VLogs topic (generated via FFI)
bool           g_sync_topic_subscribed = false;  // Track if sync topic has been subscribed
string         g_sync_topic = "";                // Sync topic for receiving PositionSnapshot

//--- Configuration panel
CGridPanel     g_config_panel;                   // Grid panel for displaying configuration
int            g_last_config_count = -1;         // Debug: track config count changes

//+------------------------------------------------------------------+
//| Expert initialization function                                     |
//+------------------------------------------------------------------+
int OnInit()
{
   Print("=== SankeyCopier Slave EA (MT5) Starting ===");

   // Auto-generate AccountID from broker name and account number
   AccountID = GenerateAccountID();
   Print("Auto-generated AccountID: ", AccountID);

   // Symbol transformation is now handled by Relay Server
   // Slave EA receives pre-transformed symbols

   // Load port configuration from sankey_copier.ini
   // 2-port architecture: Receiver (PULL) and Publisher (unified PUB for trades + configs)
   if(!LoadConfig())
   {
      Print("WARNING: Failed to load config file, using default ports");
   }
   else
   {
      Print("Config loaded: ReceiverPort=", GetReceiverPort(),
            ", PublisherPort=", GetPublisherPort(), " (unified)");
   }

   // Resolve addresses from sankey_copier.ini config file
   // 2-port architecture: PUSH (EA->Server) and SUB (Server->EA, unified)
   g_RelayAddress = GetPushAddress();
   g_SubAddress = GetTradeSubAddress();

   Print("Resolved addresses: PUSH=", g_RelayAddress, ", SUB=", g_SubAddress, " (unified)");

   // Initialize topics using FFI
   ushort topic_buffer[256];
   int len = build_config_topic(AccountID, topic_buffer, 256);
   if(len > 0) 
   {
      g_config_topic = ShortArrayToString(topic_buffer);
      Print("Generated config topic: ", g_config_topic);
   }
   else 
   {
      g_config_topic = AccountID; // Fallback
      Print("WARNING: Failed to generate config topic, using AccountID fallback: ", g_config_topic);
   }

   len = get_global_config_topic(topic_buffer, 256);
   if(len > 0) 
   {
      g_vlogs_topic = ShortArrayToString(topic_buffer);
      Print("Generated vlogs topic: ", g_vlogs_topic);
   }
   else 
   {
      Print("CRITICAL: Failed to generate vlogs topic from mt-bridge");
      return INIT_FAILED;
   }

   // Initialize ZMQ context
   g_zmq_context = InitializeZmqContext();
   if(g_zmq_context < 0)
      return INIT_FAILED;

   // Initialize EA State manager
   g_ea_state = ea_state_create();
   if(g_ea_state == 0)
   {
      Print("[ERROR] Failed to create EA State manager");
      zmq_context_destroy(g_zmq_context);
      return INIT_FAILED;
   }

   // Create and connect trade signal socket (SUB) - uses unified PUB address
   g_zmq_trade_socket = CreateAndConnectZmqSocket(g_zmq_context, ZMQ_SUB, g_SubAddress, "Slave Trade SUB");
   if(g_zmq_trade_socket < 0)
   {
      zmq_context_destroy(g_zmq_context);
      return INIT_FAILED;
   }

   // Create and connect config socket (SUB) - uses same unified PUB address
   g_zmq_config_socket = CreateAndConnectZmqSocket(g_zmq_context, ZMQ_SUB, g_SubAddress, "Slave Config SUB");
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

   // Subscribe to VictoriaLogs configuration messages (broadcast from relay-server)
   if(!SubscribeToTopic(g_zmq_config_socket, g_vlogs_topic))
   {
      CleanupZmqMultiSocket(g_zmq_trade_socket, g_zmq_config_socket, g_zmq_context, "Slave Trade SUB", "Slave Config SUB");
      return INIT_FAILED;
   }

   g_trade.SetExpertMagicNumber(0);
   g_trade.SetDeviationInPoints(DEFAULT_SLIPPAGE);  // Will be updated per-trade from config
   g_trade.SetTypeFilling(ORDER_FILLING_IOC);

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

   // Set up millisecond timer for signal polling
   // Also handles heartbeat (every HEARTBEAT_INTERVAL_SECONDS) and config messages
   int interval_ms = MathMax(100, MathMin(5000, SignalPollingIntervalMs));
   EventSetMillisecondTimer(interval_ms);
   Print("Signal polling interval: ", interval_ms, "ms");

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

   // VictoriaLogs is now configured via Web-UI settings received from relay-server
   // (vlogs_config message will be received after registration)

   ChartRedraw();
   return INIT_SUCCEEDED;
}

//+------------------------------------------------------------------+
//| Expert deinitialization function                                  |
//+------------------------------------------------------------------+
void OnDeinit(const int reason)
{
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

   // Cleanup EA State manager
   ea_state_free(g_ea_state);

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
      bool heartbeat_sent = SendHeartbeatMessage(g_zmq_context, g_RelayAddress, AccountID, "Slave", "MT5");

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

            // Request configuration logic using Rust EaState
            if(ea_state_should_request_config(g_ea_state, current_trade_allowed))
            {
               Print("[INFO] Requesting configuration (via EaState)...");
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
         // On first successful heartbeat (normal interval), request config via EaState
         if(ea_state_should_request_config(g_ea_state, current_trade_allowed))
         {
            Print("[INFO] First heartbeat/periodic check, requesting configuration (via EaState)...");
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

         // Check if this is a sync/ topic message (PositionSnapshot from Master)
         if(StringFind(topic, "sync/") == 0)
         {
            ProcessPositionSnapshot(msgpack_payload, payload_len);
         }
         // Handle VictoriaLogs configuration message
         else if(topic == g_vlogs_topic)
         {
            HANDLE_TYPE vlogs_handle = parse_vlogs_config(msgpack_payload, payload_len);
            if(vlogs_handle > 0)
            {
               VLogsApplyConfig(vlogs_handle, "slave", AccountID);
               vlogs_config_free(vlogs_handle);
            }
            else
            {
               Print("[ERROR] Failed to parse vlogs_config message");
            }
         }
         // Parse as SlaveConfig (config/{account_id} topic)
         else if(topic == g_config_topic)
         {
            ProcessConfigMessage(msgpack_payload, payload_len, g_configs, g_zmq_trade_socket,
                                 g_zmq_context, g_RelayAddress, AccountID);
            g_has_received_config = true;
            
            // Subscribe to sync/{master}/{slave} topic after receiving config
            // This is done dynamically when we learn the master_account
            if(!g_sync_topic_subscribed && ArraySize(g_configs) > 0)
            {
               SubscribeToSyncTopic();
            }
         
            // Mark config as requested in EaState so we don't spam requests
            ea_state_mark_config_requested(g_ea_state);

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
         } // end else (config/ topic)
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
   uchar trade_buffer[];
   ArrayResize(trade_buffer, MESSAGE_BUFFER_SIZE);
   int trade_bytes = zmq_socket_receive(g_zmq_trade_socket, trade_buffer, MESSAGE_BUFFER_SIZE);

   if(trade_bytes > 0)
   {
      // PUB/SUB format: topic(master_account) + space + MessagePack payload
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

         // Trade socket only handles TradeSignal messages
         // PositionSnapshot is received via config socket in OnTimer
         ProcessTradeSignal(msgpack_payload, payload_len);
      }
   }
}

//+------------------------------------------------------------------+
//| Expert tick function                                              |
//+------------------------------------------------------------------+
void OnTick()
{
   if(!g_initialized) return;

   // Process trade signals (also called from OnTimer for polling)
   g_received_via_timer = false;  // Mark that signal will be received via OnTick
   ProcessTradeSignals();

   // Flush VictoriaLogs periodically
   VLogsFlushIfNeeded();
}

//+------------------------------------------------------------------+
//| Trade transaction event handler                                   |
//| Detects when pending orders are filled and updates mappings       |
//+------------------------------------------------------------------+
void OnTradeTransaction(const MqlTradeTransaction &trans,
                        const MqlTradeRequest &request,
                        const MqlTradeResult &result)
{
   // Only process deal additions (order fills)
   if(trans.type != TRADE_TRANSACTION_DEAL_ADD)
      return;

   // Get deal info
   ulong deal_ticket = trans.deal;
   if(deal_ticket == 0)
      return;

   // Get the order that created this deal
   ulong order_ticket = HistoryDealGetInteger(deal_ticket, DEAL_ORDER);
   if(order_ticket == 0)
      return;

   // Check if this order is in our pending order map
   ulong master_ticket = GetMasterTicketFromPendingMapping(g_pending_order_map, order_ticket);
   if(master_ticket == 0)
      return;  // Not our pending order

   // Get the resulting position ticket
   ulong position_ticket = HistoryDealGetInteger(deal_ticket, DEAL_POSITION_ID);
   if(position_ticket == 0)
   {
      Print("WARNING: Deal ", deal_ticket, " has no position ID");
      return;
   }

   // Move mapping from pending to active
   RemovePendingTicketMapping(g_pending_order_map, master_ticket);
   AddTicketMapping(g_order_map, master_ticket, position_ticket);

   Print("[PENDING FILL] Order #", order_ticket, " -> Position #", position_ticket, " (master:#", master_ticket, ")");
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
   string timestamp = trade_signal_get_string(handle, "timestamp");
   string source_account = trade_signal_get_string(handle, "source_account");

   // Log trade signal receipt with key details for traceability
   Print("[SIGNAL] ", action, " master:#", master_ticket, " ", symbol, " ", order_type_str, " ", lots, " lots @ ", price, " from ", source_account);

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

      // Transform lot size based on calculation mode
      double transformed_lots = TransformLotSize(lots, g_configs[config_index], transformed_symbol);
      string transformed_order_type = ReverseOrderType(order_type_str, g_configs[config_index].reverse_trade);

      // Open position with transformed values (pass config settings)
      ExecuteOpenTrade(g_trade, g_order_map, g_pending_order_map, master_ticket, transformed_symbol,
                       transformed_order_type, transformed_lots, price, sl, tp, timestamp, source_account,
                       magic_number, trade_slippage, max_signal_delay, use_pending_for_delayed, max_retries, DEFAULT_SLIPPAGE);
   }
   else if(action == "Close")
   {
      // Close trades are always allowed (to close existing positions)
      double close_ratio = trade_signal_get_double(handle, "close_ratio");
      ExecuteCloseTrade(g_trade, g_order_map, master_ticket, close_ratio, trade_slippage, DEFAULT_SLIPPAGE);
      ExecuteCancelPendingOrder(g_trade, g_pending_order_map, master_ticket);
   }
   else if(action == "Modify")
   {
      ExecuteModifyTrade(g_trade, g_order_map, master_ticket, sl, tp);
   }

   // Free the handle
   trade_signal_free(handle);
}

//+------------------------------------------------------------------+
//| Subscribe to sync/{master}/{slave} topic for PositionSnapshot     |
//| Called after receiving first config to subscribe to sync topic    |
//+------------------------------------------------------------------+
void SubscribeToSyncTopic()
{
   if(g_sync_topic_subscribed || ArraySize(g_configs) == 0)
      return;

   // Get master account from first config
   string master_account = g_configs[0].master_account;
   if(master_account == "")
      return;

   // Build sync topic: sync/{master}/{slave}
   ushort master_utf16[256];
   ushort slave_utf16[256];
   ushort sync_topic_buffer[256];
   
   StringToShortArray(master_account, master_utf16);
   StringToShortArray(AccountID, slave_utf16);
   
   int sync_len = build_sync_topic_ffi(master_utf16, slave_utf16, sync_topic_buffer, 256);
   if(sync_len > 0)
   {
      g_sync_topic = ShortArrayToString(sync_topic_buffer);
      Print("Generated sync topic: ", g_sync_topic);
      
      if(SubscribeToTopic(g_zmq_config_socket, g_sync_topic))
      {
         Print("Subscribed to sync topic: ", g_sync_topic);
         g_sync_topic_subscribed = true;
      }
      else
      {
         Print("WARNING: Failed to subscribe to sync topic: ", g_sync_topic);
      }
   }
   else
   {
      Print("WARNING: Failed to generate sync topic");
   }
}

//+------------------------------------------------------------------+
//| Process position snapshot for sync (MT5)                          |
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

   // Check sync_mode - should not be SKIP (shouldn't receive snapshot if SKIP)
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
   if(sync_mode == SYNC_MODE_LIMIT_ORDER)
      Print("Limit order expiry: ", limit_order_expiry, " min (0=GTC)");
   else
      Print("Market sync max pips: ", market_sync_max_pips);

   // Get position count
   int position_count = position_snapshot_get_positions_count(handle);
   Print("Positions to sync: ", position_count);

   int synced_count = 0;
   int skipped_count = 0;

   // Process each position
   for(int i = 0; i < position_count; i++)
   {
      // Extract position data
      long master_ticket = position_snapshot_get_position_int(handle, i, "ticket");
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
      if(GetSlaveTicketFromMapping(g_order_map, (ulong)master_ticket) > 0)
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
         // Place limit order at master's open price
         SyncWithLimitOrder(g_trade, g_pending_order_map, (ulong)master_ticket, transformed_symbol,
                            transformed_order_type, transformed_lots, open_price, sl, tp,
                            source_account, magic_number, limit_order_expiry);
         synced_count++;
      }
      else if(sync_mode == SYNC_MODE_MARKET_ORDER)
      {
         // Execute market order if within price deviation
         if(SyncWithMarketOrder(g_trade, g_order_map, (ulong)master_ticket, transformed_symbol,
                                transformed_order_type, transformed_lots, open_price, sl, tp,
                                source_account, magic_number, trade_slippage, market_sync_max_pips, DEFAULT_SLIPPAGE))
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
