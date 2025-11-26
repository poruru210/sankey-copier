//+------------------------------------------------------------------+
//|                                      SankeyCopierTrade.mqh        |
//|                        Copyright 2025, SANKEY Copier Project      |
//|                     Trade filtering and transformation            |
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, SANKEY Copier Project"

#ifndef SANKEY_COPIER_TRADE_MQH
#define SANKEY_COPIER_TRADE_MQH

#include "Common.mqh"

//+------------------------------------------------------------------+
//| Check if trade should be processed based on filters              |
//+------------------------------------------------------------------+
//+------------------------------------------------------------------+
//| Check if trade should be processed based on filters              |
//+------------------------------------------------------------------+
bool ShouldProcessTrade(string symbol, int magic_number, CopyConfig &config)
{
   // Check if copying is enabled and connected
   // STATUS_CONNECTED (2) means both Slave is enabled AND Master is connected
   if(config.status != STATUS_CONNECTED)
   {
      Print("Trade filtering: Status is ", config.status, " (need STATUS_CONNECTED=2)");
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
         Print("Trade filtering: Symbol ", symbol, " not in allowed list");
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
            Print("Trade filtering: Symbol ", symbol, " is blocked");
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
         Print("Trade filtering: Magic number ", magic_number, " not in allowed list");
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
            Print("Trade filtering: Magic number ", magic_number, " is blocked");
            return false;
         }
      }
   }

   // All checks passed
   return true;
}

//+------------------------------------------------------------------+
//| Process configuration message (MessagePack)                      |
//+------------------------------------------------------------------+
void ProcessConfigMessage(uchar &msgpack_data[], int data_len,
                          CopyConfig &configs[],
                          HANDLE_TYPE zmq_trade_socket)
{
   Print("=== Processing Configuration Message ===");

   // Parse MessagePack once and get a handle to the Slave config structure
   HANDLE_TYPE config_handle = parse_slave_config(msgpack_data, data_len);
   if(config_handle == 0)
   {
      Print("ERROR: Failed to parse MessagePack Slave config");
      return;
   }

   // Extract fields from the parsed config using the handle
   string new_master = slave_config_get_string(config_handle, "master_account");
   string new_group = slave_config_get_string(config_handle, "trade_group_id");

   if(new_master == "" || new_group == "")
   {
      Print("ERROR: Invalid config message received");
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

   // Log configuration values
   Print("Master Account: ", new_master);
   Print("Trade Group ID: ", new_group);
   Print("Status: ", new_status, " (0=DISABLED, 1=ENABLED, 2=CONNECTED)");
   Print("Lot Calculation Mode: ", lot_calc_mode_str, " (", new_lot_calc_mode, ")");
   Print("Lot Multiplier: ", new_lot_mult);
   Print("Reverse Trade: ", new_reverse);
   Print("Config Version: ", new_version);
   Print("Source Lot Min: ", new_source_lot_min);
   Print("Source Lot Max: ", new_source_lot_max);
   Print("Master Equity: ", new_master_equity);

   Print("DEBUG: Current configs count: ", ArraySize(configs));
   for(int i=0; i<ArraySize(configs); i++) Print("DEBUG: Config[", i, "]: ", configs[i].master_account);

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
   
   Print("DEBUG: Found index: ", index);

   if(new_status == STATUS_DISABLED)
   {
      // Remove configuration if disabled
      if(index >= 0)
      {
         Print("DEBUG: Removing configuration for master ", new_master, " at index ", index);
         
         // Shift remaining elements
         for(int i = index; i < ArraySize(configs) - 1; i++)
         {
            configs[i] = configs[i + 1];
         }
         ArrayResize(configs, ArraySize(configs) - 1);
         Print("DEBUG: Configuration removed. New count: ", ArraySize(configs));
      }
      else
      {
         Print("DEBUG: Removal requested but config not found for ", new_master);
      }
      
      // Free the config handle before returning
      slave_config_free(config_handle);
      return;
   }
   else
   {
      // Add or Update configuration
      if(index == -1)
      {
         // Add new
         Print("DEBUG: Adding new configuration for ", new_master);
         index = ArraySize(configs);
         ArrayResize(configs, index + 1);
         configs[index].master_account = new_master;
         
         // Subscribe to new trade group
         if(zmq_socket_subscribe(zmq_trade_socket, new_group) == 0)
         {
            Print("ERROR: Failed to subscribe to trade group: ", new_group);
         }
         else
         {
            Print("Successfully subscribed to trade group: ", new_group);
         }
      }
      else
      {
         Print("DEBUG: Updating existing configuration for ", new_master, " at index ", index);
         if(configs[index].trade_group_id != new_group)
         {
            // Group changed, subscribe to new one
            if(zmq_socket_subscribe(zmq_trade_socket, new_group) == 0)
            {
                Print("ERROR: Failed to subscribe to trade group: ", new_group);
            }
            else
            {
                Print("Successfully subscribed to trade group: ", new_group);
            }
         }
      }
      
      // Update fields
      configs[index].trade_group_id = new_group;
      configs[index].status = new_status;
      configs[index].lot_calculation_mode = new_lot_calc_mode;
      configs[index].lot_multiplier = new_lot_mult;
      configs[index].reverse_trade = new_reverse;
      configs[index].config_version = new_version;
      configs[index].source_lot_min = new_source_lot_min;
      configs[index].source_lot_max = new_source_lot_max;
      configs[index].master_equity = new_master_equity;

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
         Print("Symbol Mappings (", mapping_count, "):");
         for(int m = 0; m < mapping_count; m++)
         {
            Print("  ", configs[index].symbol_mappings[m].source_symbol, " -> ",
                  configs[index].symbol_mappings[m].target_symbol);
         }
      }

      // TODO: Parse filters from MessagePack (not yet implemented in DLL)
      ArrayResize(configs[index].filters.allowed_symbols, 0);
      ArrayResize(configs[index].filters.blocked_symbols, 0);
      ArrayResize(configs[index].filters.allowed_magic_numbers, 0);
      ArrayResize(configs[index].filters.blocked_magic_numbers, 0);
   }

   // Free the config handle
   slave_config_free(config_handle);

   Print("=== Configuration Updated ===");
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
      Print("Lot filter: ", lots, " is below minimum ", lot_min);
      return false;
   }

   // If max filter is set and lots is above it, reject
   if(lot_max > 0 && lots > lot_max)
   {
      Print("Lot filter: ", lots, " is above maximum ", lot_max);
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
         Print("Margin ratio mode: slave_equity=", slave_equity,
               ", master_equity=", master_equity,
               ", ratio=", ratio,
               ", lots=", lots, " -> ", new_lots);
      }
      else
      {
         Print("WARNING: Master equity is 0 or not available, using 1:1 ratio");
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
//+------------------------------------------------------------------+
string ReverseOrderType(string type, bool reverse)
{
   if(!reverse) return type;
   
   if(type == "ORDER_TYPE_BUY") return "ORDER_TYPE_SELL";
   if(type == "ORDER_TYPE_SELL") return "ORDER_TYPE_BUY";
   if(type == "ORDER_TYPE_BUY_LIMIT") return "ORDER_TYPE_SELL_LIMIT";
   if(type == "ORDER_TYPE_SELL_LIMIT") return "ORDER_TYPE_BUY_LIMIT";
   if(type == "ORDER_TYPE_BUY_STOP") return "ORDER_TYPE_SELL_STOP";
   if(type == "ORDER_TYPE_SELL_STOP") return "ORDER_TYPE_BUY_STOP";
   
   return type;
}

#endif // SANKEY_COPIER_TRADE_MQH
