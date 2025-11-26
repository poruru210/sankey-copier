//+------------------------------------------------------------------+
//|                                  SankeyCopierMasterSignals.mqh   |
//|                        Copyright 2025, SANKEY Copier Project      |
//|                     Master EA specific signal functions           |
//+------------------------------------------------------------------+
// Purpose: Contains functions for Master EA to send trade signals
// Why: Separates Master-specific code from common shared code
//      to improve code organization and maintainability
#property copyright "Copyright 2025, SANKEY Copier Project"
#property strict

#ifndef SANKEY_COPIER_MASTER_SIGNALS_MQH
#define SANKEY_COPIER_MASTER_SIGNALS_MQH

#include "Common.mqh"

//+------------------------------------------------------------------+
//| Send open position signal message (Master)                       |
//| Called when Master EA opens a new position to notify Slaves      |
//+------------------------------------------------------------------+
bool SendOpenSignal(HANDLE_TYPE zmq_socket, TICKET_TYPE ticket, string symbol,
                    string order_type, double lots, double price, double sl, double tp,
                    long magic, string comment, string account_id)
{
   // Serialize open signal message using MessagePack
   int len = serialize_trade_signal("Open", (long)ticket, symbol, order_type,
                                            lots, price, sl, tp, magic, comment,
                                            FormatTimestampISO8601(TimeGMT()), account_id);

   if(len <= 0)
   {
      Print("ERROR: Failed to serialize open signal message");
      return false;
   }

   // Copy serialized data to buffer
   uchar buffer[];
   ArrayResize(buffer, len);
   int copied = copy_serialized_buffer(buffer, len);

   if(copied != len)
   {
      Print("ERROR: Failed to copy open signal message buffer");
      return false;
   }

   // Send binary MessagePack data
   return (zmq_socket_send_binary(zmq_socket, buffer, len) == 1);
}

//+------------------------------------------------------------------+
//| Send close signal message (Master)                               |
//| Called when Master EA closes a position to notify Slaves         |
//+------------------------------------------------------------------+
bool SendCloseSignal(HANDLE_TYPE zmq_socket, TICKET_TYPE ticket, string account_id)
{
   // For close signals, we send a trade signal with action="Close"
   // Only ticket, timestamp, and source_account are needed
   int len = serialize_trade_signal("Close", (long)ticket, "", "", 0.0, 0.0, 0.0, 0.0,
                                            0, "", FormatTimestampISO8601(TimeGMT()), account_id);

   if(len <= 0)
   {
      Print("ERROR: Failed to serialize close signal message");
      return false;
   }

   // Copy serialized data to buffer
   uchar buffer[];
   ArrayResize(buffer, len);
   int copied = copy_serialized_buffer(buffer, len);

   if(copied != len)
   {
      Print("ERROR: Failed to copy close signal message buffer");
      return false;
   }

   // Send binary MessagePack data
   return (zmq_socket_send_binary(zmq_socket, buffer, len) == 1);
}

//+------------------------------------------------------------------+
//| Send modify signal message (Master)                             |
//| Called when Master EA modifies SL/TP to notify Slaves            |
//+------------------------------------------------------------------+
bool SendModifySignal(HANDLE_TYPE zmq_socket, TICKET_TYPE ticket, double sl, double tp, string account_id)
{
   // For modify signals, we send a trade signal with action="Modify"
   // Only ticket, stop_loss, take_profit, timestamp, and source_account are needed
   int len = serialize_trade_signal("Modify", (long)ticket, "", "", 0.0, 0.0, sl, tp,
                                            0, "", FormatTimestampISO8601(TimeGMT()), account_id);

   if(len <= 0)
   {
      Print("ERROR: Failed to serialize modify signal message");
      return false;
   }

   // Copy serialized data to buffer
   uchar buffer[];
   ArrayResize(buffer, len);
   int copied = copy_serialized_buffer(buffer, len);

   if(copied != len)
   {
      Print("ERROR: Failed to copy modify signal message buffer");
      return false;
   }

   // Send binary MessagePack data
   return (zmq_socket_send_binary(zmq_socket, buffer, len) == 1);
}

#endif // SANKEY_COPIER_MASTER_SIGNALS_MQH
