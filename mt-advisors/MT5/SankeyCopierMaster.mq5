//+------------------------------------------------------------------+
//|                                       SankeyCopierMaster.mq5      |
//|                        Copyright 2025, SANKEY Copier Project      |
//|                                                                  |
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, SANKEY Copier Project"
#property link      ""
#property version   "1.00"  // VERSION_PLACEHOLDER
#property icon      "app.ico"

//--- Include common headers
#include "../Include/SankeyCopier/Common.mqh"
// ZMQ.mqh removed
#include "../Include/SankeyCopier/MasterSignals.mqh"
#include "../Include/SankeyCopier/Trade.mqh"
#include "../Include/SankeyCopier/GridPanel.mqh"
#include "../Include/SankeyCopier/Logging.mqh"

//--- Input parameters
// Note: SymbolPrefix/SymbolSuffix moved to Web-UI MasterSettings
// ZMQ addresses are loaded from sankey_copier.ini (no input override)
input int      ScanInterval = 100;
input bool     ShowConfigPanel = true;                  // Show configuration panel on chart
input int      PanelWidth = 280;                        // Configuration panel width (pixels)

//--- Resolved addresses (from sankey_copier.ini config file)
string g_RelayAddress = "";
string g_ConfigAddress = "";

//--- Position tracking structure
struct PositionInfo
{
   ulong  ticket;
   double sl;
   double tp;
   double lots;  // Track volume for partial close detection
};

//--- Order tracking structure (for Pending Orders)
struct OrderInfo
{
   ulong  ticket;
   double price;
   double sl;
   double tp;
   double lots;  // Track volume for partial close detection
};

//--- Global variables
string        AccountID;                  // Auto-generated from broker + account number
// ZMQ handles removed - managed by EaContext
// HANDLE_TYPE   g_zmq_context = -1; 
// HANDLE_TYPE   g_zmq_socket = -1;
// HANDLE_TYPE   g_zmq_config_socket = -1;
PositionInfo  g_tracked_positions[];
OrderInfo     g_tracked_orders[];
bool          g_initialized = false;
datetime      g_last_heartbeat = 0;
bool          g_last_trade_allowed = false; // Track auto-trading state for change detection
bool          g_config_requested = false;   // Track if config request has been sent
string        g_symbol_prefix = "";       // Symbol prefix from config (applied dynamically)
string        g_symbol_suffix = "";       // Symbol suffix from config (applied dynamically)
uint          g_config_version = 0;       // Current config version
int           g_server_status = STATUS_NO_CONFIG; // Status from server (DISABLED/CONNECTED)
string        g_config_topic = "";        // Config topic (generated via FFI)
string        g_vlogs_topic = "";         // VLogs topic (generated via FFI)
string        g_sync_topic = "";          // Sync topic prefix for receiving SyncRequest (sync/{account_id}/)
bool          g_register_sent = false;    // Track if register message has been sent
EaContextWrapper g_ea_context;        // Rust EA Context wrapper


//--- Configuration panel
CGridPanel     g_config_panel;

//+------------------------------------------------------------------+
//| Expert initialization function                                     |
//+------------------------------------------------------------------+
int OnInit()
{
   Print("=== SankeyCopier Master EA (MT5) Starting ===");

   // Auto-generate AccountID from broker name and account number
   AccountID = GenerateAccountID();
   Print("Auto-generated AccountID: ", AccountID);

   // Symbol prefix/suffix are now managed via Web-UI MasterSettings
   // They will be set when config is received from relay-server
   g_symbol_prefix = "";
   g_symbol_suffix = "";

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
   g_ConfigAddress = GetConfigSubAddress();

   Print("Resolved addresses: PUSH=", g_RelayAddress, ", SUB=", g_ConfigAddress, " (unified)");

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

   // Initialize EA Context (Stateful FFI)
   if(!g_ea_context.Initialize(AccountID, EA_TYPE_MASTER, "MT5", GetAccountNumber(), 
                               GetBrokerName(), GetAccountName(), GetServerName(), 
                               GetAccountCurrency(), GetAccountLeverage()))
   {
      Print("[ERROR] Failed to initialize EA Context");
      return INIT_FAILED;
   }
   
   // Connect via FFI (High-Level)
   // This creates and manages the sockets internally within the Rust context
   if(!g_ea_context.Connect(g_RelayAddress, g_ConfigAddress))
   {
       Print("[ERROR] Failed to connect via EA Context");
       // Note: Depending on FFI implementation, Connect might return success even if immediate connection fails,
       // but if it returns false here, it's a critical setup error.
       return INIT_FAILED;
   }

   // Subscribe to config messages for this account ID
   if(!g_ea_context.SubscribeConfig(g_config_topic))
   {
      Print("[ERROR] Failed to subscribe to config topic");
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
      
      // Note: sync_topic is a PREFIX filter in Rust/ZMQ, subscribing to it receives all subtopics?
      // ZMQ usually matches prefix. So subscribing to "sync/{account_id}/" should work.
      if(!g_ea_context.SubscribeConfig(g_sync_topic))
      {
         Print("WARNING: Failed to subscribe to sync topic");
      }
   }
   else
   {
      Print("WARNING: Failed to generate sync topic prefix");
   }

   // Scan existing positions and orders
   ScanExistingPositions();
   ScanExistingOrders();
   
   g_initialized = true;

   // Set up timer for heartbeat (1 second interval)
   EventSetTimer(1);

   // Initialize configuration panel
   if(ShowConfigPanel)
   {
      g_config_panel.InitializeMasterPanel("SankeyCopierPanel_", PanelWidth);

      // Show NO_CONFIGURATION status initially (no config received yet)
      g_config_panel.UpdateStatusRow(STATUS_NO_CONFIG);

      g_config_panel.UpdateServerRow(g_RelayAddress);
      g_config_panel.UpdateTrackedOrdersRow(ArraySize(g_tracked_orders) + ArraySize(g_tracked_positions));
      g_config_panel.UpdateSymbolConfig(g_symbol_prefix, g_symbol_suffix, "");
   }

   Print("=== SankeyCopier Master EA (MT5) Initialized ===");

   // VictoriaLogs is configured via server-pushed vlogs_config message
   // (no local endpoint parameter needed)

   ChartRedraw();
   return INIT_SUCCEEDED;
}

//+------------------------------------------------------------------+
//| Expert deinitialization function                                  |
//+------------------------------------------------------------------+
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

   // Cleanup ZMQ resources
   // Managed by EaContextWrapper destructor
   // CleanupZmqMultiSocket(g_zmq_socket, g_zmq_config_socket, g_zmq_context, "Master PUSH", "Master Config SUB");

   Print("=== SankeyCopier Master EA (MT5) Stopped ===");
}

//+------------------------------------------------------------------+
//| Timer function (called every 1 second)                            |
//+------------------------------------------------------------------+
//+------------------------------------------------------------------+
//| Timer function (called every 1 second)                            |
//+------------------------------------------------------------------+
void OnTimer()
{
   if(!g_initialized) return;

   // 1. Run ManagerTick (Handles Heartbeats, Config Requests, ZMQ Polling internally)
   bool current_trade_allowed = (bool)TerminalInfoInteger(TERMINAL_TRADE_ALLOWED);
   
   int pending_commands = g_ea_context.ManagerTick(
       GetAccountBalance(), 
       GetAccountEquity(), 
       GetOpenPositionsCount(), 
       current_trade_allowed
   );

   // 2. Process all pending commands from Rust
   EaCommand cmd;
   int processed_count = 0;
   
   while(pending_commands > 0 || processed_count < 100) // Limit per tick
   {
       if(!g_ea_context.GetCommand(cmd)) break;
       processed_count++;

       switch(cmd.command_type)
       {
           case CMD_UPDATE_UI:
           {
               // Config updated (Master or Global VLogs) via ManagerTick parsing
               CMasterConfig config;
               if(g_ea_context.GetMasterConfig(config))
               {
                   ProcessMasterConfigMessage(config);
               }
               
               // Also check global VLogs? (Maybe stored separately or we just assume updated)
               break;
           }
           case CMD_SEND_SNAPSHOT: // SyncRequest Received
           {
               // Retrieve cached SyncRequest struct
               CSyncRequest request;
               
               if(g_ea_context.GetSyncRequest(request))
               {
                   string req_master = CharArrayToString(request.master_account);
                   string req_slave = CharArrayToString(request.slave_account);

                   // Validate Master Account
                   if(req_master == AccountID)
                   {
                        // Send the snapshot
                        if(SendPositionSnapshot(g_ea_context, AccountID, g_symbol_prefix, g_symbol_suffix))
                        {
                             Print("[SYNC] Position snapshot sent to slave: ", req_slave);
                        }
                        else
                        {
                             Print("[ERROR] Failed to send position snapshot to slave: ", req_slave);
                        }
                   }
                   else
                   {
                        Print(StringFormat("[ERROR] SyncRequest Master Mismatch. Req: '%s', Self: '%s'", req_master, AccountID));
                   }
               }
               else
               {
                    // Fallback: Use comment if struct retrieval fails (should not happen if Rust logic works)
                    string slave_account = CharArrayToString(cmd.comment);
                    Print("[WARNING] GetSyncRequest failed, using comment for slave account: ", slave_account);
                    
                    if(SendPositionSnapshot(g_ea_context, AccountID, g_symbol_prefix, g_symbol_suffix))
                    {
                         Print("[SYNC] Position snapshot sent to slave: ", slave_account);
                    }
               }
               break;
           }
           default:
               break;
       }
       
       // Check if more commands pending?
       // ManagerTick returns 1 if pending > 0. But we consumed one.
       // We loop until GetCommand returns false? 
       // Rust `get_next_command` pops one. If empty, GetCommand returns false (0).
   }

   // 3. Flush VLogs
   VLogsFlushIfNeeded();
}

//+------------------------------------------------------------------+
//| Process SyncRequest message (from Slave EA)                       |
//+------------------------------------------------------------------+
void ProcessSyncRequest(HANDLE_TYPE handle)
{
   // Get the fields
   string slave_account = sync_request_get_string(handle, "slave_account");
   string master_account = sync_request_get_string(handle, "master_account");

   if(slave_account == "" || master_account == "")
   {
      Print("Invalid SyncRequest received - missing fields");
      // Note: No need to free handle - managed by EaContext
      return;
   }

   if(master_account != AccountID)
   {
      Print("SyncRequest for different master: ", master_account, " (we are: ", AccountID, ")");
      // Note: No need to free handle - managed by EaContext
      return;
   }

   // Note: No need to free handle - managed by EaContext

   // Send position snapshot
   // Note: SendPositionSnapshot helper now takes g_ea_context
   if(SendPositionSnapshot(g_ea_context, AccountID, g_symbol_prefix, g_symbol_suffix))
   {
      Print("[SYNC] Position snapshot sent to slave: ", slave_account);
   }
   else
   {
      Print("[ERROR] Failed to send position snapshot to slave: ", slave_account);
   }
}

//+------------------------------------------------------------------+
//| Expert tick function                                              |
//+------------------------------------------------------------------+
void OnTick()
{
   if(!g_initialized) return;

   static datetime last_scan = 0;
   if(TimeCurrent() - last_scan > ScanInterval / 1000)
   {
      //--- 1. Scan Positions ---
      CheckForNewPositions();
      CheckForModifiedPositions();
      CheckForPartialCloses();
      CheckForClosedPositions();
      last_scan = TimeCurrent();
      
      //--- 2. Scan Pending Orders ---
      int total_orders = OrdersTotal();
      bool order_seen[];
      ArrayResize(order_seen, ArraySize(g_tracked_orders));
      ArrayInitialize(order_seen, false);
      
      // Check for new or modified orders
      for(int i = 0; i < total_orders; i++)
      {
         ulong ticket = OrderGetTicket(i);
         if(ticket > 0)
         {
            ENUM_ORDER_TYPE type = (ENUM_ORDER_TYPE)OrderGetInteger(ORDER_TYPE);
            // Only process pending orders (ignore market orders which become positions)
            if(type == ORDER_TYPE_BUY || type == ORDER_TYPE_SELL)
               continue;
               
            bool is_tracked = false;
            for(int j = 0; j < ArraySize(g_tracked_orders); j++)
            {
               if(g_tracked_orders[j].ticket == ticket)
               {
                  is_tracked = true;
                  order_seen[j] = true;
                  
                  // Check for modification
                  double current_price = OrderGetDouble(ORDER_PRICE_OPEN);
                  double current_sl = OrderGetDouble(ORDER_SL);
                  double current_tp = OrderGetDouble(ORDER_TP);
                  
                  if(current_price != g_tracked_orders[j].price ||
                     current_sl != g_tracked_orders[j].sl ||
                     current_tp != g_tracked_orders[j].tp)
                  {
                     SendOrderModifySignal(ticket);
                     g_tracked_orders[j].price = current_price;
                     g_tracked_orders[j].sl = current_sl;
                     g_tracked_orders[j].tp = current_tp;
                  }
                  break;
               }
            }
            // Master detects ALL orders - prefix/suffix is only used for symbol name cleaning
            if(!is_tracked)
            {
               string symbol = OrderGetString(ORDER_SYMBOL);
               SendOrderOpenSignal(ticket);
               AddTrackedOrder(ticket);
               Print("[ORDER] New: #", ticket, " ", symbol);
            }
         }
      }
      
      // Check for closed/deleted orders
      for(int i = ArraySize(g_tracked_orders) - 1; i >= 0; i--)
      {
         if(!order_seen[i])
         {
            SendOrderCloseSignal(g_tracked_orders[i].ticket);
            RemoveTrackedOrder(g_tracked_orders[i].ticket);
         }
      }
      
      // Update UI
      if(ShowConfigPanel)
      {
         g_config_panel.UpdateTrackedOrdersRow(ArraySize(g_tracked_positions) + ArraySize(g_tracked_orders));
      }
   }
}

//+------------------------------------------------------------------+
//| Trade transaction event                                           |
//+------------------------------------------------------------------+
void OnTradeTransaction(const MqlTradeTransaction &trans,
                       const MqlTradeRequest &request,
                       const MqlTradeResult &result)
{
   // Master detects ALL positions - prefix/suffix is only used for symbol name cleaning
   if(trans.type == TRADE_TRANSACTION_DEAL_ADD)
   {
      if(PositionSelectByTicket(trans.position))
      {
         string symbol = PositionGetString(POSITION_SYMBOL);
         SendPositionOpenSignal(trans.position);
         AddTrackedPosition(trans.position);
         Print("[POSITION] New: #", trans.position, " ", symbol);
      }
   }
   else if(trans.type == TRADE_TRANSACTION_HISTORY_ADD)
   {
      // Deal added to history (could be partial or full close)
      if(trans.deal_type == DEAL_TYPE_BUY || trans.deal_type == DEAL_TYPE_SELL)
      {
         // Check if position still exists (partial close) or not (full close)
         if(!PositionSelectByTicket(trans.position))
         {
            // Position no longer exists - full close
            SendPositionCloseSignal(trans.position, 0.0);
            RemoveTrackedPosition(trans.position);
         }
         // If position still exists, it's a partial close - handled by CheckForPartialCloses
      }
   }
   
   //--- Order Transactions (Pending Orders) ---
   // Master detects ALL orders - prefix/suffix is only used for symbol name cleaning
   if(trans.type == TRADE_TRANSACTION_ORDER_ADD)
   {
      ulong ticket = trans.order;
      if(OrderSelect(ticket))
      {
         ENUM_ORDER_TYPE type = (ENUM_ORDER_TYPE)OrderGetInteger(ORDER_TYPE);
         // Only process pending orders (ignore market orders which become positions)
         if(type != ORDER_TYPE_BUY && type != ORDER_TYPE_SELL)
         {
            string symbol = OrderGetString(ORDER_SYMBOL);
            SendOrderOpenSignal(ticket);
            AddTrackedOrder(ticket);
            Print("[ORDER] New: #", ticket, " ", symbol);
         }
      }
   }
   else if(trans.type == TRADE_TRANSACTION_ORDER_DELETE)
   {
      ulong ticket = trans.order;
      // If it was tracked, send close signal
      if(IsOrderTracked(ticket))
      {
         SendOrderCloseSignal(ticket);
         RemoveTrackedOrder(ticket);
      }
   }
   else if(trans.type == TRADE_TRANSACTION_ORDER_UPDATE)
   {
      ulong ticket = trans.order;
      if(IsOrderTracked(ticket) && OrderSelect(ticket))
      {
         // Check if modified (compare with tracked values)
         int size = ArraySize(g_tracked_orders);
         for(int i = 0; i < size; i++)
         {
            if(g_tracked_orders[i].ticket == ticket)
            {
               double current_price = OrderGetDouble(ORDER_PRICE_OPEN);
               double current_sl = OrderGetDouble(ORDER_SL);
               double current_tp = OrderGetDouble(ORDER_TP);
               
               if(current_price != g_tracked_orders[i].price ||
                  current_sl != g_tracked_orders[i].sl ||
                  current_tp != g_tracked_orders[i].tp)
               {
                  SendOrderModifySignal(ticket);
                  // Update tracked values
                  g_tracked_orders[i].price = current_price;
                  g_tracked_orders[i].sl = current_sl;
                  g_tracked_orders[i].tp = current_tp;
               }
               break;
            }
         }
      }
   }
}

//+------------------------------------------------------------------+
//| Scan existing positions                                           |
//+------------------------------------------------------------------+
void ScanExistingPositions()
{
   ArrayResize(g_tracked_positions, 0);

   for(int i = 0; i < PositionsTotal(); i++)
   {
      ulong ticket = PositionGetTicket(i);
      if(ticket > 0)
      {
         string symbol = PositionGetString(POSITION_SYMBOL);
         if(MatchesSymbolFilter(symbol, g_symbol_prefix, g_symbol_suffix))
         {
            AddTrackedPosition(ticket);
            SendPositionOpenSignal(ticket);  // Send Open signal for existing positions
         }
      }
   }
}

//+------------------------------------------------------------------+
//| Check for new positions                                           |
//+------------------------------------------------------------------+
void CheckForNewPositions()
{
   for(int i = 0; i < PositionsTotal(); i++)
   {
      ulong ticket = PositionGetTicket(i);
      if(ticket > 0)
      {
          if(!IsPositionTracked(ticket))
          {
             string symbol = PositionGetString(POSITION_SYMBOL);
             if(MatchesSymbolFilter(symbol, g_symbol_prefix, g_symbol_suffix))
             {
                AddTrackedPosition(ticket);
                SendPositionOpenSignal(ticket);
             }
          }
      }
   }
}

//+------------------------------------------------------------------+
//| Check for modified positions (SL/TP changes)                      |
//+------------------------------------------------------------------+
void CheckForModifiedPositions()
{
   for(int i = 0; i < ArraySize(g_tracked_positions); i++)
   {
      ulong ticket = g_tracked_positions[i].ticket;
      if(PositionSelectByTicket(ticket))
      {
         double current_sl = PositionGetDouble(POSITION_SL);
         double current_tp = PositionGetDouble(POSITION_TP);

         // Check if SL or TP has changed
         if(current_sl != g_tracked_positions[i].sl || current_tp != g_tracked_positions[i].tp)
         {
            // Send modify signal
            SendPositionModifySignal(ticket, current_sl, current_tp);

            // Update tracked values
            g_tracked_positions[i].sl = current_sl;
            g_tracked_positions[i].tp = current_tp;
         }
      }
   }
}

//+------------------------------------------------------------------+
//| Check for partial closes (volume reduction)                       |
//+------------------------------------------------------------------+
void CheckForPartialCloses()
{
   for(int i = 0; i < ArraySize(g_tracked_positions); i++)
   {
      ulong ticket = g_tracked_positions[i].ticket;
      if(PositionSelectByTicket(ticket))
      {
         double current_lots = PositionGetDouble(POSITION_VOLUME);
         double tracked_lots = g_tracked_positions[i].lots;

         // Check if volume has decreased (partial close)
         if(current_lots < tracked_lots && tracked_lots > 0)
         {
            // Calculate close_ratio: portion that was closed
            double close_ratio = (tracked_lots - current_lots) / tracked_lots;

            Print("Partial close detected: #", ticket, " ", tracked_lots, " -> ", current_lots, " lots (close_ratio: ", close_ratio, ")");

            // Send partial close signal
            SendPositionCloseSignal(ticket, close_ratio);

            // Update tracked volume (position still exists)
            g_tracked_positions[i].lots = current_lots;
         }
      }
   }
}

//+------------------------------------------------------------------+
//| Check for closed positions                                        |
//+------------------------------------------------------------------+
void CheckForClosedPositions()
{
   for(int i = ArraySize(g_tracked_positions) - 1; i >= 0; i--)
   {
      ulong ticket = g_tracked_positions[i].ticket;
      if(!PositionSelectByTicket(ticket))
      {
         // Full close (close_ratio = 1.0 or 0.0 for backward compat)
         SendPositionCloseSignal(ticket, 0.0);
         RemoveTrackedPosition(ticket);
      }
   }
}

//+------------------------------------------------------------------+
//| Send position open signal                                        |
//+------------------------------------------------------------------+
//+------------------------------------------------------------------+
//| Send position open signal                                        |
//+------------------------------------------------------------------+
void SendPositionOpenSignal(ulong ticket)
{
   if(!PositionSelectByTicket(ticket))
      return;

   string raw_symbol = PositionGetString(POSITION_SYMBOL);
   string symbol = GetCleanSymbol(raw_symbol, g_symbol_prefix, g_symbol_suffix);
   
   long type = PositionGetInteger(POSITION_TYPE);
   double volume = PositionGetDouble(POSITION_VOLUME);
   double price = PositionGetDouble(POSITION_PRICE_OPEN);
   double sl = PositionGetDouble(POSITION_SL);
   double tp = PositionGetDouble(POSITION_TP);
   long magic = PositionGetInteger(POSITION_MAGIC);
   string comment = PositionGetString(POSITION_COMMENT);

   string order_type = GetOrderTypeString((ENUM_POSITION_TYPE)type);

   SendOpenSignal(g_ea_context, ticket, symbol, order_type,
                  volume, price, sl, tp, magic, comment, AccountID);
}

//+------------------------------------------------------------------+
//| Send close signal with optional close_ratio                       |
//| close_ratio: 0 = full close, 0 < ratio < 1.0 = partial close     |
//+------------------------------------------------------------------+
void SendPositionCloseSignal(ulong ticket, double close_ratio = 0.0)
{
   SendCloseSignal(g_ea_context, ticket, close_ratio, AccountID);
}

//+------------------------------------------------------------------+
//| Send modify signal                                                |
//+------------------------------------------------------------------+
void SendPositionModifySignal(ulong ticket, double sl, double tp)
{
   SendModifySignal(g_ea_context, ticket, sl, tp, AccountID);
}

//+------------------------------------------------------------------+
//| Helper functions                                                  |
//+------------------------------------------------------------------+

//+------------------------------------------------------------------+
//| Check if position is already being tracked                       |
//+------------------------------------------------------------------+
bool IsPositionTracked(ulong ticket)
{
   for(int i = 0; i < ArraySize(g_tracked_positions); i++)
      if(g_tracked_positions[i].ticket == ticket) return true;
   return false;
}

//+------------------------------------------------------------------+
//| Add position to tracking list with current SL/TP/Lots            |
//+------------------------------------------------------------------+
void AddTrackedPosition(ulong ticket)
{
   if(!PositionSelectByTicket(ticket)) return;
   if(IsPositionTracked(ticket)) return;

   int size = ArraySize(g_tracked_positions);
   ArrayResize(g_tracked_positions, size + 1);
   g_tracked_positions[size].ticket = ticket;
   g_tracked_positions[size].sl = PositionGetDouble(POSITION_SL);
   g_tracked_positions[size].tp = PositionGetDouble(POSITION_TP);
   g_tracked_positions[size].lots = PositionGetDouble(POSITION_VOLUME);
}

//+------------------------------------------------------------------+
//| Add order to tracking list with current volume                   |
//+------------------------------------------------------------------+
void AddTrackedOrder(ulong ticket)
{
   if(!OrderSelect(ticket)) return;
   if(IsOrderTracked(ticket)) return;

   int size = ArraySize(g_tracked_orders);
   ArrayResize(g_tracked_orders, size + 1);

   g_tracked_orders[size].ticket = ticket;
   g_tracked_orders[size].price  = OrderGetDouble(ORDER_PRICE_OPEN);
   g_tracked_orders[size].sl     = OrderGetDouble(ORDER_SL);
   g_tracked_orders[size].tp     = OrderGetDouble(ORDER_TP);
   g_tracked_orders[size].lots   = OrderGetDouble(ORDER_VOLUME_INITIAL);
}

//+------------------------------------------------------------------+
//| Remove position from tracking list                               |
//+------------------------------------------------------------------+
void RemoveTrackedPosition(ulong ticket)
{
   for(int i = 0; i < ArraySize(g_tracked_positions); i++)
   {
      if(g_tracked_positions[i].ticket == ticket)
      {
         for(int j = i; j < ArraySize(g_tracked_positions) - 1; j++)
            g_tracked_positions[j] = g_tracked_positions[j + 1];
         ArrayResize(g_tracked_positions, ArraySize(g_tracked_positions) - 1);
         break;
      }
   }
}

//+------------------------------------------------------------------+
//| Check if order is already being tracked                          |
//+------------------------------------------------------------------+
bool IsOrderTracked(ulong ticket)
{
   for(int i = 0; i < ArraySize(g_tracked_orders); i++)
      if(g_tracked_orders[i].ticket == ticket) return true;
   return false;
}

//+------------------------------------------------------------------+
//| Remove order from tracking list                                  |
//+------------------------------------------------------------------+
void RemoveTrackedOrder(ulong ticket)
{
   for(int i = 0; i < ArraySize(g_tracked_orders); i++)
   {
      if(g_tracked_orders[i].ticket == ticket)
      {
         for(int j = i; j < ArraySize(g_tracked_orders) - 1; j++)
            g_tracked_orders[j] = g_tracked_orders[j + 1];
         ArrayResize(g_tracked_orders, ArraySize(g_tracked_orders) - 1);
         break;
      }
   }
}

//+------------------------------------------------------------------+
//| Scan existing pending orders                                      |
//+------------------------------------------------------------------+
void ScanExistingOrders()
{
   ArrayResize(g_tracked_orders, 0);

   for(int i = 0; i < OrdersTotal(); i++)
   {
      ulong ticket = OrderGetTicket(i);
      if(ticket > 0)
      {
         ENUM_ORDER_TYPE type = (ENUM_ORDER_TYPE)OrderGetInteger(ORDER_TYPE);
         // Only process pending orders
         if(type != ORDER_TYPE_BUY && type != ORDER_TYPE_SELL)
         {
            string symbol = OrderGetString(ORDER_SYMBOL);
            if(MatchesSymbolFilter(symbol, g_symbol_prefix, g_symbol_suffix))
            {
               AddTrackedOrder(ticket);
               SendOrderOpenSignal(ticket);
            }
         }
      }
   }
}

//+------------------------------------------------------------------+
//| Send order open signal                                           |
//+------------------------------------------------------------------+
void SendOrderOpenSignal(ulong ticket)
{
   if(!OrderSelect(ticket)) return;

   string raw_symbol = OrderGetString(ORDER_SYMBOL);
   string symbol = GetCleanSymbol(raw_symbol, g_symbol_prefix, g_symbol_suffix);
   
   long type = OrderGetInteger(ORDER_TYPE);
   double volume = OrderGetDouble(ORDER_VOLUME_INITIAL);
   double price = OrderGetDouble(ORDER_PRICE_OPEN);
   double sl = OrderGetDouble(ORDER_SL);
   double tp = OrderGetDouble(ORDER_TP);
   long magic = OrderGetInteger(ORDER_MAGIC);
   string comment = OrderGetString(ORDER_COMMENT);

   string order_type = GetOrderTypeString((int)type);

   SendOpenSignal(g_ea_context, ticket, symbol, order_type,
                  volume, price, sl, tp, magic, comment, AccountID);
}

//+------------------------------------------------------------------+
//| Send order modify signal                                         |
//+------------------------------------------------------------------+
void SendOrderModifySignal(ulong ticket)
{
    if(!OrderSelect(ticket)) return;
    
    double sl = OrderGetDouble(ORDER_SL);
    double tp = OrderGetDouble(ORDER_TP);
    
    SendModifySignal(g_ea_context, ticket, sl, tp, AccountID);
}

//+------------------------------------------------------------------+
//| Send order close signal (delete)                                 |
//| Pending orders don't have partial close - always full close      |
//+------------------------------------------------------------------+
void SendOrderCloseSignal(ulong ticket)
{
   SendCloseSignal(g_ea_context, ticket, 0.0, AccountID);
}

//+------------------------------------------------------------------+
//| Process Master configuration message (struct)                    |
//+------------------------------------------------------------------+
void ProcessMasterConfigMessage(CMasterConfig &config)
{
   Print("=== Processing Master Configuration Message ===");

   // Extract fields from the struct
   string config_account_id = CharArrayToString(config.account_id);
   int status = config.status;
   string prefix = CharArrayToString(config.symbol_prefix);
   string suffix = CharArrayToString(config.symbol_suffix);
   int version = (int)config.config_version;

   if(config_account_id == "")
   {
      Print("ERROR: Invalid config message received - missing account_id");
      return;
   }

   if(config_account_id != AccountID)
   {
      Print("WARNING: Received config for different account: ", config_account_id, " (expected: ", AccountID, ")");
      // Note: No need to free handle - managed by EaContext
      return;
   }

   // Log configuration values
   Print("Account ID: ", config_account_id);
   Print("Status: ", status, " (", GetStatusString(status), ")");
   Print("Symbol Prefix: ", (prefix == "" ? "(none)" : prefix));
   Print("Symbol Suffix: ", (suffix == "" ? "(none)" : suffix));
   Print("Config Version: ", version);

   // Update global configuration variables
   if(status == STATUS_NO_CONFIG)
   {
      Print("Status is NO_CONFIG -> Resetting configuration");
      g_server_status = STATUS_NO_CONFIG;
      g_symbol_prefix = "";
      g_symbol_suffix = "";
      g_config_version = 0;
   }
   else
   {
      g_server_status = status;
      g_symbol_prefix = prefix;
      g_symbol_suffix = suffix;
      g_config_version = version;
   }

   // Update configuration panel with server status
   if(ShowConfigPanel)
   {
      g_config_panel.UpdateSymbolConfig(g_symbol_prefix, g_symbol_suffix, "");
      g_config_panel.UpdateStatusRow(g_server_status);
      ChartRedraw();
   }

   Print("=== Master Configuration Updated ===");
}

//+------------------------------------------------------------------+
//| Get status string for logging                                    |
//+------------------------------------------------------------------+
string GetStatusString(int status)
{
   switch(status)
   {
      case STATUS_DISABLED:         return "DISABLED";
      case STATUS_ENABLED:          return "ENABLED";
      case STATUS_CONNECTED:        return "CONNECTED";
      case STATUS_NO_CONFIG:        return "NO_CONFIG";
      default:                      return "UNKNOWN";
   }
}

