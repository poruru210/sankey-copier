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
// ZMQ.mqh removed
#include "../Include/SankeyCopier/Mapping.mqh"
#include "../Include/SankeyCopier/GridPanel.mqh"
//--- Include common headers (Messages.mqh removed - using high-level FFI)
#include "../Include/SankeyCopier/Trade.mqh"
#include "../Include/SankeyCopier/SlaveTrade.mqh"
// MessageParsing.mqh removed
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
// ZMQ globals removed - managed by EaContext
CTrade      g_trade;
// g_order_map/g_pending_order_map removed - managed by Rust EaContext
// g_local_mappings removed - all symbol transformation now handled by Relay Server
bool        g_initialized = false;
datetime    g_last_heartbeat = 0;
bool        g_config_requested = false; // Track if config has been requested
bool        g_last_trade_allowed = false; // Track auto-trading state for change detection
bool        g_register_sent = false;    // Track if register message has been sent
EaContextWrapper g_ea_context;        // Rust EA Context wrapper
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

   // Initialize EaContext (handles ZMQ internally)
   if(!g_ea_context.Initialize(AccountID, EA_TYPE_SLAVE, "MT5", GetAccountNumber(), 
                               AccountInfoString(ACCOUNT_COMPANY), AccountInfoString(ACCOUNT_NAME),
                               AccountInfoString(ACCOUNT_SERVER), AccountInfoString(ACCOUNT_CURRENCY),
                               AccountInfoInteger(ACCOUNT_LEVERAGE)))
   {
      Print("Failed to initialize EaContext");
      return INIT_FAILED;
   }

   // Connect to Relay Server
   if(!g_ea_context.Connect(g_RelayAddress, g_SubAddress))
   {
      Print("Failed to connect to Relay Server");
      return INIT_FAILED;
   }
   
   // Subscribe to global sync and my config topics
   if(!g_ea_context.SubscribeConfig(g_config_topic))
   {
       Print("Failed to subscribe to config topic: ", g_config_topic);
   }
   if(!g_ea_context.SubscribeConfig(g_vlogs_topic))
   {
       Print("Failed to subscribe to vlogs topic: ", g_vlogs_topic);
   }

   Print("EaContext initialized and connected");
   g_initialized = true;

   g_trade.SetExpertMagicNumber(0);
   g_trade.SetDeviationInPoints(DEFAULT_SLIPPAGE);  // Will be updated per-trade from config
   g_trade.SetTypeFilling(ORDER_FILLING_IOC);

   // Recover ticket mappings from existing positions (restart recovery)
   // We iterate local positions and tell Rust about them via FFI
   RecoverMappingsToRust(g_ea_context);

   // Initialize configuration arrays
   ArrayResize(g_configs, 0);

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
               // Process snapshot logic is now in Rust
               // Calling ProcessSnapshot triggers logic in Rust which queues new CMD_OPEN commands
               g_ea_context.ProcessSnapshot();
               break;
           }

           case CMD_UPDATE_UI:
           {
               HANDLE_TYPE config_handle = g_ea_context.GetSlaveConfig();
               if(config_handle != 0)
               {
                   // Process config handle
                   ProcessConfigMessageFromHandle(config_handle, g_configs, g_ea_context, AccountID);
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
   ulong master_ticket = (ulong)g_ea_context.GetMasterTicketFromPending((long)order_ticket);
   if(master_ticket == 0)
      return;  // Not our pending order

   // Get the resulting position ticket
   ulong position_ticket = HistoryDealGetInteger(deal_ticket, DEAL_POSITION_ID);
   if(position_ticket == 0)
   {
      Print("WARNING: Deal ", deal_ticket, " has no position ID");
      return;
   }

   // Move mapping from pending to active (notify Rust)
   g_ea_context.ReportPendingFill((long)master_ticket, (long)position_ticket);

   Print("[PENDING FILL] Order #", order_ticket, " -> Position #", position_ticket, " (master:#", master_ticket, ")");
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
      // Reuse close_ratio for market_sync_max_pips (if sync)
      double max_pips_deviation = cmd.close_ratio; // Set by Rust if SyncMode=MarketOrder

      ExecuteOpenTrade(g_trade, g_ea_context, master_ticket, symbol,
                       order_type_str, cmd.volume, cmd.price, cmd.sl, cmd.tp, timestamp_iso, source_account,
                       (int)cmd.magic, trade_slippage, max_signal_delay, use_pending_for_delayed, max_retries, DEFAULT_SLIPPAGE,
                       cmd.expiration, max_pips_deviation); // Pass expiration and close_ratio (deviation)
   }
   // CMD_CLOSE
   else if(action == CMD_CLOSE)
   {
      // Using close_ratio from command
      ExecuteCloseTrade(g_trade, g_ea_context, master_ticket, cmd.close_ratio, trade_slippage, DEFAULT_SLIPPAGE);
      ExecuteCancelPendingOrder(g_trade, g_ea_context, master_ticket);
   }
   // CMD_MODIFY
   else if(action == CMD_MODIFY)
   {
      ExecuteModifyTrade(g_trade, g_ea_context, master_ticket, cmd.sl, cmd.tp);
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
      Print("Generated sync topic: ", g_sync_topic);
      
      if(g_ea_context.SubscribeConfig(g_sync_topic))
      {
         Print("Subscribed to sync topic: ", g_sync_topic);
         // g_sync_topic_subscribed = true; // Removed global bool usage
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
//| Recover ticket mappings from existing positions (to Rust)         |
//+------------------------------------------------------------------+
void RecoverMappingsToRust(EaContextWrapper &context)
{
   int recovered_count = 0;
   LogInfo(CAT_SYNC, "Recovering ticket mappings from existing positions");

   // Scan all open positions
   int pos_total = PositionsTotal();
   for(int i = 0; i < pos_total; i++)
   {
      ulong ticket = PositionGetTicket(i);
      if(ticket == 0) continue;

      if(!PositionSelectByTicket(ticket)) continue;

      string comment = PositionGetString(POSITION_COMMENT);
      bool is_pending = false;
      ulong master_ticket = ParseMasterTicketFromComment(comment, is_pending);

      if(master_ticket > 0 && !is_pending)
      {
         // This is a position we copied from master
         context.AddMapping((long)master_ticket, (long)ticket, false);
         recovered_count++;
         LogDebug(CAT_SYNC, StringFormat("Recovered mapping: master #%d -> slave #%d (comment: %s)", master_ticket, ticket, comment));
      }
   }

   // Scan all pending orders
   int order_total = OrdersTotal();
   for(int i = 0; i < order_total; i++)
   {
      ulong ticket = OrderGetTicket(i);
      if(ticket == 0) continue;

      if(!OrderSelect(ticket)) continue;

      string comment = OrderGetString(ORDER_COMMENT);
      bool is_pending = false;
      ulong master_ticket = ParseMasterTicketFromComment(comment, is_pending);

      if(master_ticket > 0 && is_pending)
      {
         // This is a pending order we created for delayed signal
         context.AddMapping((long)master_ticket, (long)ticket, true);
         recovered_count++;
         LogDebug(CAT_SYNC, StringFormat("Recovered pending mapping: master #%d -> pending #%d (comment: %s)", master_ticket, ticket, comment));
      }
   }

   LogInfo(CAT_SYNC, StringFormat("Recovery complete: %d mappings restored", recovered_count));
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
