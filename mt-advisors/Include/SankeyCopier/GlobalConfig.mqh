//+------------------------------------------------------------------+
//|                                                 GlobalConfig.mqh |
//|                        Copyright 2025, SANKEY Copier Project     |
//|                     Global Configuration Logic                   |
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, SANKEY Copier Project"
#property strict

#include "EaContext.mqh"
#include "Logging.mqh"

//+------------------------------------------------------------------+
//| GlobalConfigManager: Handles Global Config Updates               |
//+------------------------------------------------------------------+
class GlobalConfigManager
{
private:
   EaContextWrapper *m_ctx;
   bool m_enabled_prev;
   string m_log_level_prev;
   
public:
   GlobalConfigManager(EaContextWrapper *ctx) : m_ctx(ctx), m_enabled_prev(false), m_log_level_prev("") {}
   
   void CheckForUpdate()
   {
      if(CheckPointer(m_ctx) == POINTER_INVALID) return;
      if(!m_ctx.IsInitialized()) return;
      
      SGlobalConfig config;
      if(m_ctx.GetGlobalConfig(config))
      {
         string current_log_level = CharArrayToString(config.log_level);
         bool current_enabled = (config.enabled != 0);
         
         // Fix log level string (remove nulls if any)
         StringReplace(current_log_level, "\0", "");
         
         // Apply updates only if changed
         if(current_enabled != m_enabled_prev || current_log_level != m_log_level_prev)
         {
            ApplyConfig(config);
            m_enabled_prev = current_enabled;
            m_log_level_prev = current_log_level;
         }
      }
   }
   
   void ApplyConfig(SGlobalConfig &config)
   {
      string level_str = CharArrayToString(config.log_level);
      // Ensure null termination doesn't result in garbage
      int null_pos = StringFind(level_str, "\0");
      if(null_pos >= 0) level_str = StringSubstr(level_str, 0, null_pos);
      
      if(StringLen(level_str) > 0)
      {
         VLogsSetLevel(level_str);
         LogInfo(CAT_CONFIG, StringFormat("Global Config Updated: LogLevel=%s, Enabled=%d", level_str, config.enabled));
      }
      
      // Other global settings (batch_size, etc.) can be applied here if exposed in Logging.mqh
      // For now, VLogs configuration is mostly handled internally by the library,
      // but dynamic log level is the main requirement.
   }
};
