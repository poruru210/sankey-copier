//+------------------------------------------------------------------+
//|                                              SymbolUtils.mqh     |
//|                        Copyright 2025, SANKEY Copier Project     |
//|                     Common Symbol Collection Logic               |
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, SANKEY Copier Project"
#property strict

#include "ConfigFile.mqh" 

//+------------------------------------------------------------------+
//| Helper: Check if symbol allows trading                           |
//+------------------------------------------------------------------+
bool IsSymbolTradeAllowed(string symbol)
{
#ifdef __MQL5__
   return (SymbolInfoInteger(symbol, SYMBOL_TRADE_MODE) != SYMBOL_TRADE_MODE_DISABLED);
#else
   return (MarketInfo(symbol, MODE_TRADEALLOWED) != 0);
#endif
}

//+------------------------------------------------------------------+
//| Find best matching symbol from ALL available symbols (fuzzy matching)|
//| Returns the best match and automatically SELECTS it in Market Watch|
//+------------------------------------------------------------------+
string DetectBestMatch(string candidate)
{
   string best_match = "";
   int min_diff = 1000;
   
   // Search ALL symbols (not just Market Watch)
   int total = SymbolsTotal(false);
   
   for(int i=0; i<total; i++)
   {
      string current_symbol = SymbolName(i, false);
      
      // Skip disabled symbols (e.g. read-only, gray out)
      if(!IsSymbolTradeAllowed(current_symbol)) continue;
      
      // Exact match check
      if(current_symbol == candidate)
      {
         SymbolSelect(current_symbol, true);
         return current_symbol;
      }
      
      // Fuzzy match (shortest difference)
      if(StringFind(current_symbol, candidate) >= 0)
      {
         int diff = StringLen(current_symbol) - StringLen(candidate);
         if(diff < min_diff)
         {
            min_diff = diff;
            best_match = current_symbol;
         }
      }
   }
   
   // If match found, select it
   if(StringLen(best_match) > 0)
   {
      SymbolSelect(best_match, true);
   }
   
   return best_match;
}

//+------------------------------------------------------------------+
//| Detection Result Structure                                       |
//+------------------------------------------------------------------+
struct DetectedSymbolContext
{
   string prefix;
   string suffix;
   string specials; // Comma separated list of special symbols found (e.g. "GOLD.m,BTC")
};

//+------------------------------------------------------------------+
//| Detect Prefix and Suffix by checking standard pairs              |
//+------------------------------------------------------------------+
void DetectPrefixSuffix(string &prefix, string &suffix)
{
   prefix = "";
   suffix = "";
   
   string benchmarks[];
   GetDetectionBenchmarks(benchmarks);
   
   if(ArraySize(benchmarks) == 0)
   {
       // Fallback defaults if config is missing
       // Added GOLD for brokers using GOLD/GOLD# instead of XAUUSD
       string defaults[] = {"EURUSD", "GBPUSD", "USDJPY", "XAUUSD", "GOLD"};
       ArrayCopy(benchmarks, defaults);
   }
   
   int benchmark_count = ArraySize(benchmarks);
   
   // DEBUG: Log used benchmarks
   string benchmark_str = "";
   for(int k=0; k<benchmark_count; k++) benchmark_str += benchmarks[k] + (k < benchmark_count-1 ? ", " : "");
   PrintFormat("[SymbolUtils] Checking benchmarks (Trade Allowed Only): [%s]", benchmark_str);

   int total = SymbolsTotal(false); // Scan all symbols
   
   for(int b=0; b<benchmark_count; b++)
   {
      string base = benchmarks[b];
      
      for(int i=0; i<total; i++)
      {
         string current = SymbolName(i, false);
         
         // 1. Must be a tradeable symbol to be a valid context source
         if(!IsSymbolTradeAllowed(current)) continue;
         
         // 2. Check if current symbol *contains* benchmark base
         int pos = StringFind(current, base);
         if(pos >= 0)
         {
            // Found a candidate match
            string p = StringSubstr(current, 0, pos);
            string s = StringSubstr(current, pos + StringLen(base));
            
            // Validate structure: Prefix + Base + Suffix == Symbol
            if(p + base + s == current)
            {
               prefix = p;
               suffix = s;
               PrintFormat("[SymbolUtils] Context detected: Symbol='%s' -> Prefix='%s', Suffix='%s'", current, prefix, suffix);
               return; // Found valid tradeable pattern
            }
         }
      }
   }
}

//+------------------------------------------------------------------+
//| Detect Special Symbols (Indices, Metals, Crypto)                 |
//| Returns comma-separated list of found special symbols            |
//+------------------------------------------------------------------+
string DetectSpecials(string detected_prefix, string detected_suffix)
{
   string specials = "";
   
   // Dictionary of common special symbols to look for
   // If the base symbol (e.g. XAUUSD) is NOT found with standard prefix/suffix,
   // but a synonym (e.g. GOLD) IS found, we add it to specials.
   // Or if we just find "GOLD", we report it availability.
   
   string targets[] = {
      "XAUUSD", "GOLD", 
      "BTCUSD", "BTC",
      "US30", "DJ30", "WS30",
      "NAS100", "US100", "NDX100",
      "GER30", "DE30", "DAX30", "DAX40"
   };
   
   int count = ArraySize(targets);
   int total = SymbolsTotal(false);
   
   for(int t=0; t<count; t++)
   {
      string target = targets[t];
      
      // Try to find target "as is" or with prefix/suffix
      // But for "specials", we usually look for exact matches or variations that might NOT follow standard prefix/suffix.
      // Actually, if we found prefix "pro." and suffix ".m", we should expect "pro.GOLD.m".
      // But if we find "GOLD" instead of "XAUUSD", that's a special mapping case.
      
      // Strategy: Scan all symbols. If we find a symbol that CONTAINS target, add it to list?
      // No, that's too broad.
      
      // Better Strategy: Check existence of specific specialized names
      // patterns: [Prefix][Target][Suffix]
      
      string candidate = detected_prefix + target + detected_suffix;
      
      // Check if this specific candidate exists
      bool exists = false;
      
      // Optimized check: CustomSymbolExists is for custom symbols. 
      // MarketInfo/SymbolInfo works if we know the exact name.
      // But we might not know exact name if it differs from prefix/suffix.
      // Let's iterate again (slow but safe). 
      // Actually, SymbolsTotal is large (thousands).
      // Optimization: Try selecting it.
      
      if(IsSymbolTradeAllowed(candidate))
      {
          // Found exact match with standard pattern -> Not "Special" in the sense of weird mapping, 
          // BUT if the user's master uses XAUUSD and this is GOLD, we need to report GOLD.
          
          // Actually, "available_special_symbols" is just a list of "what special symbols do I have available?".
          // The UI compares this list with standard mappings.
          // So we should report valid tradeable symbols that look like our targets.
          
          if(StringLen(specials) > 0) specials += ",";
          specials += candidate;
      }
      else
      {
         // Try target without prefix/suffix? (Some brokers don't apply suffix to indices)
         if(target != candidate && IsSymbolTradeAllowed(target))
         {
             if(StringLen(specials) > 0) specials += ",";
             specials += target;
         }
      }
   }
   
   return specials;
}

//+------------------------------------------------------------------+
//| Main Detection Function                                          |
//+------------------------------------------------------------------+
void DetectSymbolContext(string &prefix, string &suffix, string &specials)
{
   DetectPrefixSuffix(prefix, suffix);
   specials = DetectSpecials(prefix, suffix);
}

// Old helpers can be deprecated or kept if used elsewhere
// ... Remove GetCandidates stub ...

