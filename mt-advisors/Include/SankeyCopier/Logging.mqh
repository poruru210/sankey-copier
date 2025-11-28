//+------------------------------------------------------------------+
//|                                                     Logging.mqh  |
//|                        Copyright 2025, SANKEY Copier Project     |
//|                 VictoriaLogs integration helpers for EA logging  |
//+------------------------------------------------------------------+
// This file provides wrapper functions for VictoriaLogs integration.
// Logs are buffered locally and sent via HTTP to VictoriaLogs server.
// Call VLogsInit() in OnInit() and VLogsFlush() in OnDeinit().
// Call VLogsFlushIfNeeded() periodically from OnTimer().
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, SANKEY Copier Project"

#ifndef SANKEY_COPIER_LOGGING_MQH
#define SANKEY_COPIER_LOGGING_MQH

#include "Common.mqh"

//--- Log levels (matches VictoriaLogs convention)
#define LOG_DEBUG "DEBUG"
#define LOG_INFO  "INFO"
#define LOG_WARN  "WARN"
#define LOG_ERROR "ERROR"

//--- Log categories (used for filtering in VictoriaLogs)
#define CAT_TRADE  "Trade"
#define CAT_CONFIG "Config"
#define CAT_SYNC   "Sync"
#define CAT_SYSTEM "System"
#define CAT_ORDER  "Order"

//--- Global state for VictoriaLogs
bool g_vlogs_enabled = false;
datetime g_last_flush = 0;
int g_flush_interval_sec = 5;

//+------------------------------------------------------------------+
//| Initialize VictoriaLogs logging                                   |
//| Call this from OnInit() to enable logging                         |
//| Parameters:                                                       |
//|   endpoint - VictoriaLogs URL (empty to disable)                  |
//|              e.g., "http://localhost:9428/insert/jsonline"        |
//|   source   - Source identifier for logs                           |
//|              e.g., "ea:master:IC_Markets_12345"                   |
//|   flush_interval_sec - How often to flush logs (default 5 sec)    |
//+------------------------------------------------------------------+
void VLogsInit(string endpoint, string source, int flush_interval_sec = 5)
{
   // Disable if endpoint is empty
   if(endpoint == "")
   {
      g_vlogs_enabled = false;
      vlogs_disable();
      return;
   }

   // Configure via FFI
   if(vlogs_configure(endpoint, source) == 1)
   {
      g_vlogs_enabled = true;
      g_flush_interval_sec = flush_interval_sec;
      g_last_flush = TimeLocal();
      Print("[VLOGS] Configured: endpoint=", endpoint, ", source=", source);
   }
   else
   {
      g_vlogs_enabled = false;
      Print("[VLOGS] Failed to configure VictoriaLogs");
   }
}

//+------------------------------------------------------------------+
//| Add log entry (also prints locally)                               |
//| Parameters:                                                       |
//|   level    - LOG_DEBUG, LOG_INFO, LOG_WARN, LOG_ERROR            |
//|   category - CAT_TRADE, CAT_CONFIG, CAT_SYNC, CAT_SYSTEM, etc.   |
//|   message  - Log message                                          |
//|   context  - Optional JSON string with additional context         |
//+------------------------------------------------------------------+
void VLog(string level, string category, string message, string context = "")
{
   // Always print locally with prefix (consistent with existing log format)
   Print("[", level, "] [", category, "] ", message);

   // Send to VictoriaLogs if enabled
   if(!g_vlogs_enabled) return;
   vlogs_add_entry(level, category, message, context);
}

//+------------------------------------------------------------------+
//| Log debug message                                                 |
//+------------------------------------------------------------------+
void LogDebug(string category, string message, string context = "")
{
   VLog(LOG_DEBUG, category, message, context);
}

//+------------------------------------------------------------------+
//| Log info message                                                  |
//+------------------------------------------------------------------+
void LogInfo(string category, string message, string context = "")
{
   VLog(LOG_INFO, category, message, context);
}

//+------------------------------------------------------------------+
//| Log warning message                                               |
//+------------------------------------------------------------------+
void LogWarn(string category, string message, string context = "")
{
   VLog(LOG_WARN, category, message, context);
}

//+------------------------------------------------------------------+
//| Log error message                                                 |
//+------------------------------------------------------------------+
void LogError(string category, string message, string context = "")
{
   VLog(LOG_ERROR, category, message, context);
}

//+------------------------------------------------------------------+
//| Flush logs if interval has elapsed                               |
//| Call this from OnTimer()                                          |
//+------------------------------------------------------------------+
void VLogsFlushIfNeeded()
{
   if(!g_vlogs_enabled) return;

   datetime now = TimeLocal();
   if((int)(now - g_last_flush) >= g_flush_interval_sec)
   {
      vlogs_flush();
      g_last_flush = now;
   }
}

//+------------------------------------------------------------------+
//| Force flush all buffered logs                                    |
//| Call this from OnDeinit() to ensure all logs are sent            |
//+------------------------------------------------------------------+
void VLogsFlush()
{
   if(g_vlogs_enabled)
   {
      vlogs_flush();
      g_last_flush = TimeLocal();
   }
}

//+------------------------------------------------------------------+
//| Disable VictoriaLogs logging                                      |
//+------------------------------------------------------------------+
void VLogsDisable()
{
   g_vlogs_enabled = false;
   vlogs_disable();
}

//+------------------------------------------------------------------+
//| Get current buffer size                                          |
//| Returns: Number of entries waiting to be flushed                  |
//+------------------------------------------------------------------+
int VLogsBufferSize()
{
   if(!g_vlogs_enabled) return 0;
   return vlogs_buffer_size();
}

//+------------------------------------------------------------------+
//| Build JSON context string for trade-related logs                  |
//| Parameters:                                                       |
//|   ticket   - Order/Position ticket                                |
//|   symbol   - Trading symbol                                       |
//|   lots     - Lot size (optional, 0 to omit)                       |
//|   magic    - Magic number (optional, 0 to omit)                   |
//| Returns: JSON string for use as context parameter                 |
//+------------------------------------------------------------------+
string BuildTradeContext(long ticket, string symbol, double lots = 0, long magic = 0)
{
   string json = "{";
   json += "\"ticket\":" + IntegerToString(ticket);
   json += ",\"symbol\":\"" + symbol + "\"";
   if(lots > 0)
      json += ",\"lots\":" + DoubleToString(lots, 2);
   if(magic > 0)
      json += ",\"magic\":" + IntegerToString(magic);
   json += "}";
   return json;
}

//+------------------------------------------------------------------+
//| Build JSON context string for sync-related logs                   |
//| Parameters:                                                       |
//|   master_account - Master account ID                              |
//|   slave_account  - Slave account ID (optional)                    |
//|   positions      - Number of positions (optional, -1 to omit)     |
//| Returns: JSON string for use as context parameter                 |
//+------------------------------------------------------------------+
string BuildSyncContext(string master_account, string slave_account = "", int positions = -1)
{
   string json = "{";
   json += "\"master\":\"" + master_account + "\"";
   if(slave_account != "")
      json += ",\"slave\":\"" + slave_account + "\"";
   if(positions >= 0)
      json += ",\"positions\":" + IntegerToString(positions);
   json += "}";
   return json;
}

//+------------------------------------------------------------------+
//| Apply VictoriaLogs configuration from server message              |
//| Called when "vlogs_config" message is received from relay-server  |
//| Parameters:                                                       |
//|   config_handle - Handle from parse_vlogs_config()                |
//|   ea_type       - "master" or "slave" for source identification   |
//|   account_id    - Account identifier for source identification    |
//| Returns: true if config was applied successfully                  |
//+------------------------------------------------------------------+
bool VLogsApplyConfig(HANDLE_TYPE config_handle, string ea_type, string account_id)
{
   if(config_handle == 0)
   {
      Print("[VLOGS] Invalid config handle");
      return false;
   }

   // Get enabled flag
   int enabled = vlogs_config_get_bool(config_handle, "enabled");

   if(enabled == 1)
   {
      // Get configuration values
      string endpoint = vlogs_config_get_string(config_handle, "endpoint");
      int flush_interval = vlogs_config_get_int(config_handle, "flush_interval_secs");

      // Build source identifier: "ea:master:IC_Markets_12345" or "ea:slave:..."
      string source = "ea:" + ea_type + ":" + account_id;

      // Configure VictoriaLogs
      if(vlogs_configure(endpoint, source) == 1)
      {
         g_vlogs_enabled = true;
         g_flush_interval_sec = flush_interval > 0 ? flush_interval : 5;
         g_last_flush = TimeLocal();
         Print("[VLOGS] Enabled via server config: endpoint=", endpoint,
               ", source=", source, ", flush_interval=", g_flush_interval_sec, "s");
         return true;
      }
      else
      {
         Print("[VLOGS] Failed to configure VictoriaLogs from server settings");
         g_vlogs_enabled = false;
         return false;
      }
   }
   else
   {
      // Disable VictoriaLogs
      if(g_vlogs_enabled)
      {
         Print("[VLOGS] Disabled by server config");
      }
      g_vlogs_enabled = false;
      vlogs_disable();
      return true;
   }
}

#endif // SANKEY_COPIER_LOGGING_MQH
