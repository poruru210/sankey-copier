//+------------------------------------------------------------------+
//|                                       ForexCopierMaster.mq5      |
//|                        Copyright 2025, Forex Copier Project      |
//|                                                                  |
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, Forex Copier Project"
#property link      ""
#property version   "1.00"

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
input string   ServerAddress = "tcp://localhost:5555";
input ulong    MagicFilter = 0;
input int      ScanInterval = 100;

//--- Global variables
string      AccountID;                  // Auto-generated from broker + account number
int         g_zmq_context = -1;
int         g_zmq_socket = -1;
ulong       g_tracked_positions[];
bool        g_initialized = false;
datetime    g_last_heartbeat = 0;

//+------------------------------------------------------------------+
//| Expert initialization function                                     |
//+------------------------------------------------------------------+
int OnInit()
{
   Print("=== ForexCopier Master EA (MT5) Starting ===");

   // Auto-generate AccountID from broker name and account number
   string broker = AccountInfoString(ACCOUNT_COMPANY);
   long account_number = AccountInfoInteger(ACCOUNT_LOGIN);

   // Replace spaces and special characters with underscores
   StringReplace(broker, " ", "_");
   StringReplace(broker, ".", "_");
   StringReplace(broker, "-", "_");

   // Format: broker_accountnumber
   AccountID = broker + "_" + IntegerToString(account_number);
   Print("Auto-generated AccountID: ", AccountID);

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

   ScanExistingPositions();
   g_initialized = true;

   // Send registration message to server
   SendRegisterMessage();

   Print("=== ForexCopier Master EA (MT5) Initialized ===");
   return INIT_SUCCEEDED;
}

//+------------------------------------------------------------------+
//| Expert deinitialization function                                  |
//+------------------------------------------------------------------+
void OnDeinit(const int reason)
{
   // Send unregister message to server
   SendUnregisterMessage();

   if(g_zmq_socket >= 0) zmq_socket_destroy(g_zmq_socket);
   if(g_zmq_context >= 0) zmq_context_destroy(g_zmq_context);

   Print("=== ForexCopier Master EA (MT5) Stopped ===");
}

//+------------------------------------------------------------------+
//| Expert tick function                                              |
//+------------------------------------------------------------------+
void OnTick()
{
   if(!g_initialized) return;

   // Send heartbeat every 30 seconds
   if(TimeCurrent() - g_last_heartbeat >= 30)
   {
      SendHeartbeat();
      g_last_heartbeat = TimeCurrent();
   }

   static datetime last_scan = 0;
   if(TimeCurrent() - last_scan > ScanInterval / 1000)
   {
      CheckForNewPositions();
      CheckForClosedPositions();
      last_scan = TimeCurrent();
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
         SendPositionSignal("Open", trans.position);
      }
   }
   else if(trans.type == TRADE_TRANSACTION_HISTORY_ADD)
   {
      // Position was closed
      if(trans.deal_type == DEAL_TYPE_BUY || trans.deal_type == DEAL_TYPE_SELL)
      {
         SendCloseSignal(trans.position);
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
         if(MagicFilter == 0 || PositionGetInteger(POSITION_MAGIC) == MagicFilter)
         {
            AddTrackedPosition(ticket);
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
         if(MagicFilter != 0 && PositionGetInteger(POSITION_MAGIC) != MagicFilter)
            continue;

         if(!IsPositionTracked(ticket))
         {
            AddTrackedPosition(ticket);
            SendPositionSignal("Open", ticket);
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
      ulong ticket = g_tracked_positions[i];
      if(!PositionSelectByTicket(ticket))
      {
         SendCloseSignal(ticket);
         RemoveTrackedPosition(ticket);
      }
   }
}

//+------------------------------------------------------------------+
//| Send position signal                                              |
//+------------------------------------------------------------------+
void SendPositionSignal(string action, ulong ticket)
{
   if(!PositionSelectByTicket(ticket))
      return;

   string symbol = PositionGetString(POSITION_SYMBOL);
   long type = PositionGetInteger(POSITION_TYPE);
   double volume = PositionGetDouble(POSITION_VOLUME);
   double price = PositionGetDouble(POSITION_PRICE_OPEN);
   double sl = PositionGetDouble(POSITION_SL);
   double tp = PositionGetDouble(POSITION_TP);
   long magic = PositionGetInteger(POSITION_MAGIC);

   string order_type = (type == POSITION_TYPE_BUY) ? "Buy" : "Sell";
   string timestamp = TimeToString(TimeCurrent(), TIME_DATE|TIME_SECONDS);

   string json = "{";
   json += "\"action\":\"" + action + "\",";
   json += "\"ticket\":" + IntegerToString(ticket) + ",";
   json += "\"symbol\":\"" + symbol + "\",";
   json += "\"order_type\":\"" + order_type + "\",";
   json += "\"lots\":" + DoubleToString(volume, 2) + ",";
   json += "\"open_price\":" + DoubleToString(price, _Digits) + ",";
   json += "\"stop_loss\":" + ((sl > 0) ? DoubleToString(sl, _Digits) : "null") + ",";
   json += "\"take_profit\":" + ((tp > 0) ? DoubleToString(tp, _Digits) : "null") + ",";
   json += "\"magic_number\":" + IntegerToString(magic) + ",";
   json += "\"comment\":\"" + PositionGetString(POSITION_COMMENT) + "\",";
   json += "\"timestamp\":\"" + timestamp + "\",";
   json += "\"source_account\":\"" + AccountID + "\"";
   json += "}";

   zmq_socket_send(g_zmq_socket, json);
}

//+------------------------------------------------------------------+
//| Send close signal                                                 |
//+------------------------------------------------------------------+
void SendCloseSignal(ulong ticket)
{
   string json = "{";
   json += "\"action\":\"Close\",";
   json += "\"ticket\":" + IntegerToString(ticket) + ",";
   json += "\"timestamp\":\"" + TimeToString(TimeCurrent(), TIME_DATE|TIME_SECONDS) + "\",";
   json += "\"source_account\":\"" + AccountID + "\"";
   json += "}";

   zmq_socket_send(g_zmq_socket, json);
}

//+------------------------------------------------------------------+
//| Helper functions                                                  |
//+------------------------------------------------------------------+
bool IsPositionTracked(ulong ticket)
{
   for(int i = 0; i < ArraySize(g_tracked_positions); i++)
      if(g_tracked_positions[i] == ticket) return true;
   return false;
}

void AddTrackedPosition(ulong ticket)
{
   int size = ArraySize(g_tracked_positions);
   ArrayResize(g_tracked_positions, size + 1);
   g_tracked_positions[size] = ticket;
}

void RemoveTrackedPosition(ulong ticket)
{
   for(int i = 0; i < ArraySize(g_tracked_positions); i++)
   {
      if(g_tracked_positions[i] == ticket)
      {
         for(int j = i; j < ArraySize(g_tracked_positions) - 1; j++)
            g_tracked_positions[j] = g_tracked_positions[j + 1];
         ArrayResize(g_tracked_positions, ArraySize(g_tracked_positions) - 1);
         break;
      }
   }
}

//+------------------------------------------------------------------+
//| Send registration message to server                              |
//+------------------------------------------------------------------+
void SendRegisterMessage()
{
   // Get current timestamp in ISO 8601 format
   string timestamp = TimeToString(TimeCurrent(), TIME_DATE | TIME_SECONDS);
   StringReplace(timestamp, ".", "-");
   StringReplace(timestamp, " ", "T");
   timestamp += "Z";

   // Build JSON message
   string json = "{";
   json += "\"message_type\":\"Register\",";
   json += "\"account_id\":\"" + AccountID + "\",";
   json += "\"ea_type\":\"Master\",";
   json += "\"platform\":\"MT5\",";
   json += "\"account_number\":" + IntegerToString(AccountInfoInteger(ACCOUNT_LOGIN)) + ",";
   json += "\"broker\":\"" + AccountInfoString(ACCOUNT_COMPANY) + "\",";
   json += "\"account_name\":\"" + AccountInfoString(ACCOUNT_NAME) + "\",";
   json += "\"server\":\"" + AccountInfoString(ACCOUNT_SERVER) + "\",";
   json += "\"balance\":" + DoubleToString(AccountInfoDouble(ACCOUNT_BALANCE), 2) + ",";
   json += "\"equity\":" + DoubleToString(AccountInfoDouble(ACCOUNT_EQUITY), 2) + ",";
   json += "\"currency\":\"" + AccountInfoString(ACCOUNT_CURRENCY) + "\",";
   json += "\"leverage\":" + IntegerToString(AccountInfoInteger(ACCOUNT_LEVERAGE)) + ",";
   json += "\"timestamp\":\"" + timestamp + "\"";
   json += "}";

   zmq_socket_send(g_zmq_socket, json);
   Print("Registration message sent to server");
}

//+------------------------------------------------------------------+
//| Send unregister message to server                                |
//+------------------------------------------------------------------+
void SendUnregisterMessage()
{
   // Get current timestamp
   string timestamp = TimeToString(TimeCurrent(), TIME_DATE | TIME_SECONDS);
   StringReplace(timestamp, ".", "-");
   StringReplace(timestamp, " ", "T");
   timestamp += "Z";

   string json = "{";
   json += "\"message_type\":\"Unregister\",";
   json += "\"account_id\":\"" + AccountID + "\",";
   json += "\"timestamp\":\"" + timestamp + "\"";
   json += "}";

   zmq_socket_send(g_zmq_socket, json);
   Print("Unregister message sent to server");
}

//+------------------------------------------------------------------+
//| Send heartbeat message to server                                 |
//+------------------------------------------------------------------+
void SendHeartbeat()
{
   // Get current timestamp
   string timestamp = TimeToString(TimeCurrent(), TIME_DATE | TIME_SECONDS);
   StringReplace(timestamp, ".", "-");
   StringReplace(timestamp, " ", "T");
   timestamp += "Z";

   // Count open positions
   int open_positions = PositionsTotal();

   string json = "{";
   json += "\"message_type\":\"Heartbeat\",";
   json += "\"account_id\":\"" + AccountID + "\",";
   json += "\"balance\":" + DoubleToString(AccountInfoDouble(ACCOUNT_BALANCE), 2) + ",";
   json += "\"equity\":" + DoubleToString(AccountInfoDouble(ACCOUNT_EQUITY), 2) + ",";
   json += "\"open_positions\":" + IntegerToString(open_positions) + ",";
   json += "\"timestamp\":\"" + timestamp + "\"";
   json += "}";

   zmq_socket_send(g_zmq_socket, json);
}
