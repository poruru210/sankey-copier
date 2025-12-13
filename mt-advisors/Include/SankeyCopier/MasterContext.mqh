//+------------------------------------------------------------------+
//|                                          MasterContext.mqh       |
//|                        Copyright 2025, SANKEY Copier Project     |
//|                     Master EA specific EaContext extensions      |
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, SANKEY Copier Project"
#property strict

#ifndef MASTER_CONTEXT_MQH
#define MASTER_CONTEXT_MQH

#include "Common.mqh"

//+------------------------------------------------------------------+
//| MasterContextWrapper: Master EA specific extensions              |
//+------------------------------------------------------------------+
class MasterContextWrapper : public EaContextWrapper
{
public:
   MasterContextWrapper() : EaContextWrapper() {}

   //--- Master Config Accessor ---
   bool GetMasterConfig(SMasterConfig &config)
   {
      if(!IsInitialized()) return false;
      return ea_context_get_master_config(GetHandle(), config) == 1;
   }

   //--- Sync Request (received from Slave) ---
   bool GetSyncRequest(SSyncRequest &request)
   {
      if(!IsInitialized()) return false;
      return ea_context_get_sync_request(GetHandle(), request) == 1;
   }

   //--- Trade Signal Sending ---
   bool SendOpenSignal(long ticket, string symbol, string order_type, 
                       double lots, double price, double sl, double tp, 
                       long magic, string comment)
   {
      if(!IsInitialized()) return false;
      uchar buffer[1024];
      int len = ea_send_open_signal(GetHandle(), ticket, symbol, order_type, 
                                    lots, price, sl, tp, magic, comment, buffer, 1024);
      if(len > 0) return ea_send_push(GetHandle(), buffer, len) == 1;
      return false;
   }

   bool SendCloseSignal(long ticket, double close_ratio)
   {
      if(!IsInitialized()) return false;
      uchar buffer[1024];
      int len = ea_send_close_signal(GetHandle(), ticket, close_ratio, buffer, 1024);
      if(len > 0) return ea_send_push(GetHandle(), buffer, len) == 1;
      return false;
   }
   
   bool SendModifySignal(long ticket, double sl, double tp)
   {
      if(!IsInitialized()) return false;
      uchar buffer[1024];
      int len = ea_send_modify_signal(GetHandle(), ticket, sl, tp, buffer, 1024);
      if(len > 0) return ea_send_push(GetHandle(), buffer, len) == 1;
      return false;
   }

   //--- Position Snapshot (send to Slave) ---
   bool SendPositionSnapshot(SPositionInfo &positions[])
   {
      if(!IsInitialized()) return false;
      int count = ArraySize(positions);
      return ea_send_position_snapshot(GetHandle(), positions, count) == 1;
   }
};

#endif // MASTER_CONTEXT_MQH
