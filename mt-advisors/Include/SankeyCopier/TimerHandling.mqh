//+------------------------------------------------------------------+
//|                                     SankeyCopierTimerHandling.mqh |
//|                        Copyright 2025, SANKEY Copier Project        |
//|                                                                    |
//| Purpose: Unified timer handling utilities for all EAs              |
//| Why: Eliminates ~220 LOC duplication across 4 EAs for heartbeat,   |
//|      config request, and panel status update logic                 |
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, SANKEY Copier Project"
#property strict

#ifndef SANKEY_COPIER_TIMER_HANDLING_MQH
#define SANKEY_COPIER_TIMER_HANDLING_MQH

#include "Common.mqh"
#include "Messages.mqh"

// =============================================================================
// Trade State Detection
// =============================================================================

//+------------------------------------------------------------------+
//| Check if auto-trading is allowed                                  |
//| Returns: true if auto-trading is enabled, false otherwise        |
//+------------------------------------------------------------------+
bool IsAutoTradingAllowed()
{
   #ifdef IS_MT5
      return (bool)TerminalInfoInteger(TERMINAL_TRADE_ALLOWED);
   #else
      return IsTradeAllowed();
   #endif
}

// =============================================================================
// Heartbeat Timer Logic
// =============================================================================

//+------------------------------------------------------------------+
//| Check if heartbeat should be sent                                 |
//|                                                                   |
//| Parameters:                                                        |
//|   last_heartbeat       - Last heartbeat timestamp                 |
//|   last_trade_allowed   - Previous auto-trading state              |
//|   trade_state_changed  - Output: true if state changed            |
//|                                                                   |
//| Returns:                                                           |
//|   true  - Heartbeat should be sent                                |
//|   false - Not time for heartbeat yet                              |
//+------------------------------------------------------------------+
bool ShouldSendHeartbeat(datetime last_heartbeat, bool last_trade_allowed, bool &trade_state_changed)
{
   bool current_trade_allowed = IsAutoTradingAllowed();
   trade_state_changed = (current_trade_allowed != last_trade_allowed);

   datetime now = TimeLocal();
   bool interval_elapsed = (now - last_heartbeat >= HEARTBEAT_INTERVAL_SECONDS);

   return interval_elapsed || trade_state_changed;
}

//+------------------------------------------------------------------+
//| Handle heartbeat timer event                                       |
//|                                                                   |
//| Purpose: Consolidates heartbeat sending logic from all EAs        |
//|          Handles trade state change detection and logging         |
//|                                                                   |
//| Parameters:                                                        |
//|   last_heartbeat      - Last heartbeat timestamp (ref, updated)   |
//|   last_trade_allowed  - Previous auto-trading state (ref, updated)|
//|   zmq_context         - ZMQ context handle                        |
//|   server_addr         - Server address for heartbeat PUSH         |
//|   account_id          - Account ID for message                    |
//|   ea_type             - "Master" or "Slave"                       |
//|   platform            - "MT4" or "MT5"                            |
//|   symbol_prefix       - Symbol prefix for heartbeat               |
//|   symbol_suffix       - Symbol suffix for heartbeat               |
//|   symbol_map          - Symbol map for heartbeat (Slave only)     |
//|   trade_state_changed - Output: true if state changed             |
//|                                                                   |
//| Returns:                                                           |
//|   true  - Heartbeat was sent successfully                         |
//|   false - Heartbeat not sent (timing or error)                    |
//+------------------------------------------------------------------+
bool HandleHeartbeatTimer(datetime &last_heartbeat, bool &last_trade_allowed,
                          HANDLE_TYPE zmq_context, string server_addr,
                          string account_id, string ea_type, string platform,
                          string symbol_prefix, string symbol_suffix,
                          string symbol_map, bool &trade_state_changed)
{
   // Check if heartbeat should be sent
   if(!ShouldSendHeartbeat(last_heartbeat, last_trade_allowed, trade_state_changed))
   {
      return false;
   }

   // Send heartbeat
   bool heartbeat_sent = SendHeartbeatMessage(zmq_context, server_addr, account_id,
                                               ea_type, platform,
                                               symbol_prefix, symbol_suffix, symbol_map);

   if(heartbeat_sent)
   {
      last_heartbeat = TimeLocal();

      // Log trade state change if occurred
      if(trade_state_changed)
      {
         bool current_trade_allowed = IsAutoTradingAllowed();
         Print("[INFO] Auto-trading state changed: ", last_trade_allowed, " -> ", current_trade_allowed);
         last_trade_allowed = current_trade_allowed;
      }
   }

   return heartbeat_sent;
}

// =============================================================================
// Configuration Request Logic
// =============================================================================

//+------------------------------------------------------------------+
//| Handle configuration request on first heartbeat or state change   |
//|                                                                   |
//| Parameters:                                                        |
//|   config_requested - Config request flag (ref, updated)           |
//|   zmq_context      - ZMQ context handle                           |
//|   server_addr      - Server address for PUSH                      |
//|   account_id       - Account ID for request                       |
//|   ea_type          - "Master" or "Slave"                          |
//|                                                                   |
//| Returns:                                                           |
//|   true  - Config request sent successfully                        |
//|   false - Request failed or already sent                          |
//+------------------------------------------------------------------+
bool HandleConfigRequest(bool &config_requested, HANDLE_TYPE zmq_context,
                         string server_addr, string account_id, string ea_type)
{
   // Skip if already requested
   if(config_requested)
   {
      return false;
   }

   // Skip if auto-trading is disabled
   if(!IsAutoTradingAllowed())
   {
      return false;
   }

   if(SendRequestConfigMessage(zmq_context, server_addr, account_id, ea_type))
   {
      config_requested = true;
      return true;
   }
   else
   {
      Print("[ERROR] Failed to send configuration request");
      return false;
   }
}

// =============================================================================
// Panel Status Update (Slave-specific)
// =============================================================================

//+------------------------------------------------------------------+
//| Determine status to display based on trade state and configs      |
//|                                                                   |
//| Parameters:                                                        |
//|   trade_allowed   - Current auto-trading state                    |
//|   has_config      - Whether any config has been received          |
//|   config_count    - Number of configurations                      |
//|   any_connected   - Whether any master is connected               |
//|                                                                   |
//| Returns:                                                           |
//|   STATUS_* constant for panel display                             |
//+------------------------------------------------------------------+
int DetermineSlaveStatus(bool trade_allowed, bool has_config, int config_count, bool any_connected)
{
   // No config received yet
   if(!has_config)
   {
      return STATUS_NO_CONFIGURATION;
   }

   // Auto-trading disabled - show warning
   if(!trade_allowed)
   {
      return STATUS_ENABLED;  // Yellow warning state
   }

   // Auto-trading enabled - show actual status
   if(config_count == 0)
   {
      return STATUS_NO_CONFIGURATION;
   }
   else if(any_connected)
   {
      return STATUS_CONNECTED;
   }
   else
   {
      return STATUS_ENABLED;
   }
}

// =============================================================================
// Timer Initialization/Cleanup
// =============================================================================

//+------------------------------------------------------------------+
//| Initialize timer for EA                                           |
//| Sets up 1-second timer for heartbeat and message processing       |
//+------------------------------------------------------------------+
void InitializeTimer()
{
   EventSetTimer(1);
}

//+------------------------------------------------------------------+
//| Cleanup timer on EA deinitialization                              |
//+------------------------------------------------------------------+
void DeInitializeTimer()
{
   EventKillTimer();
}

#endif // SANKEY_COPIER_TIMER_HANDLING_MQH
