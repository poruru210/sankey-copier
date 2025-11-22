//+------------------------------------------------------------------+
//|                                      SankeyCopierTrade.mqh        |
//|                        Copyright 2025, SANKEY Copier Project      |
//|                     Trade filtering and transformation            |
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, SANKEY Copier Project"

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
                          int zmq_trade_socket)
{
   Print("=== Processing Configuration Message ===");

   // Parse MessagePack once and get a handle to the config structure
   HANDLE_TYPE config_handle = parse_message(msgpack_data, data_len);
   if(config_handle == 0)
   {
      Print("ERROR: Failed to parse MessagePack config");
      return;
   }

   // Extract fields from the parsed config using the handle
   string new_master = config_get_string(config_handle, "master_account");
   string new_group = config_get_string(config_handle, "trade_group_id");

   if(new_master == "" || new_group == "")
   {
      Print("ERROR: Invalid config message received");
      config_free(config_handle);
      return;
   }

   // Extract extended configuration fields
   int new_status = config_get_int(config_handle, "status");
   double new_lot_mult = config_get_double(config_handle, "lot_multiplier");
   bool new_reverse = (config_get_bool(config_handle, "reverse_trade") == 1);
   int new_version = config_get_int(config_handle, "config_version");

   // Log configuration values
   Print("Master Account: ", new_master);
   Print("Trade Group ID: ", new_group);
   Print("Status: ", new_status, " (0=DISABLED, 1=ENABLED, 2=CONNECTED)");
   Print("Lot Multiplier: ", new_lot_mult);
   Print("Reverse Trade: ", new_reverse);
   Print("Config Version: ", new_version);

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
      config_free(config_handle);
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
      configs[index].lot_multiplier = new_lot_mult;
      configs[index].reverse_trade = new_reverse;
      configs[index].config_version = new_version;
      
      // TODO: Parse symbol mappings and filters from MessagePack
      // For now, skip arrays until we implement array support in DLL
      ArrayResize(configs[index].symbol_mappings, 0);
      ArrayResize(configs[index].filters.allowed_symbols, 0);
      ArrayResize(configs[index].filters.blocked_symbols, 0);
      ArrayResize(configs[index].filters.allowed_magic_numbers, 0);
      ArrayResize(configs[index].filters.blocked_magic_numbers, 0);
   }

   // Free the config handle
   config_free(config_handle);

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
//| Transform lot size based on multiplier                           |
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
