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
//| Get candidates array from Config                                 |
//+------------------------------------------------------------------+
void GetCandidates(string &candidates[])
{
   GetSymbolSearchCandidates(candidates);
}

