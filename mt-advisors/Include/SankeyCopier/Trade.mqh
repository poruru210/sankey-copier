//+------------------------------------------------------------------+
//|                                      SankeyCopierTrade.mqh        |
//|                        Copyright 2025, SANKEY Copier Project      |
//|                     Trade filtering and transformation            |
//+------------------------------------------------------------------+
// Purpose: Slave EA trade processing functions (filtering, transformation, config)
// Note: This file contains Slave-specific functionality
//       Consider renaming to SlaveTrade.mqh in future refactoring
#property copyright "Copyright 2025, SANKEY Copier Project"

#ifndef SANKEY_COPIER_TRADE_MQH
#define SANKEY_COPIER_TRADE_MQH

#include "Common.mqh"
#include "SlaveTypes.mqh"
// Note: Messages.mqh removed - using Common.mqh for declarations
#include "Logging.mqh"

//+------------------------------------------------------------------+
//| Send sync request message (local helper for ProcessConfigMessage) |
//| Creates transient PUSH socket to send SyncRequest to server      |
//+------------------------------------------------------------------+
bool SendSyncRequestMessage_Local(HANDLE_TYPE zmq_context, string server_address,
                                  string slave_account, string master_account)
{
   // Create temporary PUSH socket for sync request
   HANDLE_TYPE push_socket = zmq_socket_create(zmq_context, ZMQ_PUSH);
   if(push_socket < 0)
   {
      LogError(CAT_SYNC, "Failed to create sync request socket");
      return false;
   }

   if(zmq_socket_connect(push_socket, server_address) == 0)
   {
      LogError(CAT_SYNC, StringFormat("Failed to connect for sync request: %s", server_address));
      zmq_socket_destroy(push_socket);
      return false;
   }

   // Create and serialize SyncRequest message
   uchar buffer[];
   ArrayResize(buffer, MESSAGE_BUFFER_SIZE);
   int len = create_sync_request(slave_account, master_account, buffer, MESSAGE_BUFFER_SIZE);

   if(len <= 0)
   {
      LogError(CAT_SYNC, "Failed to create sync request message");
      zmq_socket_destroy(push_socket);
      return false;
   }

   // Resize buffer to actual size
   ArrayResize(buffer, len);

   // Send binary MessagePack data
   bool success = (zmq_socket_send_binary(push_socket, buffer, len) == 1);

   if(success)
      LogInfo(CAT_SYNC, StringFormat("Request sent to master: %s", master_account));
   else
      LogError(CAT_SYNC, "Failed to send sync request message");

   zmq_socket_destroy(push_socket);
   return success;
}

//+------------------------------------------------------------------+
//| Check if trade should be processed based on filters              |
//+------------------------------------------------------------------+
//+------------------------------------------------------------------+
//| Check if trade should be processed based on filters              |
//+------------------------------------------------------------------+
bool ShouldProcessTrade(string symbol, int magic_number, CopyConfig &config)
{
   // Check if the server explicitly allows new trades (runtime_status + auto-trade)
   if(!config.allow_new_orders)
   {
      LogDebug(CAT_TRADE, StringFormat("Trade filtering: allow_new_orders=%d runtime_status=%d",
            config.allow_new_orders ? 1 : 0, config.status));
      return false;
   }

   // Check allowed symbols filter
   if(ArraySize(config.filters.allowed_symbols) > 0)
   {
      bool symbol_found = false;
      for(int i = 0; i < ArraySize(config.filters.allowed_symbols); i++)
      {
         if(config.filters.allowed_symbols[i] == symbol)
         {
            symbol_found = true;
            break;
         }
      }

      if(!symbol_found)
      {
         LogDebug(CAT_TRADE, StringFormat("Trade filtering: Symbol %s not in allowed list", symbol));
         return false;
      }
   }

   // Check blocked symbols filter
   if(ArraySize(config.filters.blocked_symbols) > 0)
   {
      for(int i = 0; i < ArraySize(config.filters.blocked_symbols); i++)
      {
         if(config.filters.blocked_symbols[i] == symbol)
         {
            LogDebug(CAT_TRADE, StringFormat("Trade filtering: Symbol %s is blocked", symbol));
            return false;
         }
      }
   }

   // Check allowed magic numbers filter
   if(ArraySize(config.filters.allowed_magic_numbers) > 0)
   {
      bool magic_found = false;
      for(int i = 0; i < ArraySize(config.filters.allowed_magic_numbers); i++)
      {
         if(config.filters.allowed_magic_numbers[i] == magic_number)
         {
            magic_found = true;
            break;
         }
      }

      if(!magic_found)
      {
         LogDebug(CAT_TRADE, StringFormat("Trade filtering: Magic number %d not in allowed list", magic_number));
         return false;
      }
   }

   // Check blocked magic numbers filter
   if(ArraySize(config.filters.blocked_magic_numbers) > 0)
   {
      for(int i = 0; i < ArraySize(config.filters.blocked_magic_numbers); i++)
      {
         if(config.filters.blocked_magic_numbers[i] == magic_number)
         {
            LogDebug(CAT_TRADE, StringFormat("Trade filtering: Magic number %d is blocked", magic_number));
            return false;
         }
      }
   }

   // All checks passed
   return true;
}

//+------------------------------------------------------------------+
//| Process configuration message (MessagePack)                      |
//| Extended version with sync request support                       |
//+------------------------------------------------------------------+
void ProcessConfigMessage(uchar &msgpack_data[], int data_len,
                          CopyConfig &configs[],
                          HANDLE_TYPE zmq_trade_socket,
                          HANDLE_TYPE zmq_context = -1,
                          string server_address = "",
                          string slave_account = "")
{
   LogInfo(CAT_CONFIG, "Processing configuration message");

   // Parse MessagePack once and get a handle to the Slave config structure
   HANDLE_TYPE config_handle = parse_slave_config(msgpack_data, data_len);
   if(config_handle == 0)
   {
      LogError(CAT_CONFIG, "Failed to parse MessagePack Slave config");
      return;
   }

   // Extract fields from the parsed config using the handle
   string new_master = slave_config_get_string(config_handle, "master_account");
   // trade_group_id is no longer used for subscription, but we still read it for compatibility
   string new_group = slave_config_get_string(config_handle, "trade_group_id");

   if(new_master == "")
   {
      LogError(CAT_CONFIG, "Invalid config message received - missing master_account");
      slave_config_free(config_handle);
      return;
   }

   // Generate trade topic using FFI
   ushort topic_buffer[256];
   int len = build_trade_topic(new_master, slave_account, topic_buffer, 256);
   string new_trade_topic = "";
   if(len > 0) 
   {
      new_trade_topic = ShortArrayToString(topic_buffer);
   }
   else
   {
      LogError(CAT_CONFIG, "Failed to build trade topic via FFI");
      slave_config_free(config_handle);
      return;
   }

   // Extract extended configuration fields
   int new_status = slave_config_get_int(config_handle, "status");
   string lot_calc_mode_str = slave_config_get_string(config_handle, "lot_calculation_mode");
   int new_lot_calc_mode = (lot_calc_mode_str == "margin_ratio") ? LOT_CALC_MODE_MARGIN_RATIO : LOT_CALC_MODE_MULTIPLIER;
   double new_lot_mult = slave_config_get_double(config_handle, "lot_multiplier");
   bool new_reverse = (slave_config_get_bool(config_handle, "reverse_trade") == 1);
   int new_version = slave_config_get_int(config_handle, "config_version");
   double new_source_lot_min = slave_config_get_double(config_handle, "source_lot_min");
   double new_source_lot_max = slave_config_get_double(config_handle, "source_lot_max");
   double new_master_equity = slave_config_get_double(config_handle, "master_equity");

   // Extract Open Sync Policy fields
   string sync_mode_str = slave_config_get_string(config_handle, "sync_mode");
   int new_sync_mode = SYNC_MODE_SKIP;
   if(sync_mode_str == "limit_order")
      new_sync_mode = SYNC_MODE_LIMIT_ORDER;
   else if(sync_mode_str == "market_order")
      new_sync_mode = SYNC_MODE_MARKET_ORDER;
   int new_limit_order_expiry = slave_config_get_int(config_handle, "limit_order_expiry_min");
   double new_market_sync_max_pips = slave_config_get_double(config_handle, "market_sync_max_pips");
   int new_max_slippage = slave_config_get_int(config_handle, "max_slippage");
   bool new_copy_pending_orders = (slave_config_get_bool(config_handle, "copy_pending_orders") == 1);

   // Extract Trade Execution settings
   int new_max_retries = slave_config_get_int(config_handle, "max_retries");
   if(new_max_retries <= 0) new_max_retries = 3; // Default: 3 retries
   int new_max_signal_delay_ms = slave_config_get_int(config_handle, "max_signal_delay_ms");
   if(new_max_signal_delay_ms <= 0) new_max_signal_delay_ms = 5000; // Default: 5000ms
   bool new_use_pending_order_for_delayed = (slave_config_get_bool(config_handle, "use_pending_order_for_delayed") == 1);
   bool new_allow_new_orders = (slave_config_get_bool(config_handle, "allow_new_orders") == 1);

   // Log configuration values (compact format)
   LogInfo(CAT_CONFIG, StringFormat("Master: %s, Topic: %s, Status: %d", new_master, new_trade_topic, new_status));
   LogDebug(CAT_CONFIG, StringFormat("Lot mode: %s, multiplier: %.2f, reverse: %d", lot_calc_mode_str, new_lot_mult, new_reverse));
   LogDebug(CAT_CONFIG, StringFormat("Source lot: %.2f-%.2f, master_equity: %.2f", new_source_lot_min, new_source_lot_max, new_master_equity));
   LogDebug(CAT_CONFIG, StringFormat("Sync mode: %s, limit_expiry: %d min, max_pips: %.1f", sync_mode_str, new_limit_order_expiry, new_market_sync_max_pips));
   LogDebug(CAT_CONFIG, StringFormat("Execution: slippage=%d, retries=%d, delay=%dms, pending=%d, new_orders=%d",
         new_max_slippage, new_max_retries, new_max_signal_delay_ms, new_use_pending_order_for_delayed, new_allow_new_orders));
   LogDebug(CAT_CONFIG, StringFormat("Current configs count: %d", ArraySize(configs)));

   // Find existing config
   int index = -1;
   for(int i = 0; i < ArraySize(configs); i++)
   {
      if(configs[i].master_account == new_master)
      {
         index = i;
         break;
      }
   }

   LogDebug(CAT_CONFIG, StringFormat("Found index: %d", index));

   if(new_status == STATUS_NO_CONFIG)
   {
      // Remove configuration ONLY if status is NO_CONFIG (-1)
      if(index >= 0)
      {
         LogDebug(CAT_CONFIG, StringFormat("Removing configuration for master %s at index %d", new_master, index));

         // Shift remaining elements
         for(int i = index; i < ArraySize(configs) - 1; i++)
         {
            configs[i] = configs[i + 1];
         }
         ArrayResize(configs, ArraySize(configs) - 1);
         LogInfo(CAT_CONFIG, StringFormat("Configuration removed for %s. New count: %d", new_master, ArraySize(configs)));
      }
      else
      {
         LogDebug(CAT_CONFIG, StringFormat("Removal requested but config not found for %s", new_master));
      }

      // Free the config handle before returning
      slave_config_free(config_handle);
      return;
   }
   else
   {
      // For STATUS_DISABLED (0), ENABLED (1), CONNECTED (2):
      // Add or Update configuration so it remains visible in the list
      bool is_new_config = (index == -1);

      // Add or Update configuration
      if(index == -1)
      {
         // Add new
         LogDebug(CAT_CONFIG, StringFormat("Adding new configuration for %s", new_master));
         index = ArraySize(configs);
         ArrayResize(configs, index + 1);
         configs[index].master_account = new_master;

         // Subscribe to new trade topic
         if(zmq_socket_subscribe(zmq_trade_socket, new_trade_topic) == 0)
         {
            LogError(CAT_CONFIG, StringFormat("Failed to subscribe to trade topic: %s", new_trade_topic));
         }
         else
         {
            LogInfo(CAT_CONFIG, StringFormat("Subscribed to trade topic: %s", new_trade_topic));
         }
      }
      else
      {
         LogDebug(CAT_CONFIG, StringFormat("Updating existing configuration for %s at index %d", new_master, index));
         if(configs[index].trade_group_id != new_trade_topic)
         {
            // Topic changed, subscribe to new one
            if(zmq_socket_subscribe(zmq_trade_socket, new_trade_topic) == 0)
            {
                LogError(CAT_CONFIG, StringFormat("Failed to subscribe to trade topic: %s", new_trade_topic));
            }
            else
            {
                LogInfo(CAT_CONFIG, StringFormat("Subscribed to trade topic: %s", new_trade_topic));
            }
         }
      }
      
      // Update fields
      configs[index].trade_group_id = new_trade_topic;
      configs[index].status = new_status;
      configs[index].lot_calculation_mode = new_lot_calc_mode;
      configs[index].lot_multiplier = new_lot_mult;
      configs[index].reverse_trade = new_reverse;
      configs[index].config_version = new_version;
      configs[index].source_lot_min = new_source_lot_min;
      configs[index].source_lot_max = new_source_lot_max;
      configs[index].master_equity = new_master_equity;

      // Open Sync Policy settings
      configs[index].sync_mode = new_sync_mode;
      configs[index].limit_order_expiry_min = new_limit_order_expiry;
      configs[index].market_sync_max_pips = new_market_sync_max_pips;
      configs[index].max_slippage = new_max_slippage;
      configs[index].copy_pending_orders = new_copy_pending_orders;

      // Trade Execution settings
      configs[index].max_retries = new_max_retries;
      configs[index].max_signal_delay_ms = new_max_signal_delay_ms;
      configs[index].use_pending_order_for_delayed = new_use_pending_order_for_delayed;
      configs[index].allow_new_orders = new_allow_new_orders;

      // Parse symbol prefix/suffix from MessagePack
      configs[index].symbol_prefix = slave_config_get_string(config_handle, "symbol_prefix");
      configs[index].symbol_suffix = slave_config_get_string(config_handle, "symbol_suffix");

      // Parse symbol mappings from MessagePack
      int mapping_count = slave_config_get_symbol_mappings_count(config_handle);
      ArrayResize(configs[index].symbol_mappings, mapping_count);
      for(int m = 0; m < mapping_count; m++)
      {
         configs[index].symbol_mappings[m].source_symbol = slave_config_get_symbol_mapping_source(config_handle, m);
         configs[index].symbol_mappings[m].target_symbol = slave_config_get_symbol_mapping_target(config_handle, m);
      }

      // Log symbol mappings if any
      if(mapping_count > 0)
      {
         LogDebug(CAT_CONFIG, StringFormat("Symbol Mappings (%d):", mapping_count));
         for(int m = 0; m < mapping_count; m++)
         {
            LogDebug(CAT_CONFIG, StringFormat("  %s -> %s",
                  configs[index].symbol_mappings[m].source_symbol,
                  configs[index].symbol_mappings[m].target_symbol));
         }
      }

      // Parse filters from MessagePack
      ArrayResize(configs[index].filters.allowed_symbols, 0);
      ArrayResize(configs[index].filters.blocked_symbols, 0);
      ArrayResize(configs[index].filters.blocked_magic_numbers, 0);

      // Parse allowed_magic_numbers filter
      int magic_count = slave_config_get_allowed_magic_count(config_handle);
      ArrayResize(configs[index].filters.allowed_magic_numbers, magic_count);
      for(int m = 0; m < magic_count; m++)
      {
         configs[index].filters.allowed_magic_numbers[m] = slave_config_get_allowed_magic_at(config_handle, m);
      }
      if(magic_count > 0)
      {
         LogDebug(CAT_CONFIG, StringFormat("Allowed Magic Numbers (%d)", magic_count));
      }

      // Send SyncRequest when:
      // 1. New config is added (not just updated)
      // 2. Status is CONNECTED (Master is online)
      // 3. sync_mode is not SKIP (user wants to sync existing positions)
      // 4. ZMQ context is valid (caller provided sync parameters)
      if(is_new_config &&
         new_status == STATUS_CONNECTED &&
         new_sync_mode != SYNC_MODE_SKIP &&
         zmq_context >= 0 &&
         server_address != "" &&
         slave_account != "")
      {
         LogInfo(CAT_SYNC, StringFormat("Triggering position sync request. Mode: %s",
               (new_sync_mode == SYNC_MODE_LIMIT_ORDER) ? "LIMIT_ORDER" : "MARKET_ORDER"));

         if(SendSyncRequestMessage_Local(zmq_context, server_address, slave_account, new_master))
         {
            LogInfo(CAT_SYNC, StringFormat("SyncRequest sent to master: %s", new_master));
         }
         else
         {
            LogError(CAT_SYNC, StringFormat("Failed to send SyncRequest to master: %s", new_master));
         }
      }
   }

   // Free the config handle
   slave_config_free(config_handle);

   LogInfo(CAT_CONFIG, "Configuration updated");
}

//+------------------------------------------------------------------+
//| Normalize lot size based on symbol properties                    |
//+------------------------------------------------------------------+
double NormalizeLotSize(double lots, string symbol)
{
   double step = SymbolInfoDouble(symbol, SYMBOL_VOLUME_STEP);
   double min = SymbolInfoDouble(symbol, SYMBOL_VOLUME_MIN);
   double max = SymbolInfoDouble(symbol, SYMBOL_VOLUME_MAX);
   
   if(step <= 0) return lots;
   
   // Normalize to step
   double normalized = MathFloor(lots / step + 0.5) * step;
   
   // Clamp to min/max
   if(normalized < min) normalized = min;
   if(normalized > max) normalized = max;
   
   // Normalize decimals to avoid floating point errors (e.g. 0.100000001)
   // Use 8 decimals as safe upper bound for volume precision
   return NormalizeDouble(normalized, 8);
}

//+------------------------------------------------------------------+
//| Check if symbol matches prefix/suffix filter                     |
//+------------------------------------------------------------------+
bool MatchesSymbolFilter(string symbol, string prefix, string suffix)
{
   // If no filter, everything matches
   if(prefix == "" && suffix == "") return true;
   
   // Check prefix
   if(prefix != "")
   {
      if(StringFind(symbol, prefix) != 0) return false;
   }
   
   // Check suffix
   if(suffix != "")
   {
      int suffix_len = StringLen(suffix);
      int symbol_len = StringLen(symbol);
      
      if(symbol_len < suffix_len) return false;
      
      string symbol_suffix = StringSubstr(symbol, symbol_len - suffix_len);
      if(symbol_suffix != suffix) return false;
   }
   
   return true;
}

//+------------------------------------------------------------------+
//| Remove prefix and suffix from symbol                             |
//+------------------------------------------------------------------+
string GetCleanSymbol(string symbol, string prefix, string suffix)
{
   string clean = symbol;
   
   // Remove prefix
   if(prefix != "" && StringFind(clean, prefix) == 0)
   {
      clean = StringSubstr(clean, StringLen(prefix));
   }
   
   // Remove suffix
   if(suffix != "")
   {
      int suffix_len = StringLen(suffix);
      int clean_len = StringLen(clean);
      
      if(clean_len >= suffix_len)
      {
         string current_suffix = StringSubstr(clean, clean_len - suffix_len);
         if(current_suffix == suffix)
         {
            clean = StringSubstr(clean, 0, clean_len - suffix_len);
         }
      }
   }
   
   return clean;
}

//+------------------------------------------------------------------+
//| Add prefix and suffix to symbol                                  |
//+------------------------------------------------------------------+
string GetLocalSymbol(string symbol, string prefix, string suffix)
{
   string local = symbol;
   
   // Add prefix
   if(prefix != "")
   {
      local = prefix + local;
   }
   
   // Add suffix
   if(suffix != "")
   {
      local = local + suffix;
   }
   
   return local;
}

//+------------------------------------------------------------------+
//| Parse symbol mapping string (Format: "Source=Target,Src2=Tgt2")  |
//+------------------------------------------------------------------+
void ParseSymbolMappingString(string mapping_str, SymbolMapping &mappings[])
{
   ArrayResize(mappings, 0);
   
   if(mapping_str == "") return;
   
   string pairs[];
   int pair_count = StringSplit(mapping_str, ',', pairs);
   
   for(int i = 0; i < pair_count; i++)
   {
      string pair = pairs[i];
      StringTrimLeft(pair);
      StringTrimRight(pair);
      
      if(pair == "") continue;
      
      string parts[];
      int part_count = StringSplit(pair, '=', parts);
      
      if(part_count == 2)
      {
         string source = parts[0];
         string target = parts[1];
         
         StringTrimLeft(source);
         StringTrimRight(source);
         StringTrimLeft(target);
         StringTrimRight(target);
         
         if(source != "" && target != "")
         {
            int size = ArraySize(mappings);
            ArrayResize(mappings, size + 1);
            mappings[size].source_symbol = source;
            mappings[size].target_symbol = target;
         }
      }
   }
}

//+------------------------------------------------------------------+
//| Check if lot size is within filter range                         |
//+------------------------------------------------------------------+
bool IsLotWithinFilter(double lots, double lot_min, double lot_max)
{
   // If min filter is set and lots is below it, reject
   if(lot_min > 0 && lots < lot_min)
   {
      LogDebug(CAT_TRADE, StringFormat("Lot filter: %.2f is below minimum %.2f", lots, lot_min));
      return false;
   }

   // If max filter is set and lots is above it, reject
   if(lot_max > 0 && lots > lot_max)
   {
      LogDebug(CAT_TRADE, StringFormat("Lot filter: %.2f is above maximum %.2f", lots, lot_max));
      return false;
   }

   return true;
}

//+------------------------------------------------------------------+
//| Transform lot size based on calculation mode                     |
//+------------------------------------------------------------------+
double TransformLotSize(double lots, CopyConfig &config, string symbol)
{
   double new_lots = 0;

   if(config.lot_calculation_mode == LOT_CALC_MODE_MARGIN_RATIO)
   {
      // Margin ratio mode: slave_lot = master_lot * (slave_equity / master_equity)
      double slave_equity = GetAccountEquity();
      double master_equity = config.master_equity;

      if(master_equity > 0)
      {
         double ratio = slave_equity / master_equity;
         new_lots = lots * ratio;
         LogDebug(CAT_TRADE, StringFormat("Margin ratio mode: slave_equity=%.2f, master_equity=%.2f, ratio=%.4f, lots=%.2f -> %.2f",
               slave_equity, master_equity, ratio, lots, new_lots));
      }
      else
      {
         LogWarn(CAT_TRADE, "Master equity is 0 or not available, using 1:1 ratio");
         new_lots = lots;
      }
   }
   else
   {
      // Default: Multiplier mode
      new_lots = lots * config.lot_multiplier;
   }

   return NormalizeLotSize(new_lots, symbol);
}

//+------------------------------------------------------------------+
//| Legacy transform lot size (for backwards compatibility)          |
//+------------------------------------------------------------------+
double TransformLotSize(double lots, double multiplier, string symbol)
{
   double new_lots = lots * multiplier;
   return NormalizeLotSize(new_lots, symbol);
}

//+------------------------------------------------------------------+
//| Reverse order type if enabled                                    |
//| Uses PascalCase format matching relay-server's OrderType enum    |
//+------------------------------------------------------------------+
string ReverseOrderType(string type, bool reverse)
{
   if(!reverse) return type;

   if(type == "Buy") return "Sell";
   if(type == "Sell") return "Buy";
   if(type == "BuyLimit") return "SellLimit";
   if(type == "SellLimit") return "BuyLimit";
   if(type == "BuyStop") return "SellStop";
   if(type == "SellStop") return "BuyStop";

   return type;
}

#endif // SANKEY_COPIER_TRADE_MQH
