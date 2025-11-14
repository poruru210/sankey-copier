//+------------------------------------------------------------------+
//|                                      SankeyCopierTrade.mqh        |
//|                        Copyright 2025, SANKEY Copier Project      |
//|                     Trade filtering and transformation            |
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, SANKEY Copier Project"

#include "SankeyCopierCommon.mqh"

//+------------------------------------------------------------------+
//| Check if trade should be processed based on filters              |
//+------------------------------------------------------------------+
bool ShouldProcessTrade(string symbol, int magic_number, bool enabled,
                        TradeFilters &filters)
{
   // Check if copying is enabled
   if(!enabled)
   {
      Print("Trade filtering: Copying is disabled");
      return false;
   }

   // Check allowed symbols filter
   if(ArraySize(filters.allowed_symbols) > 0)
   {
      bool symbol_found = false;
      for(int i = 0; i < ArraySize(filters.allowed_symbols); i++)
      {
         if(filters.allowed_symbols[i] == symbol)
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
   if(ArraySize(filters.blocked_symbols) > 0)
   {
      for(int i = 0; i < ArraySize(filters.blocked_symbols); i++)
      {
         if(filters.blocked_symbols[i] == symbol)
         {
            Print("Trade filtering: Symbol ", symbol, " is blocked");
            return false;
         }
      }
   }

   // Check allowed magic numbers filter
   if(ArraySize(filters.allowed_magic_numbers) > 0)
   {
      bool magic_found = false;
      for(int i = 0; i < ArraySize(filters.allowed_magic_numbers); i++)
      {
         if(filters.allowed_magic_numbers[i] == magic_number)
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
   if(ArraySize(filters.blocked_magic_numbers) > 0)
   {
      for(int i = 0; i < ArraySize(filters.blocked_magic_numbers); i++)
      {
         if(filters.blocked_magic_numbers[i] == magic_number)
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
//| Transform symbol using symbol mappings                           |
//+------------------------------------------------------------------+
string TransformSymbol(string source_symbol, SymbolMapping &mappings[])
{
   // Check if there's a mapping for this symbol
   for(int i = 0; i < ArraySize(mappings); i++)
   {
      if(mappings[i].source_symbol == source_symbol)
      {
         Print("Symbol transformation: ", source_symbol, " -> ", mappings[i].target_symbol);
         return mappings[i].target_symbol;
      }
   }

   // No mapping found, return original symbol
   return source_symbol;
}

//+------------------------------------------------------------------+
//| Apply lot multiplier to lot size                                 |
//+------------------------------------------------------------------+
double TransformLotSize(double source_lots, double multiplier)
{
   double transformed = source_lots * multiplier;
   transformed = NormalizeDouble(transformed, 2);

   Print("Lot transformation: ", source_lots, " * ", multiplier, " = ", transformed);

   return transformed;
}

//+------------------------------------------------------------------+
//| Reverse order type if configured                                 |
//+------------------------------------------------------------------+
string ReverseOrderType(string order_type, bool reverse_trade)
{
   if(!reverse_trade)
   {
      return order_type; // No reversal
   }

   // Reverse Buy <-> Sell
   if(order_type == "Buy")
   {
      Print("Order type reversed: Buy -> Sell");
      return "Sell";
   }
   else if(order_type == "Sell")
   {
      Print("Order type reversed: Sell -> Buy");
      return "Buy";
   }

   return order_type;
}

//+------------------------------------------------------------------+
//| Get order type string from enum (MT5)                            |
//+------------------------------------------------------------------+
#ifdef IS_MT5
string GetOrderTypeString(ENUM_POSITION_TYPE type)
{
   return (type == POSITION_TYPE_BUY) ? "Buy" : "Sell";
}

ENUM_ORDER_TYPE GetOrderTypeEnum(string type_str)
{
   if(type_str == "Buy") return ORDER_TYPE_BUY;
   if(type_str == "Sell") return ORDER_TYPE_SELL;
   return (ENUM_ORDER_TYPE)-1;
}
#endif

//+------------------------------------------------------------------+
//| Get order type string from enum (MT4)                            |
//+------------------------------------------------------------------+
#ifdef IS_MT4
string GetOrderTypeString(int type)
{
   switch(type)
   {
      case OP_BUY:       return "Buy";
      case OP_SELL:      return "Sell";
      case OP_BUYLIMIT:  return "BuyLimit";
      case OP_SELLLIMIT: return "SellLimit";
      case OP_BUYSTOP:   return "BuyStop";
      case OP_SELLSTOP:  return "SellStop";
      default:           return "Unknown";
   }
}

int GetOrderTypeEnum(string type_str)
{
   if(type_str == "Buy")       return OP_BUY;
   if(type_str == "Sell")      return OP_SELL;
   if(type_str == "BuyLimit")  return OP_BUYLIMIT;
   if(type_str == "SellLimit") return OP_SELLLIMIT;
   if(type_str == "BuyStop")   return OP_BUYSTOP;
   if(type_str == "SellStop")  return OP_SELLSTOP;
   return -1;
}
#endif

//+------------------------------------------------------------------+
//| Process configuration message (MessagePack)                      |
//+------------------------------------------------------------------+
void ProcessConfigMessage(uchar &msgpack_data[], int data_len,
                         string &current_master, string &trade_group_id,
                         bool &enabled, double &lot_multiplier,
                         bool &reverse_trade, int &config_version,
                         SymbolMapping &symbol_mappings[],
                         TradeFilters &filters,
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
   bool new_enabled = (config_get_bool(config_handle, "enabled") == 1);
   double new_lot_mult = config_get_double(config_handle, "lot_multiplier");
   bool new_reverse = (config_get_bool(config_handle, "reverse_trade") == 1);
   int new_version = config_get_int(config_handle, "config_version");

   // Log configuration values
   Print("Master Account: ", new_master);
   Print("Trade Group ID: ", new_group);
   Print("Enabled: ", new_enabled);
   Print("Lot Multiplier: ", new_lot_mult);
   Print("Reverse Trade: ", new_reverse);
   Print("Config Version: ", new_version);

   // TODO: Parse symbol mappings and filters from MessagePack
   // For now, skip arrays until we implement array support in DLL
   ArrayResize(symbol_mappings, 0);
   ArrayResize(filters.allowed_symbols, 0);
   ArrayResize(filters.blocked_symbols, 0);
   ArrayResize(filters.allowed_magic_numbers, 0);
   ArrayResize(filters.blocked_magic_numbers, 0);

   // Update global configuration
   enabled = new_enabled;
   lot_multiplier = new_lot_mult;
   reverse_trade = new_reverse;
   config_version = new_version;

   // Check if master/group changed
   if(new_master != current_master || new_group != trade_group_id)
   {
      Print("Master Account: ", current_master, " -> ", new_master);
      Print("Trade Group ID: ", trade_group_id, " -> ", new_group);

      // Update master and group
      current_master = new_master;
      trade_group_id = new_group;

      // Subscribe to new trade group
      if(zmq_socket_subscribe(zmq_trade_socket, trade_group_id) == 0)
      {
         Print("ERROR: Failed to subscribe to trade group: ", trade_group_id);
      }
      else
      {
         Print("Successfully subscribed to trade group: ", trade_group_id);
      }
   }

   // Free the config handle
   config_free(config_handle);

   Print("=== Configuration Updated ===");
}
