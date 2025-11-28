//+------------------------------------------------------------------+
//|                                     SankeyCopierMessages.mqh      |
//|                        Copyright 2025, SANKEY Copier Project      |
//|                     Message sending utilities                     |
//+------------------------------------------------------------------+
// Purpose: Common message functions shared between Master and Slave EAs
// Note: Master-specific signal functions moved to MasterSignals.mqh
#property copyright "Copyright 2025, SANKEY Copier Project"

#ifndef SANKEY_COPIER_MESSAGES_MQH
#define SANKEY_COPIER_MESSAGES_MQH

#include "Common.mqh"
#include "Zmq.mqh"
#include "Logging.mqh"

// Include Master signals for backward compatibility
// Master EA can include Messages.mqh and still use SendOpenSignal etc.
#include "MasterSignals.mqh"

//+------------------------------------------------------------------+
//| Send configuration request message to server (for Slave EAs)     |
//+------------------------------------------------------------------+
bool SendRequestConfigMessage(HANDLE_TYPE zmq_context, string server_address, string account_id, string ea_type)
{
   // Create temporary PUSH socket for config request
   HANDLE_TYPE push_socket = zmq_socket_create(zmq_context, ZMQ_PUSH);
   if(push_socket < 0)
   {
      LogError(CAT_SYSTEM, "Failed to create request config socket");
      return false;
   }

   if(zmq_socket_connect(push_socket, server_address) == 0)
   {
      LogError(CAT_SYSTEM, StringFormat("Failed to connect to request config server: %s", server_address));
      zmq_socket_destroy(push_socket);
      return false;
   }

   // Serialize request config message using MessagePack
   int len = serialize_request_config("RequestConfig", account_id,
                                      FormatTimestampISO8601(TimeGMT()), ea_type);

   if(len <= 0)
   {
      LogError(CAT_SYSTEM, "Failed to serialize request config message");
      zmq_socket_destroy(push_socket);
      return false;
   }

   // Copy serialized data to buffer
   uchar buffer[];
   ArrayResize(buffer, len);
   int copied = copy_serialized_buffer(buffer, len);

   if(copied != len)
   {
      LogError(CAT_SYSTEM, "Failed to copy request config message buffer");
      zmq_socket_destroy(push_socket);
      return false;
   }

   // Send binary MessagePack data
   bool success = (zmq_socket_send_binary(push_socket, buffer, len) == 1);

   if(!success)
      LogError(CAT_SYSTEM, "Failed to send request config message");

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
      LogError(CAT_SYSTEM, "Failed to create unregistration socket");
      return false;
   }

   if(zmq_socket_connect(push_socket, server_address) == 0)
   {
      LogError(CAT_SYSTEM, StringFormat("Failed to connect to unregistration server: %s", server_address));
      zmq_socket_destroy(push_socket);
      return false;
   }

   // Serialize unregistration message using MessagePack
   int len = serialize_unregister("Unregister", account_id,
                                          FormatTimestampISO8601(TimeGMT()));

   if(len <= 0)
   {
      LogError(CAT_SYSTEM, "Failed to serialize unregistration message");
      zmq_socket_destroy(push_socket);
      return false;
   }

   // Copy serialized data to buffer
   uchar buffer[];
   ArrayResize(buffer, len);
   int copied = copy_serialized_buffer(buffer, len);

   if(copied != len)
   {
      LogError(CAT_SYSTEM, "Failed to copy unregistration message buffer");
      zmq_socket_destroy(push_socket);
      return false;
   }

   // Send binary MessagePack data
   bool success = (zmq_socket_send_binary(push_socket, buffer, len) == 1);

   if(!success)
      LogError(CAT_SYSTEM, "Failed to send unregistration message");

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
   
   string timestamp = TimeToString(TimeGMT(), TIME_DATE|TIME_MINUTES|TIME_SECONDS);
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
      LogError(CAT_SYSTEM, "Failed to serialize heartbeat message");
      zmq_socket_destroy(socket);
      return false;
   }

   // Copy serialized data to buffer
   uchar buffer[];
   ArrayResize(buffer, len);
   int copied = copy_serialized_buffer(buffer, len);

   if(copied != len)
   {
      LogError(CAT_SYSTEM, "Failed to copy heartbeat message buffer");
      zmq_socket_destroy(socket);
      return false;
   }

   // Send binary MessagePack data
   bool success = (zmq_socket_send_binary(socket, buffer, len) == 1);

   zmq_socket_destroy(socket);
   return success;
}

//+------------------------------------------------------------------+
//| Send sync request message to server (Slave EA only)              |
//| Used to request position snapshot from Master when Slave starts   |
//+------------------------------------------------------------------+
bool SendSyncRequestMessage(HANDLE_TYPE zmq_context, string server_address,
                            string slave_account, string master_account)
{
   // Create temporary PUSH socket for sync request
   HANDLE_TYPE push_socket = zmq_socket_create(zmq_context, ZMQ_PUSH);
   if(push_socket < 0)
   {
      LogError(CAT_SYNC, "Failed to create sync request socket");
      return false;
   }

   if(zmq_socket_connect(push_socket, server_address) == 0)
   {
      LogError(CAT_SYNC, StringFormat("Failed to connect for sync request: %s", server_address));
      zmq_socket_destroy(push_socket);
      return false;
   }

   // Create and serialize SyncRequest message
   uchar buffer[];
   ArrayResize(buffer, MESSAGE_BUFFER_SIZE);
   int len = create_sync_request(slave_account, master_account, buffer, MESSAGE_BUFFER_SIZE);

   if(len <= 0)
   {
      LogError(CAT_SYNC, "Failed to create sync request message");
      zmq_socket_destroy(push_socket);
      return false;
   }

   // Resize buffer to actual size
   ArrayResize(buffer, len);

   // Send binary MessagePack data
   bool success = (zmq_socket_send_binary(push_socket, buffer, len) == 1);

   if(success)
      LogInfo(CAT_SYNC, StringFormat("Request sent to master: %s", master_account));
   else
      LogError(CAT_SYNC, "Failed to send sync request message");

   zmq_socket_destroy(push_socket);
   return success;
}

// Note: SendOpenSignal, SendCloseSignal, SendModifySignal moved to MasterSignals.mqh
// They are still available here through the #include for backward compatibility

#endif // SANKEY_COPIER_MESSAGES_MQH
