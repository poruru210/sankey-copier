//+------------------------------------------------------------------+
//|                                          MasterContext.mqh       |
//|                        Copyright 2025, SANKEY Copier Project     |
//|                     Master EA specific EaContext extensions      |
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, SANKEY Copier Project"
#property strict

#ifndef MASTER_CONTEXT_MQH
#define MASTER_CONTEXT_MQH

#include "EaContext.mqh"

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
   //--- Trade Signals (Master) ---
   bool SendOpenSignal(long ticket, string symbol, string order_type, 
                       double lots, double price, double sl, double tp, 
                       long magic, string comment)
   {
      if(!IsInitialized()) return false;
      return ea_send_open_signal(GetHandle(), ticket, symbol, order_type, 
                                 lots, price, sl, tp, magic, comment) == 1;
   }

   bool SendCloseSignal(long ticket, double close_ratio)
   {
      if(!IsInitialized()) return false;
      return ea_send_close_signal(GetHandle(), ticket, close_ratio) == 1;
   }
   
   bool SendModifySignal(long ticket, double sl, double tp)
   {
      if(!IsInitialized()) return false;
      return ea_send_modify_signal(GetHandle(), ticket, sl, tp) == 1;
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
