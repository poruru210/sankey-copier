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
#include "../Include/SankeyCopier/SlaveContext.mqh"
// ZMQ.mqh removed
#include "../Include/SankeyCopier/Mapping.mqh"
#include "../Include/SankeyCopier/GridPanel.mqh"
//--- Include common headers (Messages.mqh removed - using high-level FFI)
#include "../Include/SankeyCopier/SlaveConfig.mqh"
#include "../Include/SankeyCopier/SlaveTrade.mqh"
// MessageParsing.mqh removed
#include "../Include/SankeyCopier/Logging.mqh"
#include "../Include/SankeyCopier/GlobalConfig.mqh"
#include "../Include/SankeyCopier/SymbolUtils.mqh"

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
// ZMQ globals removed - managed by EaContext
TicketMapping g_order_map[];
PendingTicketMapping g_pending_order_map[];
bool        g_initialized = false;
datetime    g_last_heartbeat = 0;
bool        g_config_requested = false;   // Track if config request has been sent
bool        g_last_trade_allowed = false; // Track auto-trading state for change detection
bool        g_register_sent = false;    // Track if register message has been sent
SlaveContextWrapper g_ea_context;        // Rust EA Context wrapper


//--- Extended configuration variables (from ConfigMessage)
CopyConfig     g_configs[];                      // Array of active configurations
bool           g_has_received_config = false;    // Track if we have received at least one config

//--- Topic strings (generated via FFI)
string g_config_topic = "";
string g_vlogs_topic = "";
bool   g_sync_topic_subscribed = false;  // Track if sync topic has been subscribed
string g_sync_topic = "";                // Sync topic for receiving PositionSnapshot

//--- Configuration panel
CGridPanel     g_config_panel;

int g_last_config_count = 0;



//+------------------------------------------------------------------+
//| Expert initialization function                                     |
//+------------------------------------------------------------------+
int OnInit()
{
   LogInfo(CAT_SYSTEM, "=== SankeyCopier Slave EA (MT4) Starting ===");

   // Auto-generate AccountID from broker name and account number
   AccountID = GenerateAccountID();
   LogInfo(CAT_SYSTEM, "Auto-generated AccountID: " + AccountID);

   // Generate topics using FFI
   ushort topic_buffer[256];
   int len = build_config_topic(AccountID, topic_buffer, 256);
   if(len > 0) 
   {
      g_config_topic = ShortArrayToString(topic_buffer);
   }
   else
   {
      LogError(CAT_SYSTEM, "Failed to build config topic");
      return INIT_FAILED;
   }

   len = get_global_config_topic(topic_buffer, 256);
   if(len > 0)
   {
      g_vlogs_topic = ShortArrayToString(topic_buffer);
   }
   else
   {
      LogError(CAT_SYSTEM, "Failed to build global config topic");
      return INIT_FAILED;
   }

   LogInfo(CAT_CONFIG, "Generated topics: Config=" + g_config_topic + ", VLogs=" + g_vlogs_topic);

   // Resolve addresses from sankey_copier.ini config file
   // 2-port architecture: PUSH (EA->Server) and SUB (Server->EA, unified)
   g_RelayAddress = GetPushAddress();
   g_TradeAddress = GetTradeSubAddress();

   LogInfo(CAT_CONFIG, "Resolved addresses: PUSH=" + g_RelayAddress + ", SUB=" + g_TradeAddress + " (unified)");

   // Initialize EaContext (handles ZMQ internally)
   if(!g_ea_context.Initialize(AccountID, EA_TYPE_SLAVE, "MT4", GetAccountNumber(), 
                               AccountInfoString(ACCOUNT_COMPANY), AccountInfoString(ACCOUNT_NAME),
                               AccountInfoString(ACCOUNT_SERVER), AccountInfoString(ACCOUNT_CURRENCY),
                               AccountInfoInteger(ACCOUNT_LEVERAGE)))
   {
      LogError(CAT_SYSTEM, "Failed to initialize EaContext");
      return INIT_FAILED;
   }
   
   // Connect to Relay Server
   if(!g_ea_context.Connect(g_RelayAddress, g_TradeAddress))
   {
      LogError(CAT_SYSTEM, "Failed to connect to Relay Server");
      return INIT_FAILED;
   }

   // Detect symbols and send explicit Register message
   // This ensures the server receives the detected symbols immediately
   // Detect symbols and send explicit Register message
   // This ensures the server receives the detected symbols immediately
   // Detect symbols locally in MQL (Phase 2 implemented)
   string detected_prefix = "";
   string detected_suffix = "";
   string detected_specials = "";
   
   DetectSymbolContext(detected_prefix, detected_suffix, detected_specials);
   
   LogInfo(CAT_SYSTEM, StringFormat("Detected Context: Prefix='%s', Suffix='%s', Specials='%s'", 
           detected_prefix, detected_suffix, detected_specials));
   
   // Check IsTradeAllowed() for MT4
   bool is_trade_allowed = IsTradeAllowed();
   if(g_ea_context.SendRegister(detected_prefix, detected_suffix, detected_specials, is_trade_allowed))
   {
       g_register_sent = true;
       LogInfo(CAT_SYSTEM, "Sent initial Register message");
   }
   else
   {
       LogWarn(CAT_SYSTEM, "Failed to send initial Register message");
   }

   // Subscribe to global sync and my config topics
   if(!g_ea_context.SubscribeConfig(g_config_topic))
   {
       LogWarn(CAT_SYSTEM, "Failed to subscribe to config topic: " + g_config_topic);
   }

   // Subscribe to VictoriaLogs config (global broadcast)
   if(!g_ea_context.SubscribeConfig(g_vlogs_topic))
   {
      LogWarn(CAT_SYSTEM, "Failed to subscribe to vlogs_config topic");
   }

   // Recover ticket mappings from existing positions (restart recovery)
   int recovered = RecoverMappingsFromPositions(g_order_map, g_pending_order_map);
   if(recovered > 0)
   {
      LogInfo(CAT_SYSTEM, "Recovered " + IntegerToString(recovered) + " position mappings from previous session");
   }
   else
   {
      LogInfo(CAT_SYSTEM, "No previous position mappings to recover (fresh start)");
   }

   // Initialize configuration arrays
   ArrayResize(g_configs, 0);

   g_initialized = true;

   // Set up timer for signal polling (MT4: seconds only, minimum 1 second)
   // Also handles heartbeat (every HEARTBEAT_INTERVAL_SECONDS) and config messages
   int interval_sec = MathMax(1, SignalPollingIntervalMs / 1000);
   EventSetTimer(interval_sec);
   LogInfo(CAT_SYSTEM, "Signal polling interval: " + IntegerToString(interval_sec) + " second(s)");

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

   LogInfo(CAT_SYSTEM, "=== SankeyCopier Slave EA Initialized ===");

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
   LogInfo(CAT_SYSTEM, "=== SankeyCopier Slave EA (MT4) Stopping ===");

   // Flush VictoriaLogs before shutdown
   VLogsFlush();

   // Send unregister message to server
   if(g_ea_context.IsInitialized())
   {
      g_ea_context.SendUnregister();
   }

   // Kill timer
   EventKillTimer();

   // Delete configuration panel
   if(ShowConfigPanel)
      g_config_panel.Delete();

   // Cleanup EaContext (handles ZMQ context destruction)
   // No explicit cleanup needed for EaContextWrapper as destructor handles it
   // But we can call Reset if needed
   g_ea_context.Reset();

   // Cleanup EA Context handled by wrapper destructor
   // ea_context_free is called by ~EaContextWrapper


   LogInfo(CAT_SYSTEM, "=== SankeyCopier Slave EA (MT4) Stopped ===");
}

//+------------------------------------------------------------------+
//| Timer function (called at SignalPollingIntervalMs interval)       |
//| Handles: signal polling, heartbeat, config messages               |
//+------------------------------------------------------------------+
void OnTimer()
{
   if(!g_initialized) return;

   // 1. Run ManagerTick (Handles ZMQ Polling, Heartbeats internally)
   bool current_trade_allowed = (bool)TerminalInfoInteger(TERMINAL_TRADE_ALLOWED);
   
   int pending_commands = g_ea_context.ManagerTick(
       GetAccountBalance(), 
       GetAccountEquity(), 
       GetOpenPositionsCount(), 
       current_trade_allowed
   );

   // 2. Process all pending commands
   EaCommand cmd;
   int processed_count = 0;
   
   while(pending_commands > 0 || processed_count < 100)
   {
       // MQL4: GetCommand returns bool
       if(!g_ea_context.GetCommand(cmd)) break;
       processed_count++;

       switch(cmd.command_type)
       {
           case CMD_OPEN:
           case CMD_CLOSE:
           case CMD_MODIFY:
           case CMD_DELETE:
               ProcessTradeSignalFromCommand(cmd);
               break;

           case CMD_PROCESS_SNAPSHOT:
           {
               SPositionInfo positions[];
                if(g_ea_context.GetPositionSnapshot(positions))
                {
                    ProcessPositionSnapshot(positions);
                }
               break;
           }

           case CMD_UPDATE_UI:
           {
               SSlaveConfig config;
                if(g_ea_context.GetSlaveConfig(config))
                {
                    ProcessSlaveConfig(config, g_configs, g_ea_context, AccountID);
                   g_has_received_config = true;
                   
                   // Subscribe to sync/{master}/{slave} topic after receiving config
                   if(!g_sync_topic_subscribed && ArraySize(g_configs) > 0)
                   {
                      SubscribeToSyncTopic();
                   }
                   
                   g_ea_context.MarkConfigRequested();
               }
               
               if(ArraySize(g_configs) != g_last_config_count)
               {
                   g_last_config_count = ArraySize(g_configs);
               }

               // Update configuration panel
               if(ShowConfigPanel)
               {
                   if(!current_trade_allowed)
                   {
                      g_config_panel.UpdateStatusRow(STATUS_DISABLED);
                   }
                   else
                   {
                      g_config_panel.UpdatePanelStatusFromConfigs(g_configs);
                   }
                   g_config_panel.UpdateCarouselConfigs(g_configs);
                   ChartRedraw();
               }
               break;
           }
       }
       
       pending_commands--; // Decrement estimation
   }
   
   VLogsFlushIfNeeded();
}

//+------------------------------------------------------------------+
//| Process trade signals from ZeroMQ socket                          |
//| Called from both OnTick() and OnTimer() for low-latency reception |
//+------------------------------------------------------------------+
// ProcessTradeSignals removed - managed by EaContext.ManagerTick

//+------------------------------------------------------------------+
//| Expert tick function                                              |
//+------------------------------------------------------------------+
void OnTick()
{
   if(!g_initialized)
      return;

   // Check if any pending orders have been filled
   CheckPendingOrderFills(g_pending_order_map, g_order_map);

   // Trade signals are now processed via ManagerTick loop in OnTimer
   // ProcessTradeSignals call removed

   // Flush VictoriaLogs periodically
   VLogsFlushIfNeeded();
}

//+------------------------------------------------------------------+
//| Process incoming trade signal                                     |
//+------------------------------------------------------------------+
//+------------------------------------------------------------------+
//| Process trade signal from EaCommand                              |
//+------------------------------------------------------------------+
void ProcessTradeSignalFromCommand(EaCommand &cmd)
{
   // Extract fields from EaCommand
   ulong master_ticket = (ulong)cmd.ticket;
   // Symbol is fixed size uchar array
   string symbol = CharArrayToString(cmd.symbol);
   // Source account is in source_account field (mapped in Rust)
   string source_account = CharArrayToString(cmd.source_account);
   
   // OrderType from Rust (enum i32) -> String
   string order_type_str = GetOrderTypeString(cmd.order_type);
   
   // string timestamp_iso = TimeToString(cmd.timestamp, TIME_DATE|TIME_SECONDS); // Removed
   
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
      LogWarn(CAT_TRADE, "Trade signal rejected: No active configuration for master " + source_account);
      return;
   }

   // Get trade settings from config (use defaults as fallback)
   int trade_slippage = (g_configs[config_index].max_slippage > 0) ? g_configs[config_index].max_slippage : DEFAULT_SLIPPAGE;
   int max_retries = (g_configs[config_index].max_retries > 0) ? g_configs[config_index].max_retries : DEFAULT_MAX_RETRIES;
   int max_signal_delay = (g_configs[config_index].max_signal_delay_ms > 0) ? g_configs[config_index].max_signal_delay_ms : DEFAULT_MAX_SIGNAL_DELAY_MS;
   bool use_pending_for_delayed = g_configs[config_index].use_pending_order_for_delayed;
   bool allow_new_orders = g_configs[config_index].allow_new_orders;
   
   int action = cmd.command_type;

   // CMD_OPEN
   if(action == CMD_OPEN)
   {
      if(!allow_new_orders)
      {
         LogInfo(CAT_TRADE, "Open signal rejected: allow_new_orders=false (status=" + IntegerToString(g_configs[config_index].status) + ") for master #" + IntegerToString(master_ticket));
         return;
      }

      // Symbol is already transformed by Relay Server
      string transformed_symbol = symbol;
      
      // Transform lot size
      double transformed_lots = TransformLotSize(cmd.volume, g_configs[config_index], transformed_symbol);
      string transformed_order_type = ReverseOrderType(order_type_str, g_configs[config_index].reverse_trade);
      
      // Open position (MT4: no CTrade object passed)
      ExecuteOpenTrade(g_order_map, g_pending_order_map, master_ticket, transformed_symbol,
                       transformed_order_type, transformed_lots, cmd.price, cmd.sl, cmd.tp, cmd.algo_flags, source_account, // Replaced cmd.timestamp with cmd.algo_flags
                       (int)cmd.magic, trade_slippage, use_pending_for_delayed, max_retries, DEFAULT_SLIPPAGE);
   }
   // CMD_CLOSE
   else if(action == CMD_CLOSE)
   {
      // Using close_ratio from command
      ExecuteCloseTrade(g_order_map, master_ticket, cmd.close_ratio, trade_slippage, DEFAULT_SLIPPAGE);
      ExecuteCancelPendingOrder(g_pending_order_map, master_ticket);
   }
   // CMD_MODIFY
   else if(action == CMD_MODIFY)
   {
      ExecuteModifyTrade(g_order_map, master_ticket, cmd.sl, cmd.tp);
   }
}

//+------------------------------------------------------------------+
//| Subscribe to sync/{master}/{slave} topic for PositionSnapshot     |
//| Called after receiving first config to subscribe to sync topic    |
//+------------------------------------------------------------------+
void SubscribeToSyncTopic()
{
   if(ArraySize(g_configs) == 0)
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
      LogInfo(CAT_SYSTEM, "Generated sync topic: " + g_sync_topic);
      
      if(g_ea_context.SubscribeConfig(g_sync_topic))
      {
         LogInfo(CAT_SYSTEM, "Subscribed to sync topic: " + g_sync_topic);
         // g_sync_topic_subscribed = true;
      }
      else
      {
         LogWarn(CAT_SYSTEM, "Failed to subscribe to sync topic: " + g_sync_topic);
      }
   }
   else
   {
      LogWarn(CAT_SYSTEM, "Failed to generate sync topic");
   }
}

//+------------------------------------------------------------------+
//| Process position snapshot for sync (MT4)                          |
//| Called when Slave receives PositionSnapshot from Master           |
//+------------------------------------------------------------------+
//+------------------------------------------------------------------+
//| Process position snapshot for sync (MT4)                          |
//| Called when Slave receives PositionSnapshot from Master           |
//+------------------------------------------------------------------+
void ProcessPositionSnapshot(SPositionInfo &positions[])
{
   LogInfo(CAT_SYNC, "=== Processing Position Snapshot ===");

   string source_account = g_ea_context.GetPositionSnapshotSourceAccount();
   if(source_account == "")
   {
      LogError(CAT_SYNC, "PositionSnapshot has empty source_account");
      return;
   }

   LogInfo(CAT_SYNC, "PositionSnapshot from master: " + source_account);

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
      LogWarn(CAT_SYNC, "PositionSnapshot rejected: No config for master " + source_account);
      return;
   }

   int sync_mode = g_configs[config_index].sync_mode;
   if(sync_mode == SYNC_MODE_SKIP)
   {
      LogInfo(CAT_SYNC, "PositionSnapshot ignored: sync_mode is SKIP");
      return;
   }

   int limit_order_expiry = g_configs[config_index].limit_order_expiry_min;
   double market_sync_max_pips = g_configs[config_index].market_sync_max_pips;
   int trade_slippage = (g_configs[config_index].max_slippage > 0) ? g_configs[config_index].max_slippage : DEFAULT_SLIPPAGE;

   int position_count = ArraySize(positions);
   LogInfo(CAT_SYNC, "Positions to sync: " + IntegerToString(position_count));

   int synced_count = 0;
   int skipped_count = 0;

   for(int i = 0; i < position_count; i++)
   {
      int master_ticket = (int)positions[i].ticket;
      string symbol = CharArrayToString(positions[i].symbol);
      int order_type_int = positions[i].order_type;
      string order_type_str = GetOrderTypeString(order_type_int);
      double lots = positions[i].lots;
      double open_price = positions[i].open_price;
      double sl = positions[i].stop_loss;
      double tp = positions[i].take_profit;
      int magic_number = (int)positions[i].magic_number;

      LogInfo(CAT_SYNC, StringFormat("Position %d/%d: #%d %s %s %.2f lots @ %.5f", i + 1, position_count, master_ticket, symbol, order_type_str, lots, open_price));

      if(GetSlaveTicketFromMapping(g_order_map, master_ticket) > 0)
      {
         LogInfo(CAT_SYNC, "  -> Already mapped, skipping");
         skipped_count++;
         continue;
      }

      double transformed_lots = TransformLotSize(lots, g_configs[config_index], symbol);
      string transformed_order_type = ReverseOrderType(order_type_str, g_configs[config_index].reverse_trade);

      if(sync_mode == SYNC_MODE_LIMIT_ORDER)
      {
         SyncWithLimitOrder(g_pending_order_map, master_ticket, symbol, transformed_order_type, transformed_lots, open_price, sl, tp, source_account, magic_number, limit_order_expiry);
         synced_count++;
      }
      else if(sync_mode == SYNC_MODE_MARKET_ORDER)
      {
         if(SyncWithMarketOrder(g_order_map, master_ticket, symbol, transformed_order_type, transformed_lots, open_price, sl, tp, source_account, magic_number, trade_slippage, market_sync_max_pips, DEFAULT_SLIPPAGE))
            synced_count++;
         else
         {
            LogWarn(CAT_SYNC, "  -> Price deviation too large, skipped");
            skipped_count++;
         }
      }
   }

   LogInfo(CAT_SYNC, StringFormat("=== Position Sync Complete: %d synced, %d skipped ===", synced_count, skipped_count));
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