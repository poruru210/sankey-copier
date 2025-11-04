//+------------------------------------------------------------------+
//|                                                   TestZmqDll.mq5 |
//|                        Copyright 2025, Forex Copier Project      |
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, Forex Copier Project"
#property version   "1.00"
#property script_show_inputs

//--- Import Rust ZeroMQ DLL
#import "forex_copier_zmq.dll"
   int    zmq_context_create();
   void   zmq_context_destroy(int context);
#import

//+------------------------------------------------------------------+
//| Script program start function                                    |
//+------------------------------------------------------------------+
void OnStart()
{
   Print("=== Testing ZeroMQ DLL ===");

   Print("Step 1: Creating ZMQ context...");
   int context = zmq_context_create();
   Print("Context handle: ", context);

   if(context < 0)
   {
      Print("ERROR: Failed to create context");
      return;
   }

   Print("Step 2: Destroying ZMQ context...");
   zmq_context_destroy(context);

   Print("=== Test completed successfully ===");
}
