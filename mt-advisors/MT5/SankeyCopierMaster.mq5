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
//--- Include common headers
#include "../Include/SankeyCopier/Common.mqh"
#include "../Include/SankeyCopier/Zmq.mqh"
#include "../Include/SankeyCopier/Messages.mqh"
#include "../Include/SankeyCopier/Trade.mqh"
#include "../Include/SankeyCopier/GridPanel.mqh"

//--- Input parameters
input string   RelayServerAddress = DEFAULT_ADDR_PULL;       // Address to send signals/heartbeats (PULL)
input string   ConfigSourceAddress = DEFAULT_ADDR_PUB_CONFIG; // Address to receive config updates (SUB)
input string   SymbolPrefix = "";       // Symbol prefix to filter and strip (e.g. "pro.")
input string   SymbolSuffix = "";       // Symbol suffix to filter and strip (e.g. ".m")
input int      ScanInterval = 100;
input bool     ShowConfigPanel = true;                  // Show configuration panel on chart
input int      PanelWidth = 280;                        // Configuration panel width (pixels)

//--- Position tracking structure
struct PositionInfo
{
   ulong  ticket;
   double sl;
   double tp;
};

//--- Order tracking structure (for Pending Orders)
struct OrderInfo
{
   ulong  ticket;
   double price;
   double sl;
   double tp;
};

//--- Global variables
string        AccountID;                  // Auto-generated from broker + account number
HANDLE_TYPE   g_zmq_context = -1;
HANDLE_TYPE   g_zmq_socket = -1;
HANDLE_TYPE   g_zmq_config_socket = -1;   // Socket for receiving configuration
PositionInfo  g_tracked_positions[];
OrderInfo     g_tracked_orders[];
bool          g_initialized = false;
datetime      g_last_heartbeat = 0;
bool          g_last_trade_allowed = false; // Track auto-trading state for change detection
bool          g_config_requested = false;   // Track if config request has been sent
string        g_symbol_prefix = "";       // Symbol prefix from config (applied dynamically)
string        g_symbol_suffix = "";       // Symbol suffix from config (applied dynamically)
uint          g_config_version = 0;       // Current config version

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

   // Initialize symbol prefix/suffix from input parameters (will be overridden by config)
   g_symbol_prefix = SymbolPrefix;
   g_symbol_suffix = SymbolSuffix;

   // Initialize ZMQ context
   g_zmq_context = InitializeZmqContext();
   if(g_zmq_context < 0)
      return INIT_FAILED;

   // Create and connect PUSH socket
   g_zmq_socket = CreateAndConnectZmqSocket(g_zmq_context, ZMQ_PUSH, RelayServerAddress, "Master PUSH");
   if(g_zmq_socket < 0)
   {
      zmq_context_destroy(g_zmq_context);
      return INIT_FAILED;
   }

   // Create and connect config socket (SUB to port 5557)
   g_zmq_config_socket = CreateAndConnectZmqSocket(g_zmq_context, ZMQ_SUB, ConfigSourceAddress, "Master Config SUB");
   if(g_zmq_config_socket < 0)
   {
      CleanupZmqSocket(g_zmq_socket, "Master PUSH");
      zmq_context_destroy(g_zmq_context);
      return INIT_FAILED;
   }

   // Subscribe to config messages for this account ID
   if(!SubscribeToTopic(g_zmq_config_socket, AccountID))
   {
      CleanupZmqSocket(g_zmq_config_socket, "Master Config SUB");
      CleanupZmqSocket(g_zmq_socket, "Master PUSH");
      zmq_context_destroy(g_zmq_context);
      return INIT_FAILED;
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
      
      // Update panel immediately
      bool local_trade_allowed = (bool)TerminalInfoInteger(TERMINAL_TRADE_ALLOWED);
      if(!local_trade_allowed)
      {
         g_config_panel.UpdateStatusRow(STATUS_ENABLED); // Yellow warning
      }
      else
      {
         g_config_panel.UpdateStatusRow(STATUS_CONNECTED); // Green active
      }
      
      g_config_panel.UpdateServerRow(RelayServerAddress);
      g_config_panel.UpdateTrackedOrdersRow(ArraySize(g_tracked_orders) + ArraySize(g_tracked_positions));
      g_config_panel.UpdateSymbolConfig(g_symbol_prefix, g_symbol_suffix, "");
   }

   Print("=== SankeyCopier Master EA (MT5) Initialized ===");
   ChartRedraw();
   return INIT_SUCCEEDED;
}

//+------------------------------------------------------------------+
//| Expert deinitialization function                                  |
//+------------------------------------------------------------------+
void OnDeinit(const int reason)
{
   // Send unregister message to server
   SendUnregistrationMessage(g_zmq_context, RelayServerAddress, AccountID);

   // Kill timer
   EventKillTimer();

   // Delete configuration panel
   if(ShowConfigPanel)
      g_config_panel.Delete();

   // Cleanup ZMQ resources
   CleanupZmqMultiSocket(g_zmq_socket, g_zmq_config_socket, g_zmq_context, "Master PUSH", "Master Config SUB");

   Print("=== SankeyCopier Master EA (MT5) Stopped ===");
}

//+------------------------------------------------------------------+
//| Timer function (called every 1 second)                            |
//+------------------------------------------------------------------+
void OnTimer()
{
   if(!g_initialized) return;

   // Check for auto-trading state change (IsTradeAllowed)
   bool current_trade_allowed = (bool)TerminalInfoInteger(TERMINAL_TRADE_ALLOWED);
   bool trade_state_changed = (current_trade_allowed != g_last_trade_allowed);

   // Send heartbeat every HEARTBEAT_INTERVAL_SECONDS OR on trade state change
   // Use TimeLocal() instead of TimeCurrent() to ensure heartbeat works even when market is closed
   datetime now = TimeLocal();
   bool should_send_heartbeat = (now - g_last_heartbeat >= HEARTBEAT_INTERVAL_SECONDS) || trade_state_changed;

   if(should_send_heartbeat)
   {
      bool heartbeat_sent = SendHeartbeatMessage(g_zmq_context, RelayServerAddress, AccountID, "Master", "MT5", g_symbol_prefix, g_symbol_suffix, "");

      if(heartbeat_sent)
      {
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
            }
         }
         else
         {
            // On first successful heartbeat (normal interval), request configuration if not yet requested
            if(!g_config_requested)
            {
               Print("[INFO] First heartbeat successful, requesting configuration...");
               if(SendRequestConfigMessage(g_zmq_context, RelayServerAddress, AccountID, "Master"))
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
         ProcessMasterConfigMessage(msgpack_payload, payload_len);
      }
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
                        if(!is_tracked)
             {
                string symbol = OrderGetString(ORDER_SYMBOL);
                if(MatchesSymbolFilter(symbol, g_symbol_prefix, g_symbol_suffix))
                {
                   SendOrderOpenSignal(ticket);
                   AddTrackedOrder(ticket);
                }
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
   if(trans.type == TRADE_TRANSACTION_DEAL_ADD)
   {
      if(PositionSelectByTicket(trans.position))
      {
         string symbol = PositionGetString(POSITION_SYMBOL);
         if(MatchesSymbolFilter(symbol, g_symbol_prefix, g_symbol_suffix))
         {
            SendPositionOpenSignal(trans.position);
            AddTrackedPosition(trans.position);
         }
      }
   }
   else if(trans.type == TRADE_TRANSACTION_HISTORY_ADD)
   {
      // Position was closed
      if(trans.deal_type == DEAL_TYPE_BUY || trans.deal_type == DEAL_TYPE_SELL)
      {
         SendPositionCloseSignal(trans.position);
         RemoveTrackedPosition(trans.position);
      }
   }
   
   //--- Order Transactions (Pending Orders) ---
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
            if(MatchesSymbolFilter(symbol, g_symbol_prefix, g_symbol_suffix))
            {
               SendOrderOpenSignal(ticket);
               AddTrackedOrder(ticket);
            }
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
//| Check for closed positions                                        |
//+------------------------------------------------------------------+
void CheckForClosedPositions()
{
   for(int i = ArraySize(g_tracked_positions) - 1; i >= 0; i--)
   {
      ulong ticket = g_tracked_positions[i].ticket;
      if(!PositionSelectByTicket(ticket))
      {
         SendPositionCloseSignal(ticket);
         RemoveTrackedPosition(ticket);
      }
   }
}

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

   SendOpenSignal(g_zmq_socket, ticket, symbol, order_type,
                  volume, price, sl, tp, magic, comment, AccountID);
}

//+------------------------------------------------------------------+
//| Send close signal                                                 |
//+------------------------------------------------------------------+
void SendPositionCloseSignal(ulong ticket)
{
   SendCloseSignal(g_zmq_socket, ticket, AccountID);
}

//+------------------------------------------------------------------+
//| Send modify signal                                                |
//+------------------------------------------------------------------+
void SendPositionModifySignal(ulong ticket, double sl, double tp)
{
   SendModifySignal(g_zmq_socket, ticket, sl, tp, AccountID);
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
//| Add position to tracking list with current SL/TP                 |
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
}

//+------------------------------------------------------------------+
//| Add order to tracking list                                       |
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

   SendOpenSignal(g_zmq_socket, ticket, symbol, order_type,
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
    
    SendModifySignal(g_zmq_socket, ticket, sl, tp, AccountID);
}

//+------------------------------------------------------------------+
//| Send order close signal (delete)                                 |
//+------------------------------------------------------------------+
void SendOrderCloseSignal(ulong ticket)
{
   SendCloseSignal(g_zmq_socket, ticket, AccountID);
}

//+------------------------------------------------------------------+
//| Process Master configuration message (MessagePack)               |
//+------------------------------------------------------------------+
void ProcessMasterConfigMessage(uchar &msgpack_data[], int data_len)
{
   Print("=== Processing Master Configuration Message ===");

   // Parse MessagePack once and get a handle to the Master config structure
   HANDLE_TYPE config_handle = parse_master_config(msgpack_data, data_len);
   if(config_handle == 0)
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

   // Log configuration values
   Print("Account ID: ", config_account_id);
   Print("Symbol Prefix: ", (prefix == "" ? "(none)" : prefix));
   Print("Symbol Suffix: ", (suffix == "" ? "(none)" : suffix));
   Print("Config Version: ", version);

   // Update global configuration variables
   g_symbol_prefix = prefix;
   g_symbol_suffix = suffix;
   g_config_version = version;

   // Update configuration panel
   if(ShowConfigPanel)
   {
      g_config_panel.UpdateSymbolConfig(g_symbol_prefix, g_symbol_suffix, "");
   }

   // Free the config handle
   master_config_free(config_handle);

   Print("=== Master Configuration Updated ===");
}

