//+------------------------------------------------------------------+
//|                                       SankeyCopierMaster.mq4      |
//|                        Copyright 2025, SANKEY Copier Project      |
//|                                                                  |
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, SANKEY Copier Project"
#property link      ""
#property version   "1.00"  // VERSION_PLACEHOLDER
#property icon      "app.ico"
#property strict

//--- Include common headers
#include "../Include/SankeyCopier/Common.mqh"
// ZMQ.mqh removed
#include "../Include/SankeyCopier/MasterSignals.mqh"
#include "../Include/SankeyCopier/Trade.mqh"
#include "../Include/SankeyCopier/GridPanel.mqh"
#include "../Include/SankeyCopier/Logging.mqh"

//--- Input parameters
// Note: MagicFilter moved to Slave side (allowed_magic_numbers)
// Note: SymbolPrefix/SymbolSuffix moved to Web-UI MasterSettings
// ZMQ addresses are loaded from sankey_copier.ini (no input override)
input int      ScanInterval = 100;              // Scan interval in milliseconds
input bool     ShowConfigPanel = true;          // Show configuration panel on chart
input int      PanelWidth = 280;                // Configuration panel width (pixels)

//--- Resolved addresses (from sankey_copier.ini config file)
string g_RelayAddress = "";
string g_ConfigAddress = "";

//--- Global config variables (populated from Web-UI config)
string g_symbol_prefix = "";       // Symbol prefix from config
string g_symbol_suffix = "";       // Symbol suffix from config
uint   g_config_version = 0;       // Current config version
int    g_server_status = STATUS_NO_CONFIG; // Status from server (DISABLED/CONNECTED)

//--- Topic strings (generated via FFI)
string g_config_topic = "";
string g_vlogs_topic = "";
string g_sync_topic = "";          // Sync topic prefix for receiving SyncRequest (sync/{account_id}/)

//--- Order tracking structure
struct OrderInfo
{
   int    ticket;
   double sl;
   double tp;
   double lots;  // Track volume for partial close detection
};

//--- Global variables
string      AccountID;                  // Auto-generated from broker + account number
// ZMQ globals removed - managed by EaContext
OrderInfo   g_tracked_orders[];
bool        g_initialized = false;
datetime    g_last_heartbeat = 0;
bool        g_last_trade_allowed = false; // Track auto-trading state for change detection
bool        g_config_requested = false;   // Track if config request has been sent
bool        g_register_sent = false;    // Track if register message has been sent
EaContextWrapper g_ea_context;        // Rust EA Context wrapper


//--- Configuration panel
CGridPanel     g_config_panel;

//+------------------------------------------------------------------+
//| Expert initialization function                                     |
//+------------------------------------------------------------------+
int OnInit()
{
   Print("=== SankeyCopier Master EA (MT4) Starting ===");

   // Auto-generate AccountID from broker name and account number
   AccountID = GenerateAccountID();
   Print("Auto-generated AccountID: ", AccountID);

   // Generate topics using FFI
   ushort topic_buffer[256];
   int len = build_config_topic(AccountID, topic_buffer, 256);
   if(len > 0) 
   {
      g_config_topic = ShortArrayToString(topic_buffer);
      // Remove null terminator if present (ShortArrayToString might include it depending on implementation, 
      // but usually it stops at null. Rust FFI returns len without null, but buffer has it.)
      // StringLen check is safer.
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
   g_ConfigAddress = GetConfigSubAddress();

   Print("Resolved addresses: PUSH=", g_RelayAddress, ", SUB=", g_ConfigAddress, " (unified)");

   // Initialize EaContext (handles ZMQ internally)
   if(!g_ea_context.Initialize(AccountID, EA_TYPE_MASTER, "MT4", AccountNumber(), 
                               AccountInfoString(ACCOUNT_COMPANY), AccountInfoString(ACCOUNT_NAME),
                               AccountInfoString(ACCOUNT_SERVER), AccountInfoString(ACCOUNT_CURRENCY),
                               AccountInfoInteger(ACCOUNT_LEVERAGE)))
   {
      Print("Failed to initialize EaContext");
      return INIT_FAILED;
   }
   
   // Connect to Relay Server
   if(!g_ea_context.Connect(g_RelayAddress, g_ConfigAddress))
   {
      Print("Failed to connect to Relay Server");
      return INIT_FAILED;
   }

   // Subscribe to messages for this account ID
   if(!g_ea_context.SubscribeConfig(g_config_topic))
   {
      Print("Failed to subscribe to config topic");
      return INIT_FAILED;
   }

   // Subscribe to VictoriaLogs config (global broadcast)
   if(!g_ea_context.SubscribeConfig(g_vlogs_topic))
   {
      Print("WARNING: Failed to subscribe to vlogs_config topic");
   }

   // Subscribe to sync/{account_id}/ topic for SyncRequest messages from slaves
   ushort sync_topic_buffer[256];
   int sync_len = get_sync_topic_prefix(AccountID, sync_topic_buffer, 256);
   if(sync_len > 0)
   {
      g_sync_topic = ShortArrayToString(sync_topic_buffer);
      Print("Generated sync topic prefix: ", g_sync_topic);
      
      if(!g_ea_context.SubscribeConfig(g_sync_topic))
      {
         Print("WARNING: Failed to subscribe to sync topic");
      }
   }
   else
   {
      Print("WARNING: Failed to generate sync topic prefix");
   }

   // Scan existing orders
   ScanExistingOrders();

   // Set up timer for heartbeat (1 second interval)
   EventSetTimer(1);

   // Initialize configuration panel
   if(ShowConfigPanel)
   {
      g_config_panel.InitializeMasterPanel("SankeyCopierPanel_", PanelWidth);

      // Show NO_CONFIGURATION status initially (no config received yet)
      g_config_panel.UpdateStatusRow(STATUS_NO_CONFIG);

      g_config_panel.UpdateServerRow(g_RelayAddress);
      // MagicFilter removed - now filtered on Slave side via allowed_magic_numbers
      g_config_panel.UpdateMagicFilterRow(0); // Show 0 = All
      g_config_panel.UpdateTrackedOrdersRow(ArraySize(g_tracked_orders));
      g_config_panel.UpdateSymbolConfig(g_symbol_prefix, g_symbol_suffix, "");
   }

   g_initialized = true;
   Print("=== SankeyCopier Master EA (MT4) Initialized ===");

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
   Print("=== SankeyCopier Master EA (MT4) Stopping ===");

   // Flush VictoriaLogs before shutdown
   VLogsFlush();

   // Send unregister message
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


   Print("=== SankeyCopier Master EA (MT4) Stopped ===");
}

//+------------------------------------------------------------------+
//| Timer function (called every 1 second)                            |
//+------------------------------------------------------------------+
void OnTimer()
{
   if(!g_initialized) return;

   // 1. Run ManagerTick (Handles ZMQ Polling, Heartbeats internally)
   bool current_trade_allowed = (bool)TerminalInfoInteger(TERMINAL_TRADE_ALLOWED);
   
   // Track trade state change for UI update
   static bool last_trade_allowed = false;
   bool trade_state_changed = (current_trade_allowed != last_trade_allowed);
   if(trade_state_changed) last_trade_allowed = current_trade_allowed;

   int pending_commands = g_ea_context.ManagerTick(
       GetAccountBalance(), 
       GetAccountEquity(), 
       GetOpenPositionsCount(), 
       current_trade_allowed
   );

   // 2. Process pending commands
   EaCommand cmd;
   int processed_count = 0;
   
   while(pending_commands > 0 || processed_count < 100)
   {
       if(!g_ea_context.GetCommand(cmd)) break;
       processed_count++;

       switch(cmd.command_type)
       {
           case CMD_SEND_SNAPSHOT:
           {
               // Process SyncRequest using handle
               HANDLE_TYPE sync_handle = g_ea_context.GetSyncRequest();
               if(sync_handle != 0)
               {
                   ProcessSyncRequest(sync_handle);
               }
               break;
           }

           case CMD_UPDATE_UI:
           {
               // MasterConfig update
               HANDLE_TYPE config_handle = g_ea_context.GetMasterConfig();
               if(config_handle != 0)
               {
                   ProcessMasterConfigMessage(config_handle);
               }
               break;
           }
       }
       
       pending_commands--;
   }
   
   // Flush VLogs
   VLogsFlushIfNeeded();
   
   // Do NOT free handle - it belongs to EaContext
}

//+------------------------------------------------------------------+
//| Process Master configuration message (from Handle)                |
//+------------------------------------------------------------------+
void ProcessMasterConfigMessage(HANDLE_TYPE config_handle)
{
   if(config_handle == 0 || config_handle == -1)
   {
      Print("ERROR: Invalid Master config handle");
      return;
   }

   // Extract fields from the parsed config using the handle
   string config_account_id = master_config_get_string(config_handle, "account_id");
   string prefix = master_config_get_string(config_handle, "symbol_prefix");
   string suffix = master_config_get_string(config_handle, "symbol_suffix");
   int version = master_config_get_int(config_handle, "config_version");
   int status = master_config_get_int(config_handle, "status");
   g_server_status = status;

   if(config_account_id == "")
   {
      Print("ERROR: Invalid config message received - missing account_id");
      return;
   }

   // Verify this config is for us
   if(config_account_id != AccountID)
   {
      Print("WARNING: Received config for different account: ", config_account_id, " (expected: ", AccountID, ")");
      return;
   }

   // Log configuration update
   Print("[CONFIG] Received: prefix=", (prefix == "" ? "(none)" : prefix),
         " suffix=", (suffix == "" ? "(none)" : suffix), " version=", version);

   // Update global configuration variables
   if(g_server_status == STATUS_NO_CONFIG)
   {
      Print("Status is NO_CONFIG -> Resetting configuration");
      g_server_status = STATUS_NO_CONFIG;
      g_symbol_prefix = "";
      g_symbol_suffix = "";
      g_config_version = 0;
   }
   else
   {
      g_symbol_prefix = prefix;
      g_symbol_suffix = suffix;
      g_config_version = (uint)version;
   }

   // Update configuration panel
   if(ShowConfigPanel)
   {
      g_config_panel.UpdateSymbolConfig(g_symbol_prefix, g_symbol_suffix, "");

      // Use the status received from server directly
      // (server already considers both Web UI state and is_trade_allowed)
      g_config_panel.UpdateStatusRow(g_server_status);

      ChartRedraw();
   }
   
   // Do NOT free handle - it belongs to EaContext
}

//+------------------------------------------------------------------+
//| Process SyncRequest message (from Handle)                         |
//+------------------------------------------------------------------+
void ProcessSyncRequest(HANDLE_TYPE handle)
{
   if(handle == 0 || handle == -1)
   {
      Print("ERROR: Invalid SyncRequest handle");
      return;
   }

   // Get the fields
   string slave_account = sync_request_get_string(handle, "slave_account");
   string master_account = sync_request_get_string(handle, "master_account");

   if(slave_account == "" || master_account == "")
   {
      Print("Invalid SyncRequest received - missing fields");
      return;
   }

   // Check if this request is for us
   if(master_account != AccountID)
   {
      Print("SyncRequest for different master: ", master_account, " (we are: ", AccountID, ")");
      return;
   }

   // Send position snapshot
   if(SendPositionSnapshot(g_ea_context, AccountID, g_symbol_prefix, g_symbol_suffix))
   {
      Print("[SYNC] Position snapshot sent to slave: ", slave_account);
   }
   else
   {
      Print("[ERROR] Failed to send position snapshot to slave: ", slave_account);
   }
   
   // Do NOT free handle - it belongs to EaContext
}

//+------------------------------------------------------------------+
//| Expert tick function                                              |
//+------------------------------------------------------------------+
void OnTick()
{
   if(!g_initialized)
      return;

   // Periodic scan for new orders
   static datetime last_scan = 0;
   if(TimeCurrent() - last_scan > ScanInterval / 1000)
   {
      CheckForNewOrders();
      CheckForModifiedOrders();
      CheckForPartialCloses();
      CheckForClosedOrders();
      last_scan = TimeCurrent();
      
      // Update tracked orders count on panel
      if(ShowConfigPanel)
      {
         g_config_panel.UpdateTrackedOrdersRow(ArraySize(g_tracked_orders));
      }
   }
}

//+------------------------------------------------------------------+
//| Scan existing orders on startup                                   |
//+------------------------------------------------------------------+
void ScanExistingOrders()
{
   ArrayResize(g_tracked_orders, 0);

   // Master detects ALL orders - prefix/suffix is only used for symbol name cleaning
   // Magic number filtering is done on Slave side via allowed_magic_numbers
   for(int i = 0; i < OrdersTotal(); i++)
   {
      if(OrderSelect(i, SELECT_BY_POS, MODE_TRADES))
      {
         int ticket = OrderTicket();
         AddTrackedOrder(ticket);
         SendOpenSignalFromOrder(ticket);  // Send Open signal for existing orders
         Print("[ORDER] Tracking existing: #", ticket, " ", OrderSymbol(), " ", GetOrderTypeString(OrderType()), " ", OrderLots(), " lots");
      }
   }

   Print("Found ", ArraySize(g_tracked_orders), " existing orders");
}

//+------------------------------------------------------------------+
//| Check for new orders                                              |
//+------------------------------------------------------------------+
void CheckForNewOrders()
{
   // Master detects ALL orders - prefix/suffix is only used for symbol name cleaning
   // Magic number filtering is done on Slave side via allowed_magic_numbers
   for(int i = 0; i < OrdersTotal(); i++)
   {
      if(OrderSelect(i, SELECT_BY_POS, MODE_TRADES))
      {
         int ticket = OrderTicket();

         if(!IsOrderTracked(ticket))
         {
            string symbol = OrderSymbol();
            AddTrackedOrder(ticket);
            SendOpenSignalFromOrder(ticket);
            Print("[ORDER] New: #", ticket, " ", symbol, " ", GetOrderTypeString(OrderType()), " ", OrderLots(), " lots @ ", OrderOpenPrice());
         }
      }
   }
}

//+------------------------------------------------------------------+
//| Check for modified orders (SL/TP changes)                         |
//+------------------------------------------------------------------+
void CheckForModifiedOrders()
{
   for(int i = 0; i < ArraySize(g_tracked_orders); i++)
   {
      int ticket = g_tracked_orders[i].ticket;
      if(OrderSelect(ticket, SELECT_BY_TICKET, MODE_TRADES))
      {
         double current_sl = OrderStopLoss();
         double current_tp = OrderTakeProfit();

         // Check if SL or TP has changed
         if(current_sl != g_tracked_orders[i].sl || current_tp != g_tracked_orders[i].tp)
         {
            // Send modify signal
            SendOrderModifySignal(ticket, current_sl, current_tp);
            Print("[ORDER] Modified: #", ticket, " SL=", current_sl, " TP=", current_tp);

            // Update tracked values
            g_tracked_orders[i].sl = current_sl;
            g_tracked_orders[i].tp = current_tp;
         }
      }
   }
}

//+------------------------------------------------------------------+
//| Check for partial closes (volume reduction)                       |
//+------------------------------------------------------------------+
void CheckForPartialCloses()
{
   for(int i = 0; i < ArraySize(g_tracked_orders); i++)
   {
      int ticket = g_tracked_orders[i].ticket;
      if(OrderSelect(ticket, SELECT_BY_TICKET, MODE_TRADES))
      {
         double current_lots = OrderLots();
         double tracked_lots = g_tracked_orders[i].lots;

         // Check if volume has decreased (partial close)
         if(current_lots < tracked_lots && tracked_lots > 0)
         {
            // Calculate close_ratio: portion that was closed
            double close_ratio = (tracked_lots - current_lots) / tracked_lots;

            Print("[ORDER] Partial close: #", ticket, " ", tracked_lots, " -> ", current_lots, " lots (ratio: ", DoubleToString(close_ratio * 100, 1), "%)");

            // Send partial close signal
            SendCloseSignal(g_ea_context, (TICKET_TYPE)ticket, close_ratio, AccountID);

            // Update tracked volume (order still exists)
            g_tracked_orders[i].lots = current_lots;
         }
      }
   }
}

//+------------------------------------------------------------------+
//| Check for closed orders                                           |
//+------------------------------------------------------------------+
void CheckForClosedOrders()
{
   // Check if any tracked order is no longer in open orders
   for(int i = ArraySize(g_tracked_orders) - 1; i >= 0; i--)
   {
      int ticket = g_tracked_orders[i].ticket;
      bool found = false;

      for(int j = 0; j < OrdersTotal(); j++)
      {
         if(OrderSelect(j, SELECT_BY_POS, MODE_TRADES))
         {
            if(OrderTicket() == ticket)
            {
               found = true;
               break;
            }
         }
      }

      if(!found)
      {
         // Order was closed (full close)
         if(OrderSelect(ticket, SELECT_BY_TICKET, MODE_HISTORY))
         {
            SendCloseSignal(g_ea_context, (TICKET_TYPE)ticket, 0.0, AccountID);
            RemoveTrackedOrder(ticket);
            Print("[ORDER] Closed: #", ticket);
         }
      }
   }
}

//+------------------------------------------------------------------+
//| Send open signal from order                                      |
//+------------------------------------------------------------------+
void SendOpenSignalFromOrder(int ticket)
{
   if(!OrderSelect(ticket, SELECT_BY_TICKET))
   {
      // Try history
      if(!OrderSelect(ticket, SELECT_BY_TICKET, MODE_HISTORY))
      {
         Print("ERROR: Cannot select order #", ticket);
         return;
      }
   }

   string order_type = GetOrderTypeString(OrderType());
   string raw_symbol = OrderSymbol();
   string symbol = GetCleanSymbol(raw_symbol, g_symbol_prefix, g_symbol_suffix);
   
   SendOpenSignal(g_ea_context, (TICKET_TYPE)ticket, symbol,
                  order_type, OrderLots(), OrderOpenPrice(), OrderStopLoss(),
                  OrderTakeProfit(), OrderMagicNumber(), OrderComment(), AccountID);
}

//+------------------------------------------------------------------+
//| Send modify signal                                                |
//+------------------------------------------------------------------+
void SendOrderModifySignal(int ticket, double sl, double tp)
{
   SendModifySignal(g_ea_context, (TICKET_TYPE)ticket, sl, tp, AccountID);
}

//+------------------------------------------------------------------+
//| Helper functions                                                  |
//+------------------------------------------------------------------+

//+------------------------------------------------------------------+
//| Check if order is already being tracked                          |
//+------------------------------------------------------------------+
bool IsOrderTracked(int ticket)
{
   for(int i = 0; i < ArraySize(g_tracked_orders); i++)
   {
      if(g_tracked_orders[i].ticket == ticket)
         return true;
   }
   return false;
}

//+------------------------------------------------------------------+
//| Add order to tracking list with current SL/TP/Lots               |
//+------------------------------------------------------------------+
void AddTrackedOrder(int ticket)
{
   if(!OrderSelect(ticket, SELECT_BY_TICKET))
      return;

   int size = ArraySize(g_tracked_orders);
   ArrayResize(g_tracked_orders, size + 1);
   g_tracked_orders[size].ticket = ticket;
   g_tracked_orders[size].sl = OrderStopLoss();
   g_tracked_orders[size].tp = OrderTakeProfit();
   g_tracked_orders[size].lots = OrderLots();
}

//+------------------------------------------------------------------+
//| Remove order from tracking list                                  |
//+------------------------------------------------------------------+
void RemoveTrackedOrder(int ticket)
{
   for(int i = 0; i < ArraySize(g_tracked_orders); i++)
   {
      if(g_tracked_orders[i].ticket == ticket)
      {
         // Shift array elements
         for(int j = i; j < ArraySize(g_tracked_orders) - 1; j++)
         {
            g_tracked_orders[j] = g_tracked_orders[j + 1];
         }
         ArrayResize(g_tracked_orders, ArraySize(g_tracked_orders) - 1);
         break;
      }
   }
}
