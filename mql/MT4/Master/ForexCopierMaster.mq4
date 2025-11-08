//+------------------------------------------------------------------+
//|                                       ForexCopierMaster.mq4      |
//|                        Copyright 2025, Forex Copier Project      |
//|                                                                  |
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, Forex Copier Project"
#property link      ""
#property version   "1.00"
#property strict

//--- Include common headers
#include <ForexCopierCommon.mqh>
#include <ForexCopierMessages.mqh>
#include <ForexCopierTrade.mqh>

//--- Input parameters
input string   ServerAddress = "tcp://localhost:5555";  // Server ZMQ address
input int      MagicFilter = 0;                         // Magic number filter (0 = all)
input int      ScanInterval = 100;                      // Scan interval in milliseconds

//--- Global variables
string      AccountID;                  // Auto-generated from broker + account number
int         g_zmq_context = -1;
int         g_zmq_socket = -1;
int         g_tracked_orders[];
bool        g_initialized = false;
datetime    g_last_heartbeat = 0;

//+------------------------------------------------------------------+
//| Expert initialization function                                     |
//+------------------------------------------------------------------+
int OnInit()
{
   Print("=== ForexCopier Master EA (MT4) Starting ===");

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

   // Send registration message
   SendRegistrationMessage(g_zmq_context, ServerAddress, AccountID, "Master", "MT4");

   // Scan existing orders
   ScanExistingOrders();

   g_initialized = true;
   Print("=== ForexCopier Master EA (MT4) Initialized ===");

   return INIT_SUCCEEDED;
}

//+------------------------------------------------------------------+
//| Expert deinitialization function                                  |
//+------------------------------------------------------------------+
void OnDeinit(const int reason)
{
   Print("=== ForexCopier Master EA (MT4) Stopping ===");

   // Send unregister message
   SendUnregistrationMessage(g_zmq_context, ServerAddress, AccountID);

   if(g_zmq_socket >= 0) zmq_socket_destroy(g_zmq_socket);
   if(g_zmq_context >= 0) zmq_context_destroy(g_zmq_context);

   Print("=== ForexCopier Master EA (MT4) Stopped ===");
}

//+------------------------------------------------------------------+
//| Expert tick function                                              |
//+------------------------------------------------------------------+
void OnTick()
{
   if(!g_initialized)
      return;

   // Send heartbeat every 30 seconds
   static datetime last_heartbeat = 0;
   if(TimeCurrent() - last_heartbeat >= 30)
   {
      SendHeartbeatMessage(g_zmq_context, ServerAddress, AccountID);
      last_heartbeat = TimeCurrent();
   }

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
//| Check for modified orders                                         |
//+------------------------------------------------------------------+
void CheckForModifiedOrders()
{
   // Placeholder for modify detection
}

//+------------------------------------------------------------------+
//| Check for closed orders                                           |
//+------------------------------------------------------------------+
void CheckForClosedOrders()
{
   // Check if any tracked order is no longer in open orders
   for(int i = ArraySize(g_tracked_orders) - 1; i >= 0; i--)
   {
      int ticket = g_tracked_orders[i];
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
//| Helper functions                                                  |
//+------------------------------------------------------------------+
bool IsOrderTracked(int ticket)
{
   for(int i = 0; i < ArraySize(g_tracked_orders); i++)
   {
      if(g_tracked_orders[i] == ticket)
         return true;
   }
   return false;
}

void AddTrackedOrder(int ticket)
{
   int size = ArraySize(g_tracked_orders);
   ArrayResize(g_tracked_orders, size + 1);
   g_tracked_orders[size] = ticket;
}

void RemoveTrackedOrder(int ticket)
{
   for(int i = 0; i < ArraySize(g_tracked_orders); i++)
   {
      if(g_tracked_orders[i] == ticket)
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
