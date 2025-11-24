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

//--- Input parameters
input string   RelayServerAddress = DEFAULT_ADDR_PULL;  // Server ZMQ address (PULL)
input int      MagicFilter = 0;                         // Magic number filter (0 = all)
input string   SymbolPrefix = "";                       // Symbol prefix to filter and strip (e.g. "pro.")
input string   SymbolSuffix = "";                       // Symbol suffix to filter and strip (e.g. ".m")
input int      ScanInterval = 100;                      // Scan interval in milliseconds
input bool     ShowConfigPanel = true;                  // Show configuration panel on chart
input int      PanelWidth = 280;                        // Configuration panel width (pixels)

//--- Order tracking structure
struct OrderInfo
{
   int    ticket;
   double sl;
   double tp;
};

//--- Global variables
string      AccountID;                  // Auto-generated from broker + account number
HANDLE_TYPE g_zmq_context = -1;
HANDLE_TYPE g_zmq_socket = -1;
OrderInfo   g_tracked_orders[];
bool        g_initialized = false;
datetime    g_last_heartbeat = 0;
bool        g_last_trade_allowed = false; // Track auto-trading state for change detection

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

   Print("Relay Server Address: ", RelayServerAddress);
   Print("Magic Filter: ", MagicFilter);

   // Initialize ZMQ context
   g_zmq_context = InitializeZmqContext();
   if(g_zmq_context < 0)
      return INIT_FAILED;

   // Create and connect PUSH socket
   g_zmq_socket = CreateAndConnectZmqSocket(g_zmq_context, ZMQ_PUSH, RelayServerAddress, "Master PUSH");
   if(g_zmq_socket < 0)
   {
      CleanupZmqContext(g_zmq_context);
      return INIT_FAILED;
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

      g_config_panel.UpdateServerRow(RelayServerAddress);
      g_config_panel.UpdateMagicFilterRow(MagicFilter);
      g_config_panel.UpdateTrackedOrdersRow(ArraySize(g_tracked_orders));
      g_config_panel.UpdateSymbolConfig(SymbolPrefix, SymbolSuffix, "");
   }

   g_initialized = true;
   Print("=== SankeyCopier Master EA (MT4) Initialized ===");

   ChartRedraw();
   return INIT_SUCCEEDED;
}

//+------------------------------------------------------------------+
//| Expert deinitialization function                                  |
//+------------------------------------------------------------------+
void OnDeinit(const int reason)
{
   Print("=== SankeyCopier Master EA (MT4) Stopping ===");

   // Send unregister message
   SendUnregistrationMessage(g_zmq_context, RelayServerAddress, AccountID);

   // Kill timer
   EventKillTimer();

   // Delete configuration panel
   if(ShowConfigPanel)
      g_config_panel.Delete();

   // Cleanup ZMQ resources
   CleanupZmqResources(g_zmq_socket, g_zmq_context, "Master PUSH");

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
      SendHeartbeatMessage(g_zmq_context, RelayServerAddress, AccountID, "Master", "MT4", SymbolPrefix, SymbolSuffix, "");
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

   for(int i = 0; i < OrdersTotal(); i++)
   {
      if(OrderSelect(i, SELECT_BY_POS, MODE_TRADES))
      {
         if(MagicFilter == 0 || OrderMagicNumber() == MagicFilter)
         {
            string symbol = OrderSymbol();
            if(MatchesSymbolFilter(symbol, SymbolPrefix, SymbolSuffix))
            {
               int ticket = OrderTicket();
               AddTrackedOrder(ticket);
               SendOpenSignalFromOrder(ticket);  // Send Open signal for existing orders
               Print("Tracking existing order: #", ticket);
            }
         }
      }
   }

   Print("Found ", ArraySize(g_tracked_orders), " existing orders");
}

//+------------------------------------------------------------------+
//| Check for new orders                                              |
//+------------------------------------------------------------------+
void CheckForNewOrders()
{
   for(int i = 0; i < OrdersTotal(); i++)
   {
      if(OrderSelect(i, SELECT_BY_POS, MODE_TRADES))
      {
         int ticket = OrderTicket();

         if(MagicFilter != 0 && OrderMagicNumber() != MagicFilter)
            continue;

         if(!IsOrderTracked(ticket))
         {
            string symbol = OrderSymbol();
            if(MatchesSymbolFilter(symbol, SymbolPrefix, SymbolSuffix))
            {
               AddTrackedOrder(ticket);
               SendOpenSignalFromOrder(ticket);
               Print("New order detected: #", ticket, " ", symbol, " ", OrderLots(), " lots");
            }
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

            // Update tracked values
            g_tracked_orders[i].sl = current_sl;
            g_tracked_orders[i].tp = current_tp;
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
         // Order was closed
         if(OrderSelect(ticket, SELECT_BY_TICKET, MODE_HISTORY))
         {
            SendCloseSignal(g_zmq_socket, (TICKET_TYPE)ticket, AccountID);
            RemoveTrackedOrder(ticket);
            Print("Order closed: #", ticket);
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
   string symbol = GetCleanSymbol(raw_symbol, SymbolPrefix, SymbolSuffix);
   
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
//| Add order to tracking list with current SL/TP                    |
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
