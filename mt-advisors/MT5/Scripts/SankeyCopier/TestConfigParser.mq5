//+------------------------------------------------------------------+
//|                                           TestConfigParser.mq5   |
//|                        Copyright 2025, SANKEY Copier Project     |
//|                            Test Script for ConfigFile.mqh        |
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, SANKEY Copier Project"
#property script_show_inputs

#include <SankeyCopier/ConfigFile.mqh>

//+------------------------------------------------------------------+
//| Script program start function                                    |
//+------------------------------------------------------------------+
void OnStart()
{
   Print("=== Running TestConfigParser ===");
   
   // 1. Force reload config to ensure we read from disk
   ReloadConfig();
   
   // 2. Verify Config Loading State
   // Note: g_ConfigLoaded is module-local in MQH but shared if included? 
   // Actually variables in MQH are static to the compilation unit.
   
   if(!ConfigFileExists())
   {
      Print("WARNING: sankey_copier.ini not found. Test will use defaults.");
   }
   else
   {
      Print("INFO: sankey_copier.ini found.");
   }

   // 3. Verify Candidates
   string candidates[];
   GetSymbolSearchCandidates(candidates);
   
   int count = ArraySize(candidates);
   PrintFormat("Candidates found: %d", count);
   
   for(int i=0; i<count; i++)
   {
      PrintFormat("Candidate[%d]: '%s'", i, candidates[i]);
   }
   
   // 4. Verify Ports (Regression Check)
   PrintFormat("ReceiverPort: %d", GetReceiverPort());
   PrintFormat("PublisherPort: %d", GetPublisherPort());

   Print("=== TestConfigParser Completed ===");
}
