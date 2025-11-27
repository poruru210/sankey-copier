//+------------------------------------------------------------------+
//|                                      SankeyCopierCleanupBase.mqh  |
//|                        Copyright 2025, SANKEY Copier Project        |
//|                                                                    |
//| Purpose: Common EA cleanup/deinitialization utilities              |
//| Why: Consolidates cleanup patterns shared across all EAs           |
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, SANKEY Copier Project"
#property strict

#ifndef SANKEY_COPIER_CLEANUP_BASE_MQH
#define SANKEY_COPIER_CLEANUP_BASE_MQH

#include "Common.mqh"
#include "Zmq.mqh"
#include "Messages.mqh"
#include "TimerHandling.mqh"

// =============================================================================
// EA Cleanup Functions
// =============================================================================

//+------------------------------------------------------------------+
//| Cleanup Master EA resources                                       |
//|                                                                   |
//| Parameters:                                                        |
//|   zmq_context       - ZMQ context to cleanup                      |
//|   zmq_socket        - PUSH socket to cleanup                      |
//|   zmq_config_socket - SUB socket to cleanup                       |
//|   relay_addr        - Server address for unregister message       |
//|   account_id        - Account ID for unregister message           |
//+------------------------------------------------------------------+
void CleanupMasterEA(HANDLE_TYPE &zmq_context, HANDLE_TYPE &zmq_socket,
                     HANDLE_TYPE &zmq_config_socket,
                     string relay_addr, string account_id)
{
   // Send unregistration message
   SendUnregistrationMessage(zmq_context, relay_addr, account_id);

   // Kill timer
   DeInitializeTimer();

   // Cleanup ZMQ resources
   CleanupZmqMultiSocket(zmq_socket, zmq_config_socket, zmq_context,
                         "Master PUSH", "Master CONFIG SUB");

   Print("[INFO] Master EA cleanup completed");
}

//+------------------------------------------------------------------+
//| Cleanup Slave EA resources                                        |
//|                                                                   |
//| Parameters:                                                        |
//|   zmq_context       - ZMQ context to cleanup                      |
//|   zmq_socket        - PUSH socket to cleanup                      |
//|   zmq_trade_socket  - Trade SUB socket to cleanup                 |
//|   zmq_config_socket - Config SUB socket to cleanup                |
//|   relay_addr        - Server address for unregister message       |
//|   account_id        - Account ID for unregister message           |
//+------------------------------------------------------------------+
void CleanupSlaveEA(HANDLE_TYPE &zmq_context, HANDLE_TYPE &zmq_socket,
                    HANDLE_TYPE &zmq_trade_socket, HANDLE_TYPE &zmq_config_socket,
                    string relay_addr, string account_id)
{
   // Send unregistration message
   SendUnregistrationMessage(zmq_context, relay_addr, account_id);

   // Kill timer
   DeInitializeTimer();

   // Cleanup ZMQ sockets
   CleanupZmqSocket(zmq_socket, "Slave PUSH");
   CleanupZmqSocket(zmq_trade_socket, "Slave TRADE SUB");
   CleanupZmqSocket(zmq_config_socket, "Slave CONFIG SUB");

   // Cleanup context
   CleanupZmqContext(zmq_context);

   Print("[INFO] Slave EA cleanup completed");
}

//+------------------------------------------------------------------+
//| Cleanup panel if shown                                            |
//|                                                                   |
//| Parameters:                                                        |
//|   show_panel - Whether panel is shown                             |
//|   panel      - Panel object reference                             |
//|                                                                   |
//| Note: This function should be called before ZMQ cleanup           |
//+------------------------------------------------------------------+
template<typename T>
void CleanupPanel(bool show_panel, T &panel)
{
   if(show_panel)
   {
      panel.Delete();
   }
}

#endif // SANKEY_COPIER_CLEANUP_BASE_MQH
