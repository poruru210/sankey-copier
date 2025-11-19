//+------------------------------------------------------------------+
//|                                     SankeyCopierMessages.mqh      |
//|                        Copyright 2025, SANKEY Copier Project      |
//|                     Message sending utilities                     |
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, SANKEY Copier Project"

#include "Common.mqh"

//+------------------------------------------------------------------+
//| Send configuration request message to server (for Slave EAs)     |
//+------------------------------------------------------------------+
bool SendRequestConfigMessage(HANDLE_TYPE zmq_context, string server_address, string account_id, string ea_type)
{
   // Create temporary PUSH socket for config request
   HANDLE_TYPE push_socket = zmq_socket_create(zmq_context, ZMQ_PUSH);
   if(push_socket < 0)
   {
      Print("ERROR: Failed to create request config socket");
      return false;
   }

   if(zmq_socket_connect(push_socket, server_address) == 0)
   {
      Print("ERROR: Failed to connect to request config server: ", server_address);
      zmq_socket_destroy(push_socket);
      return false;
   }

   // Serialize request config message using MessagePack
   int len = serialize_request_config("RequestConfig", account_id,
                                      FormatTimestampISO8601(TimeCurrent()), ea_type);

   if(len <= 0)
   {
      Print("ERROR: Failed to serialize request config message");
      zmq_socket_destroy(push_socket);
      return false;
   }

   // Copy serialized data to buffer
   uchar buffer[];
   ArrayResize(buffer, len);
   int copied = copy_serialized_buffer(buffer, len);

   if(copied != len)
   {
      Print("ERROR: Failed to copy request config message buffer");
      zmq_socket_destroy(push_socket);
      return false;
   }

   // Send binary MessagePack data
   bool success = (zmq_socket_send_binary(push_socket, buffer, len) == 1);

   if(success)
      Print("RequestConfig message sent successfully");
   else
      Print("ERROR: Failed to send request config message");

   zmq_socket_destroy(push_socket);
   return success;
}


//+------------------------------------------------------------------+
//| Send unregistration message to server                            |
//+------------------------------------------------------------------+
bool SendUnregistrationMessage(HANDLE_TYPE zmq_context, string server_address, string account_id)
{
   // Create temporary PUSH socket for unregistration
   HANDLE_TYPE push_socket = zmq_socket_create(zmq_context, ZMQ_PUSH);
   if(push_socket < 0)
   {
      Print("ERROR: Failed to create unregistration socket");
      return false;
   }

   if(zmq_socket_connect(push_socket, server_address) == 0)
   {
      Print("ERROR: Failed to connect to unregistration server: ", server_address);
      zmq_socket_destroy(push_socket);
      return false;
   }

   // Serialize unregistration message using MessagePack
   int len = serialize_unregister("Unregister", account_id,
                                          FormatTimestampISO8601(TimeCurrent()));

   if(len <= 0)
   {
      Print("ERROR: Failed to serialize unregistration message");
      zmq_socket_destroy(push_socket);
      return false;
   }

   // Copy serialized data to buffer
   uchar buffer[];
   ArrayResize(buffer, len);
   int copied = copy_serialized_buffer(buffer, len);

   if(copied != len)
   {
      Print("ERROR: Failed to copy unregistration message buffer");
      zmq_socket_destroy(push_socket);
      return false;
   }

   // Send binary MessagePack data
   bool success = (zmq_socket_send_binary(push_socket, buffer, len) == 1);

   if(success)
      Print("Unregistration message sent successfully");
   else
      Print("ERROR: Failed to send unregistration message");

   zmq_socket_destroy(push_socket);
   return success;
}

//+------------------------------------------------------------------+
//| Send heartbeat message to server (includes EA info for auto-registration) |
//+------------------------------------------------------------------+
bool SendHeartbeatMessage(HANDLE_TYPE zmq_context, string server_address, string account_id, string ea_type, string platform)
{
   // Create temporary PUSH socket for heartbeat
   HANDLE_TYPE push_socket = zmq_socket_create(zmq_context, ZMQ_PUSH);
   if(push_socket < 0)
   {
      Print("ERROR: Failed to create heartbeat socket");
      return false;
   }

   if(zmq_socket_connect(push_socket, server_address) == 0)
   {
      Print("ERROR: Failed to connect to heartbeat server: ", server_address);
      zmq_socket_destroy(push_socket);
      return false;
   }

   // Get current auto-trading state (TERMINAL_TRADE_ALLOWED)
   int is_trade_allowed = (int)TerminalInfoInteger(TERMINAL_TRADE_ALLOWED);

   // Serialize heartbeat message using MessagePack (includes EA info for auto-registration)
   int len = serialize_heartbeat("Heartbeat", account_id, GetAccountBalance(),
                                 GetAccountEquity(), GetOpenPositionsCount(),
                                 FormatTimestampISO8601(TimeCurrent()),
                                 ea_type, platform,
                                 GetAccountNumber(), GetBrokerName(), GetAccountName(),
                                 GetServerName(), GetAccountCurrency(), GetAccountLeverage(),
                                 is_trade_allowed);

   if(len <= 0)
   {
      Print("ERROR: Failed to serialize heartbeat message");
      zmq_socket_destroy(push_socket);
      return false;
   }

   // Copy serialized data to buffer
   uchar buffer[];
   ArrayResize(buffer, len);
   int copied = copy_serialized_buffer(buffer, len);

   if(copied != len)
   {
      Print("ERROR: Failed to copy heartbeat message buffer");
      zmq_socket_destroy(push_socket);
      return false;
   }

   // Send binary MessagePack data
   bool success = (zmq_socket_send_binary(push_socket, buffer, len) == 1);

   zmq_socket_destroy(push_socket);
   return success;
}

//+------------------------------------------------------------------+
//| Send open position signal message (Master)                       |
//+------------------------------------------------------------------+
bool SendOpenSignal(HANDLE_TYPE zmq_socket, TICKET_TYPE ticket, string symbol,
                    string order_type, double lots, double price, double sl, double tp,
                    long magic, string comment, string account_id)
{
   // Serialize open signal message using MessagePack
   int len = serialize_trade_signal("Open", (long)ticket, symbol, order_type,
                                            lots, price, sl, tp, magic, comment,
                                            FormatTimestampISO8601(TimeCurrent()), account_id);

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
//+------------------------------------------------------------------+
bool SendCloseSignal(HANDLE_TYPE zmq_socket, TICKET_TYPE ticket, string account_id)
{
   // For close signals, we send a trade signal with action="Close"
   // Only ticket, timestamp, and source_account are needed
   int len = serialize_trade_signal("Close", (long)ticket, "", "", 0.0, 0.0, 0.0, 0.0,
                                            0, "", FormatTimestampISO8601(TimeCurrent()), account_id);

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
//+------------------------------------------------------------------+
bool SendModifySignal(HANDLE_TYPE zmq_socket, TICKET_TYPE ticket, double sl, double tp, string account_id)
{
   // For modify signals, we send a trade signal with action="Modify"
   // Only ticket, stop_loss, take_profit, timestamp, and source_account are needed
   int len = serialize_trade_signal("Modify", (long)ticket, "", "", 0.0, 0.0, sl, tp,
                                            0, "", FormatTimestampISO8601(TimeCurrent()), account_id);

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
