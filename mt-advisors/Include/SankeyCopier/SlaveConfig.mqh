//+------------------------------------------------------------------+
//|                                     SankeyCopierSlaveConfig.mqh  |
//|                        Copyright 2025, SANKEY Copier Project      |
//|                     Slave EA configuration processing            |
//+------------------------------------------------------------------+
// Purpose: Slave EA configuration processing, filtering, and transformation
// Note: Renamed from Trade.mqh for clarity - this file is Slave-specific
#property copyright "Copyright 2025, SANKEY Copier Project"

#ifndef SANKEY_COPIER_SLAVE_CONFIG_MQH
#define SANKEY_COPIER_SLAVE_CONFIG_MQH

#include "SlaveContext.mqh"
#include "SlaveTypes.mqh"
// Note: Messages.mqh removed - using SlaveContext for declarations
#include "Logging.mqh"

// SendSyncRequestMessage_Local removed - using SlaveContextWrapper

// Note: ShouldProcessTrade removed - trade filtering is now handled by Rust (mt-bridge)

//+------------------------------------------------------------------+
//| Process configuration message from struct (Stateful FFI)         |
//+------------------------------------------------------------------+
void ProcessSlaveConfig(SSlaveConfig &config,
                        CopyConfig &configs[],
                        SlaveContextWrapper &context,
                        string slave_account)
{
   // Extract fields from the struct
   string new_master = CharArrayToString(config.master_account);
   string new_group = CharArrayToString(config.trade_group_id);

   if(new_master == "")
   {
      LogError(CAT_CONFIG, "Invalid config message received - missing master_account");
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
      return;
   }

   // Extract extended configuration fields
   int new_status = config.status;
   int new_lot_calc_mode = config.lot_calculation_mode; // 0=Multiplier, 1=MarginRatio
   string lot_calc_mode_str = (new_lot_calc_mode == LOT_CALC_MODE_MARGIN_RATIO) ? "margin_ratio" : "multiplier";

   double new_lot_mult = config.lot_multiplier;
   bool new_reverse = (config.reverse_trade != 0);
   int new_version = (int)config.config_version;
   double new_source_lot_min = config.source_lot_min;
   double new_source_lot_max = config.source_lot_max;

   double new_master_equity = config.master_equity;

   // Extract Open Sync Policy fields
   int new_sync_mode = config.sync_mode;
   string sync_mode_str = "skip";
   if(new_sync_mode == SYNC_MODE_LIMIT_ORDER) sync_mode_str = "limit_order";
   if(new_sync_mode == SYNC_MODE_MARKET_ORDER) sync_mode_str = "market_order";

   int new_limit_order_expiry = config.limit_order_expiry_min;
   double new_market_sync_max_pips = config.market_sync_max_pips;
   int new_max_slippage = config.max_slippage;
   bool new_copy_pending_orders = (config.copy_pending_orders != 0);

   // Extract Trade Execution settings
   int new_max_retries = config.max_retries;
   if(new_max_retries <= 0) new_max_retries = 3; // Default: 3 retries
   int new_max_signal_delay_ms = config.max_signal_delay_ms;
   if(new_max_signal_delay_ms <= 0) new_max_signal_delay_ms = 5000; // Default: 5000ms
   bool new_use_pending_order_for_delayed = (config.use_pending_order_for_delayed != 0);
   bool new_allow_new_orders = (config.allow_new_orders != 0);

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
         if(!context.SubscribeConfig(new_trade_topic))
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
            if(!context.SubscribeConfig(new_trade_topic))
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

      // Parse symbol prefix/suffix
      configs[index].symbol_prefix = CharArrayToString(config.symbol_prefix);
      configs[index].symbol_suffix = CharArrayToString(config.symbol_suffix);

      // Parse symbol mappings (Separately via Accessor)
      SSymbolMapping mapping_arr[];
      if(context.GetSymbolMappings(mapping_arr))
      {
          int mapping_count = ArraySize(mapping_arr);
          ArrayResize(configs[index].symbol_mappings, mapping_count);
          for(int m = 0; m < mapping_count; m++)
          {
             configs[index].symbol_mappings[m].source_symbol = CharArrayToString(mapping_arr[m].source);
             configs[index].symbol_mappings[m].target_symbol = CharArrayToString(mapping_arr[m].target);
          }

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
      }
      else
      {
          // No mappings or failed to get
          ArrayResize(configs[index].symbol_mappings, 0);
      }

      // Parse filters from MessagePack (Skipping dynamic arrays for now if not in Struct)
      // Allowed magic numbers are dynamic, need separate accessor if strictly necessary.
      // Current structs don't support dynamic arrays.
      // We assume basic filters are enough or we add separate accessors later.
      // For now, clear them to avoid stale data.
      ArrayResize(configs[index].filters.allowed_symbols, 0);
      ArrayResize(configs[index].filters.blocked_symbols, 0);
      ArrayResize(configs[index].filters.blocked_magic_numbers, 0);
      ArrayResize(configs[index].filters.allowed_magic_numbers, 0);

      // Send SyncRequest when needed
       if(is_new_config &&
          new_status == STATUS_CONNECTED &&
          new_sync_mode != SYNC_MODE_SKIP &&
          context.IsInitialized())
       {
          LogInfo(CAT_SYNC, StringFormat("Triggering position sync request. Mode: %s",
                (new_sync_mode == SYNC_MODE_LIMIT_ORDER) ? "LIMIT_ORDER" : "MARKET_ORDER"));

          if(context.SendSyncRequest(new_master))
          {
             LogInfo(CAT_SYNC, StringFormat("SyncRequest sent to master: %s", new_master));
          }
          else
          {
             LogError(CAT_SYNC, StringFormat("Failed to send SyncRequest to master: %s", new_master));
          }
       }
   }

   LogInfo(CAT_CONFIG, "Configuration updated");
}

// ProcessConfigMessage (Legacy wrapper) removed

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

// Note: ParseSymbolMappingString removed - symbol mappings now retrieved via FFI
// Note: IsLotWithinFilter removed - lot filtering is now handled by Rust (mt-bridge)

//+------------------------------------------------------------------+
//| Transform lot size based on calculation mode                     |
//| Note: This logic is partially duplicated in mt-bridge.           |
//| Kept here for PositionSnapshot Sync which bypasses EaContext.    |
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
//| Note: Logic also present in mt-bridge.                           |
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

#endif // SANKEY_COPIER_SLAVE_CONFIG_MQH
