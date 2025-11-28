//+------------------------------------------------------------------+
//|                                    SankeyCopierMessageParsing.mqh |
//|                        Copyright 2025, SANKEY Copier Project        |
//|                                                                    |
//| Purpose: Unified ZMQ message parsing utilities for all EAs         |
//| Why: Eliminates ~114 LOC duplication across 6 message parsing      |
//|      locations (OnTimer/OnTick in all 4 EAs)                       |
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, SANKEY Copier Project"
#property strict

#ifndef SANKEY_COPIER_MESSAGE_PARSING_MQH
#define SANKEY_COPIER_MESSAGE_PARSING_MQH

#include "Common.mqh"

// =============================================================================
// ZMQ PUB/SUB Message Parsing
// =============================================================================
// ZMQ PUB/SUB message format: "TOPIC PAYLOAD"
//   - TOPIC: subscription topic string (account ID or topic ID)
//   - SPACE: single space separator (ASCII 32)
//   - PAYLOAD: MessagePack binary data
//
// Example: "IC_Markets_12345 <binary msgpack data>"

//+------------------------------------------------------------------+
//| Extract topic and MessagePack payload from ZMQ PUB/SUB message   |
//|                                                                   |
//| Parameters:                                                        |
//|   buffer       - Input buffer containing raw ZMQ message          |
//|   buffer_size  - Actual bytes received from socket                |
//|   topic        - Output: extracted topic string                   |
//|   payload      - Output: extracted payload bytes                  |
//|                                                                   |
//| Returns:                                                           |
//|   true  - Successfully extracted topic and payload                |
//|   false - Invalid format (no space separator found)               |
//|                                                                   |
//| Side Effects:                                                      |
//|   - Resizes payload[] array to actual payload size                |
//|   - Sets topic and payload output parameters                      |
//+------------------------------------------------------------------+
bool ExtractZmqTopicAndPayload(uchar &buffer[], int buffer_size,
                                string &topic, uchar &payload[])
{
   // Find the space separator between topic and MessagePack payload
   int space_pos = -1;
   for(int i = 0; i < buffer_size; i++)
   {
      if(buffer[i] == SPACE_CHAR)
      {
         space_pos = i;
         break;
      }
   }

   // Check if space separator was found
   if(space_pos <= 0)
   {
      // No valid separator found - invalid message format
      return false;
   }

   // Extract topic (bytes before space)
   topic = CharArrayToString(buffer, 0, space_pos);

   // Extract MessagePack payload (bytes after space)
   int payload_start = space_pos + 1;
   int payload_len = buffer_size - payload_start;

   // Validate payload length
   if(payload_len <= 0)
   {
      return false;
   }

   // Resize and copy payload
   ArrayResize(payload, payload_len);
   ArrayCopy(payload, buffer, 0, payload_start, payload_len);

   return true;
}

//+------------------------------------------------------------------+
//| Receive and parse ZMQ message in one operation                    |
//|                                                                   |
//| Parameters:                                                        |
//|   socket       - ZMQ socket handle                                |
//|   topic        - Output: extracted topic string                   |
//|   payload      - Output: extracted payload bytes                  |
//|                                                                   |
//| Returns:                                                           |
//|   > 0  - Payload size (message received and parsed successfully)  |
//|   0    - No message available                                     |
//|   -1   - Parse error (invalid message format)                     |
//+------------------------------------------------------------------+
int ReceiveAndParseZmqMessage(HANDLE_TYPE socket, string &topic, uchar &payload[])
{
   // Allocate receive buffer
   uchar buffer[];
   ArrayResize(buffer, MESSAGE_BUFFER_SIZE);

   // Receive message from socket
   int bytes = zmq_socket_receive(socket, buffer, MESSAGE_BUFFER_SIZE);

   // No message available
   if(bytes <= 0)
   {
      return 0;
   }

   // Parse message
   if(!ExtractZmqTopicAndPayload(buffer, bytes, topic, payload))
   {
      return -1;
   }

   return ArraySize(payload);
}

//+------------------------------------------------------------------+
//| Check if topic matches account ID or is a broadcast topic         |
//|                                                                   |
//| Parameters:                                                        |
//|   topic         - Received topic string                           |
//|   account_id    - This EA's account ID                           |
//|                                                                   |
//| Returns:                                                           |
//|   true  - Topic matches this account or is broadcast              |
//|   false - Topic is for different account                          |
//+------------------------------------------------------------------+
bool IsTopicForAccount(string topic, string account_id)
{
   // Check if topic matches this account
   if(topic == account_id)
      return true;

   // Check for broadcast topics (if any)
   if(topic == "broadcast" || topic == "*")
      return true;

   return false;
}

#endif // SANKEY_COPIER_MESSAGE_PARSING_MQH
