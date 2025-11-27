//+------------------------------------------------------------------+
//|                                        SankeyCopierInitBase.mqh   |
//|                        Copyright 2025, SANKEY Copier Project        |
//|                                                                    |
//| Purpose: Common EA initialization utilities                        |
//| Why: Consolidates initialization patterns shared across all EAs    |
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, SANKEY Copier Project"
#property strict

#ifndef SANKEY_COPIER_INIT_BASE_MQH
#define SANKEY_COPIER_INIT_BASE_MQH

#include "Common.mqh"
#include "Zmq.mqh"

// =============================================================================
// Account ID Initialization
// =============================================================================

//+------------------------------------------------------------------+
//| Initialize account ID if not provided                             |
//|                                                                   |
//| Parameters:                                                        |
//|   account_id - Account ID string (ref, may be modified)           |
//|                                                                   |
//| Returns:                                                           |
//|   Initialized account ID (generated if empty)                     |
//+------------------------------------------------------------------+
string InitializeAccountID(string account_id)
{
   if(account_id == "")
   {
      return GenerateAccountID();
   }
   return account_id;
}

// =============================================================================
// ZMQ Socket Initialization Helpers
// =============================================================================

//+------------------------------------------------------------------+
//| Initialize Master EA ZMQ sockets                                   |
//|                                                                   |
//| Parameters:                                                        |
//|   zmq_context       - Output: Created ZMQ context                 |
//|   zmq_socket        - Output: PUSH socket for heartbeat/signals   |
//|   zmq_config_socket - Output: SUB socket for config messages      |
//|   relay_addr        - PUSH socket address                         |
//|   config_addr       - SUB socket address for config               |
//|   account_id        - Account ID for subscription topic           |
//|                                                                   |
//| Returns:                                                           |
//|   true  - All sockets initialized successfully                    |
//|   false - Initialization failed                                   |
//+------------------------------------------------------------------+
bool InitializeMasterZmqSockets(HANDLE_TYPE &zmq_context, HANDLE_TYPE &zmq_socket,
                                 HANDLE_TYPE &zmq_config_socket,
                                 string relay_addr, string config_addr, string account_id)
{
   // Initialize context
   if(!InitializeZmqContext(zmq_context))
   {
      Print("[ERROR] Failed to initialize ZMQ context");
      return false;
   }

   // Create and connect PUSH socket for heartbeat/signals
   if(!CreateAndConnectZmqSocket(zmq_context, zmq_socket, ZMQ_PUSH, relay_addr, "Master PUSH"))
   {
      Print("[ERROR] Failed to create PUSH socket");
      CleanupZmqContext(zmq_context);
      return false;
   }

   // Create and connect SUB socket for config
   if(!CreateAndConnectZmqSocket(zmq_context, zmq_config_socket, ZMQ_SUB, config_addr, "Master CONFIG SUB"))
   {
      Print("[ERROR] Failed to create CONFIG SUB socket");
      CleanupZmqSocket(zmq_socket, "Master PUSH");
      CleanupZmqContext(zmq_context);
      return false;
   }

   // Subscribe to account-specific topic
   if(!SubscribeToTopic(zmq_config_socket, account_id, "Master"))
   {
      Print("[ERROR] Failed to subscribe to config topic");
      CleanupZmqMultiSocket(zmq_socket, zmq_config_socket, zmq_context, "Master PUSH", "Master CONFIG SUB");
      return false;
   }

   Print("[INFO] Master ZMQ sockets initialized successfully");
   return true;
}

//+------------------------------------------------------------------+
//| Initialize Slave EA ZMQ sockets                                    |
//|                                                                   |
//| Parameters:                                                        |
//|   zmq_context       - Output: Created ZMQ context                 |
//|   zmq_socket        - Output: PUSH socket for heartbeat           |
//|   zmq_trade_socket  - Output: SUB socket for trade signals        |
//|   zmq_config_socket - Output: SUB socket for config messages      |
//|   relay_addr        - PUSH socket address                         |
//|   trade_addr        - SUB socket address for trades               |
//|   config_addr       - SUB socket address for config               |
//|   account_id        - Account ID for subscription topic           |
//|                                                                   |
//| Returns:                                                           |
//|   true  - All sockets initialized successfully                    |
//|   false - Initialization failed                                   |
//+------------------------------------------------------------------+
bool InitializeSlaveZmqSockets(HANDLE_TYPE &zmq_context, HANDLE_TYPE &zmq_socket,
                                HANDLE_TYPE &zmq_trade_socket, HANDLE_TYPE &zmq_config_socket,
                                string relay_addr, string trade_addr, string config_addr,
                                string account_id)
{
   // Initialize context
   if(!InitializeZmqContext(zmq_context))
   {
      Print("[ERROR] Failed to initialize ZMQ context");
      return false;
   }

   // Create and connect PUSH socket for heartbeat
   if(!CreateAndConnectZmqSocket(zmq_context, zmq_socket, ZMQ_PUSH, relay_addr, "Slave PUSH"))
   {
      Print("[ERROR] Failed to create PUSH socket");
      CleanupZmqContext(zmq_context);
      return false;
   }

   // Create and connect SUB socket for trades
   if(!CreateAndConnectZmqSocket(zmq_context, zmq_trade_socket, ZMQ_SUB, trade_addr, "Slave TRADE SUB"))
   {
      Print("[ERROR] Failed to create TRADE SUB socket");
      CleanupZmqSocket(zmq_socket, "Slave PUSH");
      CleanupZmqContext(zmq_context);
      return false;
   }

   // Create and connect SUB socket for config
   if(!CreateAndConnectZmqSocket(zmq_context, zmq_config_socket, ZMQ_SUB, config_addr, "Slave CONFIG SUB"))
   {
      Print("[ERROR] Failed to create CONFIG SUB socket");
      CleanupZmqSocket(zmq_socket, "Slave PUSH");
      CleanupZmqSocket(zmq_trade_socket, "Slave TRADE SUB");
      CleanupZmqContext(zmq_context);
      return false;
   }

   // Subscribe to account-specific topic on config socket
   if(!SubscribeToTopic(zmq_config_socket, account_id, "Slave CONFIG"))
   {
      Print("[ERROR] Failed to subscribe to config topic");
      CleanupZmqSocket(zmq_socket, "Slave PUSH");
      CleanupZmqSocket(zmq_trade_socket, "Slave TRADE SUB");
      CleanupZmqSocket(zmq_config_socket, "Slave CONFIG SUB");
      CleanupZmqContext(zmq_context);
      return false;
   }

   // Note: Trade socket subscription is done dynamically when config is received
   // (subscribes to trade_group_id from each master config)

   Print("[INFO] Slave ZMQ sockets initialized successfully");
   return true;
}

// =============================================================================
// Common State Initialization
// =============================================================================

//+------------------------------------------------------------------+
//| Initialize common EA state variables                              |
//|                                                                   |
//| Parameters:                                                        |
//|   last_heartbeat      - Output: Set to 0                          |
//|   last_trade_allowed  - Output: Set to current state              |
//|   config_requested    - Output: Set to false                      |
//|   initialized         - Output: Set to false                      |
//+------------------------------------------------------------------+
void InitializeEAState(datetime &last_heartbeat, bool &last_trade_allowed,
                       bool &config_requested, bool &initialized)
{
   last_heartbeat = 0;
   last_trade_allowed = false;  // Will be set on first timer tick
   config_requested = false;
   initialized = false;
}

#endif // SANKEY_COPIER_INIT_BASE_MQH
