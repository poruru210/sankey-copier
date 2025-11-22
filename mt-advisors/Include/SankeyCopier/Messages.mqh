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
                                      FormatTimestampISO8601(TimeGMT()), ea_type);

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
                                          FormatTimestampISO8601(TimeGMT()));

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
//| Send heartbeat message                                           |
//+------------------------------------------------------------------+
bool SendHeartbeatMessage(HANDLE_TYPE context, string address, string account_id, string ea_type, string platform,
                         string symbol_prefix="", string symbol_suffix="", string symbol_map="")
{
   HANDLE_TYPE socket = CreateAndConnectZmqSocket(context, ZMQ_PUSH, address, "Heartbeat PUSH");
   if(socket < 0) return false;
   
   double balance = AccountInfoDouble(ACCOUNT_BALANCE);
   double equity = AccountInfoDouble(ACCOUNT_EQUITY);
   int open_positions = 0;
   
   #ifdef IS_MT5
      open_positions = PositionsTotal();
   #else
      for(int i=0; i<OrdersTotal(); i++) {
         if(OrderSelect(i, SELECT_BY_POS, MODE_TRADES)) open_positions++;
      }
   #endif
   
   string timestamp = GetTimestampISO();
   long account_number = AccountInfoInteger(ACCOUNT_LOGIN);
   string broker = AccountInfoString(ACCOUNT_COMPANY);
   string account_name = AccountInfoString(ACCOUNT_NAME);
   string server = AccountInfoString(ACCOUNT_SERVER);
   string currency = AccountInfoString(ACCOUNT_CURRENCY);
   long leverage = AccountInfoInteger(ACCOUNT_LEVERAGE);
   int is_trade_allowed = (int)TerminalInfoInteger(TERMINAL_TRADE_ALLOWED);
   
   int len = serialize_heartbeat("Heartbeat", account_id, balance, equity, open_positions, timestamp,
                                ea_type, platform, account_number, broker, account_name, server,
                                currency, leverage, is_trade_allowed,
                                symbol_prefix, symbol_suffix, symbol_map);

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
