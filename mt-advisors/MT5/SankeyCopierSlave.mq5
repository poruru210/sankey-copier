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
// ZMQ globals removed - managed by EaContext
CTrade      g_trade;
TicketMapping g_order_map[];
PendingTicketMapping g_pending_order_map[];
// g_local_mappings removed - all symbol transformation now handled by Relay Server
bool        g_initialized = false;
datetime    g_last_heartbeat = 0;
bool        g_config_requested = false; // Track if config has been requested
bool        g_last_trade_allowed = false; // Track auto-trading state for change detection
bool        g_register_sent = false;    // Track if register message has been sent
SlaveContextWrapper g_ea_context;        // Rust EA Context wrapper
GlobalConfigManager *g_global_config = NULL; // Global config manager
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
   LogInfo(CAT_SYSTEM, "=== SankeyCopier Slave EA (MT5) Starting ===");

   // Auto-generate AccountID from broker name and account number
   AccountID = GenerateAccountID();
   LogInfo(CAT_SYSTEM, "Auto-generated AccountID: " + AccountID);

   // Symbol transformation is now handled by Relay Server
   // Slave EA receives pre-transformed symbols

   // Load port configuration from sankey_copier.ini
   // 2-port architecture: Receiver (PULL) and Publisher (unified PUB for trades + configs)
   if(!LoadConfig())
   {
      LogWarn(CAT_CONFIG, "Failed to load config file, using default ports");
   }
   else
   {
      LogInfo(CAT_CONFIG, "Config loaded: ReceiverPort=" + IntegerToString(GetReceiverPort()) + ", PublisherPort=" + IntegerToString(GetPublisherPort()) + " (unified)");
   }

   // Resolve addresses from sankey_copier.ini config file
   // 2-port architecture: PUSH (EA->Server) and SUB (Server->EA, unified)
   g_RelayAddress = GetPushAddress();
   g_SubAddress = GetTradeSubAddress();

   LogInfo(CAT_CONFIG, "Resolved addresses: PUSH=" + g_RelayAddress + ", SUB=" + g_SubAddress + " (unified)");

   // Initialize topics using FFI
   ushort topic_buffer[256];
   int len = build_config_topic(AccountID, topic_buffer, 256);
   if(len > 0) 
   {
      g_config_topic = ShortArrayToString(topic_buffer);
      LogInfo(CAT_CONFIG, "Generated config topic: " + g_config_topic);
   }
   else 
   {
      g_config_topic = AccountID; // Fallback
      LogWarn(CAT_CONFIG, "Failed to generate config topic, using AccountID fallback: " + g_config_topic);
   }

   len = get_global_config_topic(topic_buffer, 256);
   if(len > 0) 
   {
      g_vlogs_topic = ShortArrayToString(topic_buffer);
      LogInfo(CAT_SYSTEM, "Generated vlogs topic: " + g_vlogs_topic);
   }
   else 
   {
      LogError(CAT_SYSTEM, "Failed to generate vlogs topic from mt-bridge");
      return INIT_FAILED;
   }

   // Initialize EaContext (handles ZMQ internally)
   if(!g_ea_context.Initialize(AccountID, EA_TYPE_SLAVE, "MT5", GetAccountNumber(), 
                               AccountInfoString(ACCOUNT_COMPANY), AccountInfoString(ACCOUNT_NAME),
                               AccountInfoString(ACCOUNT_SERVER), AccountInfoString(ACCOUNT_CURRENCY),
                               AccountInfoInteger(ACCOUNT_LEVERAGE)))
   {
      LogError(CAT_SYSTEM, "Failed to initialize EaContext");
      return INIT_FAILED;
   }

   // Initialize Global Config Manager
   g_global_config = new GlobalConfigManager(&g_ea_context);

   // Connect to Relay Server
   if(!g_ea_context.Connect(g_RelayAddress, g_SubAddress))
   {
      LogError(CAT_SYSTEM, "Failed to connect to Relay Server");
      return INIT_FAILED;
   }
   
   // Subscribe to global sync and my config topics
   if(!g_ea_context.SubscribeConfig(g_config_topic))
   {
       LogWarn(CAT_SYSTEM, "Failed to subscribe to config topic: " + g_config_topic);
   }
   if(!g_ea_context.SubscribeConfig(g_vlogs_topic))
   {
       LogWarn(CAT_SYSTEM, "Failed to subscribe to vlogs topic: " + g_vlogs_topic);
   }

   LogInfo(CAT_SYSTEM, "EaContext initialized and connected");
   g_initialized = true;

   g_trade.SetExpertMagicNumber(0);
   g_trade.SetDeviationInPoints(DEFAULT_SLIPPAGE);  // Will be updated per-trade from config
   g_trade.SetTypeFilling(ORDER_FILLING_IOC);

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

   // Set up millisecond timer for signal polling
   // Also handles heartbeat (every HEARTBEAT_INTERVAL_SECONDS) and config messages
   int interval_ms = MathMax(100, MathMin(5000, SignalPollingIntervalMs));
   EventSetMillisecondTimer(interval_ms);
   LogInfo(CAT_SYSTEM, StringFormat("Signal polling interval: %dms", interval_ms));

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
   if(g_ea_context.IsInitialized())
   {
      g_ea_context.SendUnregister();
   }

   // Cleanup Global Config Manager
   if(g_global_config != NULL)
   {
      delete g_global_config;
      g_global_config = NULL;
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
   
   // 1a. Detect auto-trading state change and update panel immediately
   // This ensures DISABLED/ENABLED status reflects instantly without waiting for CONFIG
   if(ShowConfigPanel && current_trade_allowed != g_last_trade_allowed)
   {
      g_last_trade_allowed = current_trade_allowed;
      if(!current_trade_allowed)
      {
         g_config_panel.UpdateStatusRow(STATUS_DISABLED);
      }
      else if(g_has_received_config)
      {
         g_config_panel.UpdatePanelStatusFromConfigs(g_configs);
      }
      // Else: no config yet, keep current status (NO_CONFIG)
      ChartRedraw();
   }
   
   int pending_commands = g_ea_context.ManagerTick(
       GetAccountBalance(), 
       GetAccountEquity(), 
       GetOpenPositionsCount(), 
       current_trade_allowed
   );

   // 1b. Check for Global Config Updates
   if(g_global_config != NULL) g_global_config.CheckForUpdate();

   // 2. Process all pending commands
   EaCommand cmd;
   int processed_count = 0;
   
   while(pending_commands > 0 || processed_count < 100)
   {
       if(!g_ea_context.GetCommand(cmd)) break;
       processed_count++;

       switch(cmd.command_type)
       {
           case CMD_OPEN:
           case CMD_CLOSE:
           case CMD_MODIFY:
           {
               ProcessTradeSignalFromCommand(cmd);
               break;
           }
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
                   // Process config struct
                   ProcessSlaveConfig(config, g_configs, g_ea_context, AccountID);
                   g_has_received_config = true;
                   
                   // Subscribe to sync/{master}/{slave} topic after receiving config
                   if(!g_sync_topic_subscribed && ArraySize(g_configs) > 0)
                   {
                      SubscribeToSyncTopic();
                   }
                   
                   g_ea_context.MarkConfigRequested();
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
           // ...
       }
   }
   
   VLogsFlushIfNeeded();
}

//+------------------------------------------------------------------+
//| Expert tick function                                              |
//+------------------------------------------------------------------+
void OnTick()
{
   if(!g_initialized) return;

   // Trade signals are now processed via ManagerTick loop in OnTimer primarily
   // But we can also trigger a check here if we want tick-based responsiveness
   // For now, let's rely on high-frequency Timer (100ms) for consistency
   // Or call OnTimer() logic here? 
   // ManagerTick is safe to call frequently.
   
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
      LogWarn(CAT_TRADE, "Deal " + IntegerToString(deal_ticket) + " has no position ID");
      return;
   }

   // Move mapping from pending to active
   RemovePendingTicketMapping(g_pending_order_map, master_ticket);
   AddTicketMapping(g_order_map, master_ticket, position_ticket);

   LogTrade("Pending Fill", (long)order_ticket, "", StringFormat("-> Position #%d (master:#%d)", position_ticket, master_ticket));
}

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
   
   string timestamp_iso = TimeToString(cmd.timestamp, TIME_DATE|TIME_SECONDS); // Simplified
   
   // Find matching config for this master (ONLY to get trade execution settings)
   // Business logic (filters, multipliers) is already applied by mt-bridge
   int config_index = -1;
   for(int i=0; i<ArraySize(g_configs); i++)
   {
      if(g_configs[i].master_account == source_account)
      {
         config_index = i;
         break;
      }
   }

   int trade_slippage = DEFAULT_SLIPPAGE;
   int max_retries = DEFAULT_MAX_RETRIES;
   int max_signal_delay = DEFAULT_MAX_SIGNAL_DELAY_MS;
   bool use_pending_for_delayed = false;

   if(config_index != -1)
   {
       if(g_configs[config_index].max_slippage > 0) trade_slippage = g_configs[config_index].max_slippage;
       if(g_configs[config_index].max_retries > 0) max_retries = g_configs[config_index].max_retries;
       if(g_configs[config_index].max_signal_delay_ms > 0) max_signal_delay = g_configs[config_index].max_signal_delay_ms;
       use_pending_for_delayed = g_configs[config_index].use_pending_order_for_delayed;
   }

   int action = cmd.command_type; // CMD_OPEN, CMD_CLOSE, etc.

   // CMD_OPEN
   if(action == CMD_OPEN)
   {
      // Open position using pre-calculated values from Rust
      // Note: order_type_str is already reversed if needed
      // Note: cmd.volume is already transformed
      // Note: symbol is passed as is (usually pre-transformed by Relay, or local config)
      
      // Execute Open Trade
      ExecuteOpenTrade(g_trade, g_order_map, g_pending_order_map, master_ticket, symbol,
                       order_type_str, cmd.volume, cmd.price, cmd.sl, cmd.tp, timestamp_iso, source_account,
                       (int)cmd.magic, trade_slippage, max_signal_delay, use_pending_for_delayed, max_retries, DEFAULT_SLIPPAGE);
   }
   // CMD_CLOSE
   else if(action == CMD_CLOSE)
   {
      // Using close_ratio from command
      ExecuteCloseTrade(g_trade, g_order_map, master_ticket, cmd.close_ratio, trade_slippage, DEFAULT_SLIPPAGE);
      ExecuteCancelPendingOrder(g_trade, g_pending_order_map, master_ticket);
   }
   // CMD_MODIFY
   else if(action == CMD_MODIFY)
   {
      ExecuteModifyTrade(g_trade, g_order_map, master_ticket, cmd.sl, cmd.tp);
   }
}

//+------------------------------------------------------------------+
//| Subscribe to sync/{master}/{slave} topic for PositionSnapshot     |
//| Called after receiving first config to subscribe to sync topic    |
//+------------------------------------------------------------------+
void SubscribeToSyncTopic()
{
   if(ArraySize(g_configs) == 0) // Removed g_sync_topic_subscribed check (optional, or use member if we turn this into class)
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
         // g_sync_topic_subscribed = true; // Removed global bool usage
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
//| Process position snapshot for sync (MT5)                          |
//| Called when Slave receives PositionSnapshot from Master           |
//+------------------------------------------------------------------+
void ProcessPositionSnapshot(SPositionInfo &positions[])
{
   LogInfo(CAT_SYNC, "=== Processing Position Snapshot ===");

   // Get source account (master)
   string source_account = g_ea_context.GetPositionSnapshotSourceAccount();
   if(source_account == "")
   {
      LogError(CAT_SYNC, "PositionSnapshot has empty source_account");
      return;
   }

   LogInfo(CAT_SYNC, "PositionSnapshot from master: " + source_account);

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
      LogWarn(CAT_SYNC, "PositionSnapshot rejected: No config for master " + source_account);
      return;
   }

   // Check sync_mode - should not be SKIP (shouldn't receive snapshot if SKIP)
   int sync_mode = g_configs[config_index].sync_mode;
   if(sync_mode == SYNC_MODE_SKIP)
   {
      LogInfo(CAT_SYNC, "PositionSnapshot ignored: sync_mode is SKIP");
      return;
   }

   // Get sync parameters from config
   int limit_order_expiry = g_configs[config_index].limit_order_expiry_min;
   double market_sync_max_pips = g_configs[config_index].market_sync_max_pips;
   int trade_slippage = (g_configs[config_index].max_slippage > 0)
                        ? g_configs[config_index].max_slippage
                        : DEFAULT_SLIPPAGE;

   LogInfo(CAT_SYNC, "Sync mode: " + ((sync_mode == SYNC_MODE_LIMIT_ORDER) ? "LIMIT_ORDER" : "MARKET_ORDER"));
   if(sync_mode == SYNC_MODE_LIMIT_ORDER)
      LogInfo(CAT_SYNC, "Limit order expiry: " + IntegerToString(limit_order_expiry) + " min (0=GTC)");
   else
      LogInfo(CAT_SYNC, "Market sync max pips: " + DoubleToString(market_sync_max_pips, 1));

   // Get position count
   int position_count = ArraySize(positions);
   LogInfo(CAT_SYNC, "Positions to sync: " + IntegerToString(position_count));

   int synced_count = 0;
   int skipped_count = 0;

   // Process each position
   for(int i = 0; i < position_count; i++)
   {
      // Extract position data
      long master_ticket = positions[i].ticket;
      string symbol = CharArrayToString(positions[i].symbol);

      // Order type: Rust sends string logic for PositionSnapshot?
      // Wait, ea_context_get_position_snapshot converts internal Rust strings to MQL strings if we used the old API.
      // But now we pass a struct.
      // In FFI logic I added earlier for `ea_send_position_snapshot` (Master -> Rust), I converted int to String.
      // Now Slave <- Rust (ea_context_get_position_snapshot), we need to check how FFI implements it.
      // ffi.rs `ea_context_get_position_snapshot` maps logic.
      // It populates `CPositionInfo`.
      // The `CPositionInfo` in Rust `ffi.rs` sets `order_type: i32` by parsing the string stored in Rust `PositionInfo`.
      // So `CPositionInfo.order_type` is `i32` (Enum value).
      // We need to convert this int back to String "Buy"/"Sell" for `TransformLotSize` / `SyncWithLimitOrder`?
      // Or update `SyncWithLimitOrder` to take int?
      // Currently `SyncWithLimitOrder` takes `string type_str`.
      // So we convert int -> string here.

      int order_type_int = positions[i].order_type;
      string order_type_str = GetOrderTypeString(order_type_int); // Common.mqh function

      double lots = positions[i].lots;
      double open_price = positions[i].open_price;
      double sl = positions[i].stop_loss;
      double tp = positions[i].take_profit;
      long magic_long = positions[i].magic_number;
      int magic_number = (int)magic_long;

      LogInfo(CAT_SYNC, StringFormat("Position %d/%d: #%d %s %s %.2f lots @ %.5f", i + 1, position_count, master_ticket, symbol, order_type_str, lots, open_price));

      // Check if we already have this position mapped
      if(GetSlaveTicketFromMapping(g_order_map, (ulong)master_ticket) > 0)
      {
         LogInfo(CAT_SYNC, "  -> Already mapped, skipping");
         skipped_count++;
         continue;
      }

      // Symbol is already transformed by Relay Server (mapping + prefix/suffix applied)
      string transformed_symbol = symbol;

      // NOTE: Position Snapshot sync still calculates lots in MQL because
      // position snapshots are currently passed as raw handles to MQL, not via `EaCommand`.
      // To unify this, `EaContext` should also process PositionSnapshot and emit
      // individual `CMD_OPEN` commands for sync, OR we leave sync logic in MQL for now.
      // Given the task scope, we focused on Trade Signals.
      // Sync logic remains in MQL for now (it uses `TransformLotSize` below).
      // This means we CANNOT remove `TransformLotSize` from `Trade.mqh` yet.

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
