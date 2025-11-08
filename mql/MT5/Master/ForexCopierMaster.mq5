//+------------------------------------------------------------------+
//|                                       ForexCopierMaster.mq5      |
//|                        Copyright 2025, Forex Copier Project      |
//|                                                                  |
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, Forex Copier Project"
#property link      ""
#property version   "1.00"

//--- Include common headers
#include <ForexCopierCommon.mqh>
#include <ForexCopierMessages.mqh>
#include <ForexCopierTrade.mqh>

//--- Input parameters
input string   ServerAddress = "tcp://localhost:5555";
input ulong    MagicFilter = 0;
input int      ScanInterval = 100;

//--- Position tracking structure
struct PositionInfo
{
   ulong  ticket;
   double sl;
   double tp;
};

//--- Global variables
string        AccountID;                  // Auto-generated from broker + account number
int           g_zmq_context = -1;
int           g_zmq_socket = -1;
PositionInfo  g_tracked_positions[];
bool          g_initialized = false;
datetime      g_last_heartbeat = 0;

//+------------------------------------------------------------------+
//| Expert initialization function                                     |
//+------------------------------------------------------------------+
int OnInit()
{
   Print("=== ForexCopier Master EA (MT5) Starting ===");

   // Auto-generate AccountID from broker name and account number
   AccountID = GenerateAccountID();
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
   SendRegistrationMessage(g_zmq_context, ServerAddress, AccountID, "Master", "MT5");

   Print("=== ForexCopier Master EA (MT5) Initialized ===");
   return INIT_SUCCEEDED;
}

//+------------------------------------------------------------------+
//| Expert deinitialization function                                  |
//+------------------------------------------------------------------+
void OnDeinit(const int reason)
{
   // Send unregister message to server
   SendUnregistrationMessage(g_zmq_context, ServerAddress, AccountID);

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

   // Send heartbeat every HEARTBEAT_INTERVAL_SECONDS
   if(TimeCurrent() - g_last_heartbeat >= HEARTBEAT_INTERVAL_SECONDS)
   {
      SendHeartbeatMessage(g_zmq_context, ServerAddress, AccountID);
      g_last_heartbeat = TimeCurrent();
   }

   static datetime last_scan = 0;
   if(TimeCurrent() - last_scan > ScanInterval / 1000)
   {
      CheckForNewPositions();
      CheckForModifiedPositions();
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
         SendPositionOpenSignal(trans.position);
      }
   }
   else if(trans.type == TRADE_TRANSACTION_HISTORY_ADD)
   {
      // Position was closed
      if(trans.deal_type == DEAL_TYPE_BUY || trans.deal_type == DEAL_TYPE_SELL)
      {
         SendPositionCloseSignal(trans.position);
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
         if(MagicFilter != 0 && PositionGetInteger(POSITION_MAGIC) != MagicFilter)
            continue;

         if(!IsPositionTracked(ticket))
         {
            AddTrackedPosition(ticket);
            SendPositionOpenSignal(ticket);
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

   string symbol = PositionGetString(POSITION_SYMBOL);
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

   int size = ArraySize(g_tracked_positions);
   ArrayResize(g_tracked_positions, size + 1);
   g_tracked_positions[size].ticket = ticket;
   g_tracked_positions[size].sl = PositionGetDouble(POSITION_SL);
   g_tracked_positions[size].tp = PositionGetDouble(POSITION_TP);
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

