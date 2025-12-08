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
