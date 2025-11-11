//+------------------------------------------------------------------+
//|                                       SankeyCopierMaster.mq4      |
//|                        Copyright 2025, SANKEY Copier Project      |
//|                                                                  |
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, SANKEY Copier Project"
#property link      ""
#property version   "1.00"
#property strict

//--- Include common headers
#include <SankeyCopier/SankeyCopierCommon.mqh>
#include <SankeyCopier/SankeyCopierMessages.mqh>
#include <SankeyCopier/SankeyCopierTrade.mqh>

//--- Input parameters
input string   ServerAddress = "tcp://localhost:5555";  // Server ZMQ address
input int      MagicFilter = 0;                         // Magic number filter (0 = all)
input int      ScanInterval = 100;                      // Scan interval in milliseconds

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

//+------------------------------------------------------------------+
//| Expert initialization function                                     |
//+------------------------------------------------------------------+
int OnInit()
{
   Print("=== SankeyCopier Master EA (MT4) Starting ===");

   // Auto-generate AccountID from broker name and account number
   AccountID = GenerateAccountID();
   Print("Auto-generated AccountID: ", AccountID);

   Print("Server Address: ", ServerAddress);
   Print("Magic Filter: ", MagicFilter);

   g_zmq_context = zmq_context_create();
   if(g_zmq_context < 0)
   {
      Print("ERROR: Failed to create ZMQ context");
      return INIT_FAILED;
   }

   g_zmq_socket = zmq_socket_create(g_zmq_context, ZMQ_PUSH);
   if(g_zmq_socket < 0)
   {
      Print("ERROR: Failed to create ZMQ socket");
      zmq_context_destroy(g_zmq_context);
      return INIT_FAILED;
   }

   if(zmq_socket_connect(g_zmq_socket, ServerAddress) == 0)
   {
      Print("ERROR: Failed to connect to server");
      zmq_socket_destroy(g_zmq_socket);
      zmq_context_destroy(g_zmq_context);
      return INIT_FAILED;
   }

   Print("Connected to server successfully");

   // Scan existing orders
   ScanExistingOrders();

   // Set up timer for heartbeat (1 second interval)
   EventSetTimer(1);

   g_initialized = true;
   Print("=== SankeyCopier Master EA (MT4) Initialized ===");

   return INIT_SUCCEEDED;
}

//+------------------------------------------------------------------+
//| Expert deinitialization function                                  |
//+------------------------------------------------------------------+
void OnDeinit(const int reason)
{
   Print("=== SankeyCopier Master EA (MT4) Stopping ===");

   // Send unregister message
   SendUnregistrationMessage(g_zmq_context, ServerAddress, AccountID);

   // Kill timer
   EventKillTimer();

   if(g_zmq_socket >= 0) zmq_socket_destroy(g_zmq_socket);
   if(g_zmq_context >= 0) zmq_context_destroy(g_zmq_context);

   Print("=== SankeyCopier Master EA (MT4) Stopped ===");
}

//+------------------------------------------------------------------+
//| Timer function (called every 1 second)                            |
//+------------------------------------------------------------------+
void OnTimer()
{
   if(!g_initialized)
      return;

   // Send heartbeat every HEARTBEAT_INTERVAL_SECONDS
   // Use TimeLocal() instead of TimeCurrent() to ensure heartbeat works even when market is closed
   if(TimeLocal() - g_last_heartbeat >= HEARTBEAT_INTERVAL_SECONDS)
   {
      SendHeartbeatMessage(g_zmq_context, ServerAddress, AccountID, "Master", "MT4");
      g_last_heartbeat = TimeLocal();
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
            int ticket = OrderTicket();
            AddTrackedOrder(ticket);
            SendOpenSignalFromOrder(ticket);  // Send Open signal for existing orders
            Print("Tracking existing order: #", ticket);
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
            AddTrackedOrder(ticket);
            SendOpenSignalFromOrder(ticket);
            Print("New order detected: #", ticket, " ", OrderSymbol(), " ", OrderLots(), " lots");
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
   SendOpenSignal(g_zmq_socket, (TICKET_TYPE)ticket, OrderSymbol(),
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
