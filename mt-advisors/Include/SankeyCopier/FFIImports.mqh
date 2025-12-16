//+------------------------------------------------------------------+
//|                                                 FFIImports.mqh   |
//|                        Copyright 2025, SANKEY Copier Project     |
//|                     FFI Function Imports (sankey_copier_zmq.dll) |
//+------------------------------------------------------------------+
// This file contains all #import declarations for the Rust DLL.
// Separated from Common.mqh for clarity and easier FFI maintenance.
#property copyright "Copyright 2025, SANKEY Copier Project"
#property strict

#ifndef FFI_IMPORTS_MQH
#define FFI_IMPORTS_MQH

#include "FFITypes.mqh"

//+------------------------------------------------------------------+
//| Import Rust ZeroMQ DLL                                            |
//+------------------------------------------------------------------+
#import "sankey_copier_zmq.dll"

   //--- VictoriaLogs Functions ---
   int         vlogs_configure(string endpoint, string source);
   int         vlogs_add_entry(string level, string category, string message, string context_json);
   int         vlogs_flush();
   int         vlogs_disable();
   int         vlogs_buffer_size();

   // VictoriaLogs config accessors
   int         vlogs_config_get_bool(HANDLE_TYPE handle, string field_name);
   string      vlogs_config_get_string(HANDLE_TYPE handle, string field_name);
   int         vlogs_config_get_int(HANDLE_TYPE handle, string field_name);

   //--- Topic Generation Functions ---
   int         build_config_topic(string account_id, ushort &output[], int output_len);
   int         build_trade_topic(string master_id, string slave_id, ushort &output[], int output_len);
   int         get_global_config_topic(ushort &output[], int output_len);
   int         build_sync_topic_ffi(ushort &master_id[], ushort &slave_id[], ushort &output[], int output_len);
   int         get_sync_topic_prefix(string account_id, ushort &output[], int output_len);

   //--- EA Context Lifecycle ---
   HANDLE_TYPE ea_init(string account_id, string ea_type, string platform, long account_number, 
                       string broker, string account_name, string server, string currency, long leverage);
   void        ea_context_free(HANDLE_TYPE context);

   //--- Main Loop & Command Retrieval ---
   int         ea_manager_tick(HANDLE_TYPE context, double balance, double equity, int open_positions, int is_trade_allowed);
   int         ea_get_command(HANDLE_TYPE context, EaCommand &command);
   
   //--- Struct-Based Accessors ---
   int         ea_context_get_master_config(HANDLE_TYPE context, SMasterConfig &config);
   int         ea_context_get_slave_config(HANDLE_TYPE context, SSlaveConfig &config);
   int         ea_context_get_global_config(HANDLE_TYPE context, SGlobalConfig &config);
   int         ea_context_get_position_snapshot(HANDLE_TYPE context, SPositionInfo &positions[], int max_count);
   int         ea_context_get_position_snapshot_count(HANDLE_TYPE context);
   int         ea_context_get_position_snapshot_source_account(HANDLE_TYPE context, uchar &buffer[], int len);
   int         ea_context_get_sync_request(HANDLE_TYPE context, SSyncRequest &request);
   
   //--- Array Accessors for Slave Config ---
   int         ea_context_get_symbol_mappings_count(HANDLE_TYPE context);
   int         ea_context_get_symbol_mappings(HANDLE_TYPE context, SSymbolMapping &mappings[], int max_count);

   //--- Message Sending ---
   int         ea_send_register(HANDLE_TYPE context, uchar &output[], int output_len, string candidates);
   int         ea_send_heartbeat(HANDLE_TYPE context, double balance, double equity, int open_positions, 
                                 int is_trade_allowed, uchar &output[], int output_len);
   int         ea_send_unregister(HANDLE_TYPE context, uchar &output[], int output_len);
   
   //--- Config Request Management ---
   int         ea_context_should_request_config(HANDLE_TYPE context, int current_trade_allowed);
   void        ea_context_mark_config_requested(HANDLE_TYPE context);
   void        ea_context_reset(HANDLE_TYPE context);
   
   //--- High-Level Connection ---
   int         ea_connect(HANDLE_TYPE context, string push_addr, string sub_addr);
   int         ea_send_push(HANDLE_TYPE context, uchar &data[], int len);
   int         ea_receive_config(HANDLE_TYPE context, uchar &buffer[], int buffer_size);
   int         ea_subscribe_config(HANDLE_TYPE context, string topic);

   //--- Trade Signals (Master) ---
   //--- Trade Signals (Master) ---
   int         ea_send_open_signal(HANDLE_TYPE context, long ticket, string symbol, string order_type, 
                                   double lots, double price, double sl, double tp, long magic, string comment);
   int         ea_send_close_signal(HANDLE_TYPE context, long ticket, double close_ratio);
   int         ea_send_modify_signal(HANDLE_TYPE context, long ticket, double sl, double tp);

   //--- Sync/Config ---
   int         ea_send_request_config(HANDLE_TYPE context, uint version);
   int         ea_send_sync_request(HANDLE_TYPE context, string master_account, string last_sync_time);

   //--- Position Snapshot (Master) ---
   int         ea_send_position_snapshot(HANDLE_TYPE context, SPositionInfo &positions[], int count);

#import

#endif // FFI_IMPORTS_MQH
