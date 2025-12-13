//+------------------------------------------------------------------+
//|                                           SlaveContext.mqh       |
//|                        Copyright 2025, SANKEY Copier Project     |
//|                     Slave EA specific EaContext extensions       |
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, SANKEY Copier Project"
#property strict

#ifndef SLAVE_CONTEXT_MQH
#define SLAVE_CONTEXT_MQH

#include "Common.mqh"

//+------------------------------------------------------------------+
//| SlaveContextWrapper: Slave EA specific extensions                |
//+------------------------------------------------------------------+
class SlaveContextWrapper : public EaContextWrapper
{
public:
   SlaveContextWrapper() : EaContextWrapper() {}

   //--- Slave Config Accessor ---
   bool GetSlaveConfig(SSlaveConfig &config)
   {
      if(!IsInitialized()) return false;
      return ea_context_get_slave_config(GetHandle(), config) == 1;
   }

   //--- Position Snapshot (received from Master) ---
   int GetPositionSnapshotCount()
   {
      if(!IsInitialized()) return 0;
      return ea_context_get_position_snapshot_count(GetHandle());
   }

   bool GetPositionSnapshot(SPositionInfo &positions[])
   {
      if(!IsInitialized()) return false;
      int count = ea_context_get_position_snapshot_count(GetHandle());
      if (count <= 0) return false;

      ArrayResize(positions, count);
      return ea_context_get_position_snapshot(GetHandle(), positions, count) > 0;
   }

   string GetPositionSnapshotSourceAccount()
   {
       if(!IsInitialized()) return "";
       uchar buffer[64];
       if (ea_context_get_position_snapshot_source_account(GetHandle(), buffer, 64) == 1) {
           return CharArrayToString(buffer);
       }
       return "";
   }

   //--- Symbol Mappings ---
   int GetSymbolMappingsCount()
   {
      if(!IsInitialized()) return 0;
      return ea_context_get_symbol_mappings_count(GetHandle());
   }

   bool GetSymbolMappings(SSymbolMapping &mappings[])
   {
      if(!IsInitialized()) return false;
      int count = ea_context_get_symbol_mappings_count(GetHandle());
      if (count <= 0) return false;

      ArrayResize(mappings, count);
      return ea_context_get_symbol_mappings(GetHandle(), mappings, count) > 0;
   }

   //--- Sync Request (send to Master) ---
   bool SendSyncRequest(string master_account)
   {
      if(!IsInitialized()) return false;
      uchar buffer[1024];
      int len = ea_send_sync_request(GetHandle(), master_account, buffer, 1024);
      if(len > 0) return ea_send_push(GetHandle(), buffer, len) == 1;
      return false;
   }

   //--- Request Config ---
   bool SendRequestConfig()
   {
      if(!IsInitialized()) return false;
      uchar buffer[1024];
      int len = ea_send_request_config(GetHandle(), buffer, 1024);
      if(len > 0) return ea_send_push(GetHandle(), buffer, len) == 1;
      return false;
   }
};

#endif // SLAVE_CONTEXT_MQH
