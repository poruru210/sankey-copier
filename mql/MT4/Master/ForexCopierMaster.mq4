//+------------------------------------------------------------------+
//|                                       ForexCopierMaster.mq4      |
//|                        Copyright 2025, Forex Copier Project      |
//|                                                                  |
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, Forex Copier Project"
#property link      ""
#property version   "1.00"
#property strict

//--- Import Rust ZeroMQ DLL
#import "forex_copier_zmq.dll"
   int    zmq_context_create();
   void   zmq_context_destroy(int context);
   int    zmq_socket_create(int context, int socket_type);
   void   zmq_socket_destroy(int socket);
   int    zmq_socket_connect(int socket, string address);
   int    zmq_socket_send(int socket, string message);
#import

//--- ZeroMQ socket types
#define ZMQ_PUSH 8

//--- Input parameters
input string   ServerAddress = "tcp://localhost:5555";  // Server ZMQ address
input string   AccountID = "MASTER_001";                // Master account identifier
input int      MagicFilter = 0;                         // Magic number filter (0 = all)
input int      ScanInterval = 100;                      // Scan interval in milliseconds

//--- Global variables
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
   Print("Server Address: ", ServerAddress);
   Print("Account ID: ", AccountID);
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
   SendRegisterMessage();

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
   SendUnregisterMessage();

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
      SendHeartbeat();
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
            SendTradeSignal("Open", ticket);
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
   for(int i = 0; i < OrdersTotal(); i++)
   {
      if(OrderSelect(i, SELECT_BY_POS, MODE_TRADES))
      {
         int ticket = OrderTicket();

         if(IsOrderTracked(ticket))
         {
            // Check if SL/TP modified (simplified check)
            // In production, you'd want to track previous values
            // For now, we'll send a modify signal
            // SendTradeSignal("Modify", ticket);
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
            SendTradeSignal("Close", ticket);
            RemoveTrackedOrder(ticket);
            Print("Order closed: #", ticket);
         }
      }
   }
}

//+------------------------------------------------------------------+
//| Send trade signal to server                                       |
//+------------------------------------------------------------------+
void SendTradeSignal(string action, int ticket)
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
   string timestamp = TimeToString(TimeCurrent(), TIME_DATE|TIME_SECONDS);

   // Build JSON message
   string json = "{";
   json += "\"action\":\"" + action + "\",";
   json += "\"ticket\":" + IntegerToString(ticket) + ",";
   json += "\"symbol\":\"" + OrderSymbol() + "\",";
   json += "\"order_type\":\"" + order_type + "\",";
   json += "\"lots\":" + DoubleToString(OrderLots(), 2) + ",";
   json += "\"open_price\":" + DoubleToString(OrderOpenPrice(), 5) + ",";
   json += "\"stop_loss\":" + (OrderStopLoss() > 0 ? DoubleToString(OrderStopLoss(), 5) : "null") + ",";
   json += "\"take_profit\":" + (OrderTakeProfit() > 0 ? DoubleToString(OrderTakeProfit(), 5) : "null") + ",";
   json += "\"magic_number\":" + IntegerToString(OrderMagicNumber()) + ",";
   json += "\"comment\":\"" + OrderComment() + "\",";
   json += "\"timestamp\":\"" + timestamp + "\",";
   json += "\"source_account\":\"" + AccountID + "\"";
   json += "}";

   if(zmq_socket_send(g_zmq_socket, json) == 1)
   {
      Print("Sent ", action, " signal for order #", ticket);
   }
   else
   {
      Print("ERROR: Failed to send signal for order #", ticket);
   }
}

//+------------------------------------------------------------------+
//| Get order type as string                                          |
//+------------------------------------------------------------------+
string GetOrderTypeString(int type)
{
   switch(type)
   {
      case OP_BUY:       return "Buy";
      case OP_SELL:      return "Sell";
      case OP_BUYLIMIT:  return "BuyLimit";
      case OP_SELLLIMIT: return "SellLimit";
      case OP_BUYSTOP:   return "BuyStop";
      case OP_SELLSTOP:  return "SellStop";
      default:           return "Unknown";
   }
}

//+------------------------------------------------------------------+
//| Check if order is tracked                                         |
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

//+------------------------------------------------------------------+
//| Add order to tracking list                                        |
//+------------------------------------------------------------------+
void AddTrackedOrder(int ticket)
{
   int size = ArraySize(g_tracked_orders);
   ArrayResize(g_tracked_orders, size + 1);
   g_tracked_orders[size] = ticket;
}

//+------------------------------------------------------------------+
//| Remove order from tracking list                                   |
//+------------------------------------------------------------------+
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

//+------------------------------------------------------------------+
//| Send EA registration message                                      |
//+------------------------------------------------------------------+
void SendRegisterMessage()
{
   string timestamp = TimeToString(TimeCurrent(), TIME_DATE|TIME_SECONDS);

   string json = "{";
   json += "\"message_type\":\"Register\",";
   json += "\"account_id\":\"" + AccountID + "\",";
   json += "\"ea_type\":\"Master\",";
   json += "\"platform\":\"MT4\",";
   json += "\"account_number\":" + IntegerToString(AccountNumber()) + ",";
   json += "\"broker\":\"" + AccountCompany() + "\",";
   json += "\"account_name\":\"" + AccountName() + "\",";
   json += "\"server\":\"" + AccountServer() + "\",";
   json += "\"balance\":" + DoubleToString(AccountBalance(), 2) + ",";
   json += "\"equity\":" + DoubleToString(AccountEquity(), 2) + ",";
   json += "\"currency\":\"" + AccountCurrency() + "\",";
   json += "\"leverage\":" + IntegerToString(AccountLeverage()) + ",";
   json += "\"timestamp\":\"" + timestamp + "\"";
   json += "}";

   if(zmq_socket_send(g_zmq_socket, json) == 1)
   {
      Print("EA Registration sent successfully");
   }
   else
   {
      Print("ERROR: Failed to send registration message");
   }
}

//+------------------------------------------------------------------+
//| Send unregistration message                                       |
//+------------------------------------------------------------------+
void SendUnregisterMessage()
{
   string timestamp = TimeToString(TimeCurrent(), TIME_DATE|TIME_SECONDS);

   string json = "{";
   json += "\"message_type\":\"Unregister\",";
   json += "\"account_id\":\"" + AccountID + "\",";
   json += "\"timestamp\":\"" + timestamp + "\"";
   json += "}";

   if(zmq_socket_send(g_zmq_socket, json) == 1)
   {
      Print("Unregistration sent successfully");
   }
   else
   {
      Print("ERROR: Failed to send unregistration message");
   }
}

//+------------------------------------------------------------------+
//| Send heartbeat message                                            |
//+------------------------------------------------------------------+
void SendHeartbeat()
{
   string timestamp = TimeToString(TimeCurrent(), TIME_DATE|TIME_SECONDS);
   int open_positions = 0;

   // Count open positions
   for(int i = 0; i < OrdersTotal(); i++)
   {
      if(OrderSelect(i, SELECT_BY_POS, MODE_TRADES))
      {
         open_positions++;
      }
   }

   string json = "{";
   json += "\"message_type\":\"Heartbeat\",";
   json += "\"account_id\":\"" + AccountID + "\",";
   json += "\"balance\":" + DoubleToString(AccountBalance(), 2) + ",";
   json += "\"equity\":" + DoubleToString(AccountEquity(), 2) + ",";
   json += "\"open_positions\":" + IntegerToString(open_positions) + ",";
   json += "\"timestamp\":\"" + timestamp + "\"";
   json += "}";

   if(zmq_socket_send(g_zmq_socket, json) == 1)
   {
      // Silent success (don't clutter logs)
   }
   else
   {
      Print("ERROR: Failed to send heartbeat");
   }
}
