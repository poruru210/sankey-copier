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
#include <SankeyCopier/Common.mqh>
#include <SankeyCopier/Zmq.mqh>
#include <SankeyCopier/Messages.mqh>
#include <SankeyCopier/Trade.mqh>
#include <SankeyCopier/GridPanel.mqh>
#include <SankeyCopier/Logging.mqh>

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
HANDLE_TYPE g_zmq_context = -1;
HANDLE_TYPE g_zmq_socket = -1;
HANDLE_TYPE g_zmq_config_socket = -1;   // Socket for receiving config/sync requests
OrderInfo   g_tracked_orders[];
bool        g_initialized = false;
datetime    g_last_heartbeat = 0;
bool        g_last_trade_allowed = false; // Track auto-trading state for change detection
bool        g_config_requested = false;   // Track if config request has been sent

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

   // Initialize ZMQ context
   g_zmq_context = InitializeZmqContext();
   if(g_zmq_context < 0)
      return INIT_FAILED;

   // Create and connect PUSH socket
   g_zmq_socket = CreateAndConnectZmqSocket(g_zmq_context, ZMQ_PUSH, g_RelayAddress, "Master PUSH");
   if(g_zmq_socket < 0)
   {
      CleanupZmqContext(g_zmq_context);
      return INIT_FAILED;
   }

   // Create and connect config socket (SUB to receive SyncRequest)
   g_zmq_config_socket = CreateAndConnectZmqSocket(g_zmq_context, ZMQ_SUB, g_ConfigAddress, "Master Config SUB");
   if(g_zmq_config_socket < 0)
   {
      CleanupZmqSocket(g_zmq_socket, "Master PUSH");
      CleanupZmqContext(g_zmq_context);
      return INIT_FAILED;
   }

   // Subscribe to messages for this account ID
   if(!SubscribeToTopic(g_zmq_config_socket, AccountID))
   {
      CleanupZmqSocket(g_zmq_config_socket, "Master Config SUB");
      CleanupZmqSocket(g_zmq_socket, "Master PUSH");
      CleanupZmqContext(g_zmq_context);
      return INIT_FAILED;
   }

   // Subscribe to VictoriaLogs config (global broadcast)
   if(!SubscribeToTopic(g_zmq_config_socket, "vlogs_config"))
   {
      Print("WARNING: Failed to subscribe to vlogs_config topic");
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
      g_config_panel.UpdateStatusRow(STATUS_NO_CONFIGURATION);

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
   SendUnregistrationMessage(g_zmq_context, g_RelayAddress, AccountID);

   // Kill timer
   EventKillTimer();

   // Delete configuration panel
   if(ShowConfigPanel)
      g_config_panel.Delete();

   // Cleanup ZMQ resources
   CleanupZmqMultiSocket(g_zmq_socket, g_zmq_config_socket, g_zmq_context, "Master PUSH", "Master Config SUB");

   Print("=== SankeyCopier Master EA (MT4) Stopped ===");
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
   // Use TimeLocal() instead of TimeCurrent() to ensure heartbeat works even when market is closed
   datetime now = TimeLocal();
   bool should_send_heartbeat = (now - g_last_heartbeat >= HEARTBEAT_INTERVAL_SECONDS) || trade_state_changed;

   if(should_send_heartbeat)
   {
      SendHeartbeatMessage(g_zmq_context, g_RelayAddress, AccountID, "Master", "MT4", g_symbol_prefix, g_symbol_suffix, "");
      g_last_heartbeat = TimeLocal();

      // If trade state changed, log it and update tracking variable
      if(trade_state_changed)
      {
         Print("[INFO] Auto-trading state changed: ", g_last_trade_allowed, " -> ", current_trade_allowed);
         g_last_trade_allowed = current_trade_allowed;

         // Update panel status
         if(ShowConfigPanel)
         {
            if(!current_trade_allowed)
            {
               g_config_panel.UpdateStatusRow(STATUS_ENABLED); // Yellow warning
            }
            else
            {
               g_config_panel.UpdateStatusRow(STATUS_CONNECTED); // Green active
            }
            ChartRedraw();
         }
      }

      // Request configuration if not yet requested (on any successful heartbeat)
      if(!g_config_requested && current_trade_allowed)
      {
         Print("[INFO] First heartbeat successful, requesting configuration...");
         if(SendRequestConfigMessage(g_zmq_context, g_RelayAddress, AccountID, "Master"))
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

   // Check for SyncRequest messages
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
         if(topic == "vlogs_config")
         {
            HANDLE_TYPE vlogs_handle = parse_vlogs_config(msgpack_payload, payload_len);
            if(vlogs_handle != 0 && vlogs_handle != -1)
            {
               VLogsApplyConfig(vlogs_handle, "master", AccountID);
               vlogs_config_free(vlogs_handle);
            }
         }
         // Try to parse as MasterConfig
         else if(topic == AccountID)
         {
            HANDLE_TYPE config_handle = parse_master_config(msgpack_payload, payload_len);
            if(config_handle != 0 && config_handle != -1)
            {
               // Valid MasterConfig
               ProcessMasterConfigMessage(msgpack_payload, payload_len);
            }
            else
            {
               // Not MasterConfig - try SyncRequest
               ProcessSyncRequest(msgpack_payload, payload_len);
            }
         }
      }
   }

   // Flush VictoriaLogs periodically
   VLogsFlushIfNeeded();
}

//+------------------------------------------------------------------+
//| Process Master configuration message (MessagePack)               |
//+------------------------------------------------------------------+
void ProcessMasterConfigMessage(uchar &msgpack_data[], int data_len)
{
   // Parse MessagePack once and get a handle to the Master config structure
   HANDLE_TYPE config_handle = parse_master_config(msgpack_data, data_len);
   if(config_handle == 0 || config_handle == -1)
   {
      Print("ERROR: Failed to parse MessagePack Master config");
      return;
   }

   // Extract fields from the parsed config using the handle
   string config_account_id = master_config_get_string(config_handle, "account_id");
   string prefix = master_config_get_string(config_handle, "symbol_prefix");
   string suffix = master_config_get_string(config_handle, "symbol_suffix");
   int version = master_config_get_int(config_handle, "config_version");

   if(config_account_id == "")
   {
      Print("ERROR: Invalid config message received - missing account_id");
      master_config_free(config_handle);
      return;
   }

   // Verify this config is for us
   if(config_account_id != AccountID)
   {
      Print("WARNING: Received config for different account: ", config_account_id, " (expected: ", AccountID, ")");
      master_config_free(config_handle);
      return;
   }

   // Log configuration update
   Print("[CONFIG] Received: prefix=", (prefix == "" ? "(none)" : prefix),
         " suffix=", (suffix == "" ? "(none)" : suffix), " version=", version);

   // Update global configuration variables
   g_symbol_prefix = prefix;
   g_symbol_suffix = suffix;
   g_config_version = (uint)version;

   // Update configuration panel
   if(ShowConfigPanel)
   {
      g_config_panel.UpdateSymbolConfig(g_symbol_prefix, g_symbol_suffix, "");

      // Update status after receiving configuration
      bool local_trade_allowed = (bool)TerminalInfoInteger(TERMINAL_TRADE_ALLOWED);
      if(!local_trade_allowed)
      {
         g_config_panel.UpdateStatusRow(STATUS_ENABLED); // Yellow warning
      }
      else
      {
         g_config_panel.UpdateStatusRow(STATUS_CONNECTED); // Green active
      }
      ChartRedraw();
   }

   // Free the config handle
   master_config_free(config_handle);
}

//+------------------------------------------------------------------+
//| Process SyncRequest message (from Slave EA)                       |
//+------------------------------------------------------------------+
void ProcessSyncRequest(uchar &msgpack_data[], int data_len)
{
   // Try to parse as SyncRequest
   HANDLE_TYPE handle = parse_sync_request(msgpack_data, data_len);
   if(handle == 0 || handle == -1)
   {
      // Not a SyncRequest - ignore silently
      return;
   }

   // Get the fields
   string slave_account = sync_request_get_string(handle, "slave_account");
   string master_account = sync_request_get_string(handle, "master_account");

   if(slave_account == "" || master_account == "")
   {
      Print("Invalid SyncRequest received - missing fields");
      sync_request_free(handle);
      return;
   }

   // Check if this request is for us
   if(master_account != AccountID)
   {
      Print("SyncRequest for different master: ", master_account, " (we are: ", AccountID, ")");
      sync_request_free(handle);
      return;
   }

   // Free the handle before sending response
   sync_request_free(handle);

   // Send position snapshot
   if(SendPositionSnapshot(g_zmq_socket, AccountID, g_symbol_prefix, g_symbol_suffix))
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
            SendCloseSignal(g_zmq_socket, (TICKET_TYPE)ticket, close_ratio, AccountID);

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
            SendCloseSignal(g_zmq_socket, (TICKET_TYPE)ticket, 0.0, AccountID);
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
   
   SendOpenSignal(g_zmq_socket, (TICKET_TYPE)ticket, symbol,
                  order_type, OrderLots(), OrderOpenPrice(), OrderStopLoss(),
                  OrderTakeProfit(), OrderMagicNumber(), OrderComment(), AccountID);
}

//+------------------------------------------------------------------+
//| Send modify signal                                                |
//+------------------------------------------------------------------+
void SendOrderModifySignal(int ticket, double sl, double tp)
{
   SendModifySignal(g_zmq_socket, (TICKET_TYPE)ticket, sl, tp, AccountID);
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
