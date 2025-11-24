//+------------------------------------------------------------------+
//|                                        SankeyCopierSlave.mq5      |
//|                        Copyright 2025, SANKEY Copier Project      |
//|                                                                  |
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, SANKEY Copier Project"
#property link      ""
#property version   "1.00"  // VERSION_PLACEHOLDER
#property icon      "app.ico"

#include "../Include/Trade/Trade.mqh"

//--- Include common headers
#include "../Include/SankeyCopier/Common.mqh"
#include "../Include/SankeyCopier/Zmq.mqh"
#include "../Include/SankeyCopier/Mapping.mqh"
#include "../Include/SankeyCopier/GridPanel.mqh"
#include "../Include/SankeyCopier/Messages.mqh"
#include "../Include/SankeyCopier/Trade.mqh"

//--- Input parameters
input string   RelayServerAddress = DEFAULT_ADDR_PULL;       // Address to send heartbeats/requests (PULL)
input string   TradeSignalSourceAddress = DEFAULT_ADDR_PUB_TRADE; // Address to receive trade signals (SUB)
input string   ConfigSourceAddress = DEFAULT_ADDR_PUB_CONFIG;     // Address to receive configuration (SUB)
input int      Slippage = 3;
input int      MaxRetries = 3;
input bool     AllowNewOrders = true;
input bool     AllowCloseOrders = true;
input int      MaxSignalDelayMs = 5000;                      // Maximum allowed signal delay (milliseconds)
input bool     UsePendingOrderForDelayed = false;            // Use pending order for delayed signals
input string   SymbolPrefix = "";       // Symbol prefix to add (e.g. "pro.")
input string   SymbolSuffix = "";       // Symbol suffix to add (e.g. ".m")
input string   SymbolMap = "";          // Symbol mapping (e.g. "XAUUSD=GOLD,BTCUSD=Bitcoin")
input bool     ShowConfigPanel = true;                       // Show configuration panel on chart
input int      PanelWidth = 280;                             // Configuration panel width (pixels)

//--- Global variables
string      AccountID;                  // Auto-generated from broker + account number
HANDLE_TYPE g_zmq_context = -1;
HANDLE_TYPE g_zmq_trade_socket = -1;    // Socket for receiving trade signals
HANDLE_TYPE g_zmq_config_socket = -1;   // Socket for receiving configuration
CTrade      g_trade;
TicketMapping g_order_map[];
PendingTicketMapping g_pending_order_map[];
SymbolMapping g_local_mappings[];
bool        g_initialized = false;
datetime    g_last_heartbeat = 0;
bool        g_config_requested = false; // Track if config has been requested
bool        g_last_trade_allowed = false; // Track auto-trading state for change detection

//--- Extended configuration variables (from ConfigMessage)
CopyConfig     g_configs[];                      // Array of active configurations
bool           g_has_received_config = false;    // Track if we have received at least one config

//--- Configuration panel
CGridPanel     g_config_panel;                   // Grid panel for displaying configuration
int            g_last_config_count = -1;         // Debug: track config count changes

//+------------------------------------------------------------------+
//| Expert initialization function                                     |
//+------------------------------------------------------------------+
int OnInit()
{
   Print("=== SankeyCopier Slave EA (MT5) Starting ===");

   // Auto-generate AccountID from broker name and account number
   AccountID = GenerateAccountID();
   Print("Auto-generated AccountID: ", AccountID);

   // Parse symbol mapping
   ParseSymbolMappingString(SymbolMap, g_local_mappings);
   Print("Parsed ", ArraySize(g_local_mappings), " local symbol mappings");

   // Initialize ZMQ context
   g_zmq_context = InitializeZmqContext();
   if(g_zmq_context < 0)
      return INIT_FAILED;

   // Create and connect trade signal socket (SUB to port 5556)
   g_zmq_trade_socket = CreateAndConnectZmqSocket(g_zmq_context, ZMQ_SUB, TradeSignalSourceAddress, "Slave Trade SUB");
   if(g_zmq_trade_socket < 0)
   {
      zmq_context_destroy(g_zmq_context);
      return INIT_FAILED;
   }

   // Create and connect config socket (SUB to port 5557)
   g_zmq_config_socket = CreateAndConnectZmqSocket(g_zmq_context, ZMQ_SUB, ConfigSourceAddress, "Slave Config SUB");
   if(g_zmq_config_socket < 0)
   {
      CleanupZmqSocket(g_zmq_trade_socket, "Slave Trade SUB");
      CleanupZmqContext(g_zmq_context);
      return INIT_FAILED;
   }

   // Subscribe to config messages for this account ID
   if(!SubscribeToTopic(g_zmq_config_socket, AccountID))
   {
      CleanupZmqMultiSocket(g_zmq_trade_socket, g_zmq_config_socket, g_zmq_context, "Slave Trade SUB", "Slave Config SUB");
      return INIT_FAILED;
   }

   g_trade.SetExpertMagicNumber(0);
   g_trade.SetDeviationInPoints(Slippage);
   g_trade.SetTypeFilling(ORDER_FILLING_IOC);

   ArrayResize(g_order_map, 0);
   ArrayResize(g_pending_order_map, 0);

   // Initialize configuration arrays
   ArrayResize(g_configs, 0);

   g_initialized = true;

   // Set up timer for heartbeat and config messages (1 second interval)
   EventSetTimer(1);

   // Initialize configuration panel (Grid Panel)
   if(ShowConfigPanel)
   {
      g_config_panel.InitializeSlavePanel("SankeyCopierPanel_", PanelWidth);
      // Show NO_CONFIGURATION status initially (no config received yet)
      g_config_panel.UpdateStatusRow(STATUS_NO_CONFIGURATION);
      g_config_panel.UpdateServerRow(RelayServerAddress);
      g_config_panel.UpdateSymbolConfig(SymbolPrefix, SymbolSuffix, SymbolMap);
   }

   ChartRedraw();
   return INIT_SUCCEEDED;
}

//+------------------------------------------------------------------+
//| Expert deinitialization function                                  |
//+------------------------------------------------------------------+
void OnDeinit(const int reason)
{
   // Send unregister message to server
   SendUnregistrationMessage(g_zmq_context, RelayServerAddress, AccountID);

   // Kill timer
   EventKillTimer();

   // Delete configuration panel
   if(ShowConfigPanel)
      g_config_panel.Delete();

   // Cleanup ZMQ resources
   CleanupZmqMultiSocket(g_zmq_trade_socket, g_zmq_config_socket, g_zmq_context, "Slave Trade SUB", "Slave Config SUB");
}

//+------------------------------------------------------------------+
//| Timer function (called every 1 second)                            |
//+------------------------------------------------------------------+
void OnTimer()
{
   if(!g_initialized)
      return;

   // Check for auto-trading state change (IsTradeAllowed)
   bool current_trade_allowed = (bool)TerminalInfoInteger(TERMINAL_TRADE_ALLOWED);
   bool trade_state_changed = (current_trade_allowed != g_last_trade_allowed);

   // Send heartbeat every HEARTBEAT_INTERVAL_SECONDS OR on trade state change
   datetime now = TimeLocal();
   bool should_send_heartbeat = (now - g_last_heartbeat >= HEARTBEAT_INTERVAL_SECONDS) || trade_state_changed;

   if(should_send_heartbeat)
   {
      bool heartbeat_sent = SendHeartbeatMessage(g_zmq_context, RelayServerAddress, AccountID, "Slave", "MT5", SymbolPrefix, SymbolSuffix, SymbolMap);

      if(heartbeat_sent)
      {
         g_last_heartbeat = TimeLocal();

         // If trade state changed, log it and update panel
         if(trade_state_changed)
         {
            Print("[INFO] Auto-trading state changed: ", g_last_trade_allowed, " -> ", current_trade_allowed);
            g_last_trade_allowed = current_trade_allowed;

            // Update panel to reflect auto-trading state
            // Only update if we have received config, otherwise keep showing "Waiting"
            if(ShowConfigPanel && g_has_received_config)
            {
               if(!current_trade_allowed)
               {
                  // Auto-trading OFF -> show ENABLED (yellow) as warning, like Web UI
                  g_config_panel.UpdateStatusRow(STATUS_ENABLED);
               }
               else
               {
                  // Auto-trading ON -> show actual config status
                  // If we have at least one connected master, show CONNECTED
                  bool any_connected = false;
                  for(int i=0; i<ArraySize(g_configs); i++)
                  {
                     if(g_configs[i].status == STATUS_CONNECTED)
                     {
                        any_connected = true;
                        break;
                     }
                  }
                  
                  if(any_connected)
                     g_config_panel.UpdateStatusRow(STATUS_CONNECTED);
                  else if(ArraySize(g_configs) > 0)
                     g_config_panel.UpdateStatusRow(STATUS_ENABLED);
                  else
                     g_config_panel.UpdateStatusRow(STATUS_DISABLED);
               }
            }

            // If auto-trading was just enabled, request configuration
            if(current_trade_allowed && !g_config_requested)
            {
               Print("[INFO] Auto-trading enabled, requesting configuration...");
               if(SendRequestConfigMessage(g_zmq_context, RelayServerAddress, AccountID, "Slave"))
               {
                  g_config_requested = true;
                  Print("[INFO] Configuration request sent successfully");
               }
               else
               {
                  Print("[ERROR] Failed to send configuration request, will retry on next state change");
               }
            }
         }
         else
         {
            // On first successful heartbeat (normal interval), request configuration if not yet requested
            if(!g_config_requested)
            {
               Print("[INFO] First heartbeat successful, requesting configuration...");
               if(SendRequestConfigMessage(g_zmq_context, RelayServerAddress, AccountID, "Slave"))
               {
                  g_config_requested = true;
                  Print("[INFO] Configuration request sent successfully");
               }
               else
               {
                  Print("[ERROR] Failed to send configuration request, will retry on next heartbeat");
               }
            }
         }
      }
   }

   // Check for configuration messages (MessagePack format)
   uchar config_buffer[];
   ArrayResize(config_buffer, MESSAGE_BUFFER_SIZE);
   int config_bytes = zmq_socket_receive(g_zmq_config_socket, config_buffer, MESSAGE_BUFFER_SIZE);

   if(config_bytes > 0)
   {
      // Find the space separator between topic and MessagePack payload
      int space_pos = -1;
      for(int i = 0; i < config_bytes; i++)
      {
         if(config_buffer[i] == SPACE_CHAR)
         {
            space_pos = i;
            break;
         }
      }

      if(space_pos > 0)
      {
         // Extract topic
         string topic = CharArrayToString(config_buffer, 0, space_pos);

         // Extract MessagePack payload
         int payload_start = space_pos + 1;
         int payload_len = config_bytes - payload_start;
         uchar msgpack_payload[];
         ArrayResize(msgpack_payload, payload_len);
         ArrayCopy(msgpack_payload, config_buffer, 0, payload_start, payload_len);

         Print("Received MessagePack config for topic '", topic, "' (", payload_len, " bytes)");
         ProcessConfigMessage(msgpack_payload, payload_len, g_configs, g_zmq_trade_socket);

         g_has_received_config = true;
         
         if(ArraySize(g_configs) != g_last_config_count)
         {
            Print("DEBUG: Config count changed: ", g_last_config_count, " -> ", ArraySize(g_configs));
            g_last_config_count = ArraySize(g_configs);
         }

         // Update configuration panel
         if(ShowConfigPanel)
         {
            // Check local auto-trading state - if OFF, show ENABLED (yellow) as warning
            bool local_trade_allowed = (bool)TerminalInfoInteger(TERMINAL_TRADE_ALLOWED);
            if(!local_trade_allowed)
            {
               // Local auto-trading OFF -> show ENABLED (yellow) warning, like Web UI
               g_config_panel.UpdateStatusRow(STATUS_ENABLED);
            }
            else
            {
               // Local auto-trading ON -> show actual config status from server
               // If we have at least one connected master, show CONNECTED
               bool any_connected = false;
               for(int i=0; i<ArraySize(g_configs); i++)
               {
                  if(g_configs[i].status == STATUS_CONNECTED)
                  {
                     any_connected = true;
                     break;
                  }
               }
               
               if(any_connected)
                  g_config_panel.UpdateStatusRow(STATUS_CONNECTED);
               else if(ArraySize(g_configs) > 0)
                  g_config_panel.UpdateStatusRow(STATUS_ENABLED); // Has configs but none connected?
               else
                  g_config_panel.UpdateStatusRow(STATUS_DISABLED);
            }
            
            // Update dynamic config list
            g_config_panel.UpdateConfigList(g_configs);
         }
      }
   }
}

//+------------------------------------------------------------------+
//| Expert tick function                                              |
//+------------------------------------------------------------------+
void OnTick()
{
   if(!g_initialized) return;

   // Check for trade signal messages (MessagePack format)
   uchar trade_buffer[];
   ArrayResize(trade_buffer, MESSAGE_BUFFER_SIZE);
   int trade_bytes = zmq_socket_receive(g_zmq_trade_socket, trade_buffer, MESSAGE_BUFFER_SIZE);

   if(trade_bytes > 0)
   {
      // PUB/SUB format: topic(master_account) + space + MessagePack payload
      int space_pos = -1;
      for(int i = 0; i < trade_bytes; i++)
      {
         if(trade_buffer[i] == SPACE_CHAR)
         {
            space_pos = i;
            break;
         }
      }

      if(space_pos > 0)
      {
         // Extract topic
         string topic = CharArrayToString(trade_buffer, 0, space_pos);

         // Extract MessagePack payload
         int payload_start = space_pos + 1;
         int payload_len = trade_bytes - payload_start;
         uchar msgpack_payload[];
         ArrayResize(msgpack_payload, payload_len);
         ArrayCopy(msgpack_payload, trade_buffer, 0, payload_start, payload_len);

         Print("Received MessagePack trade signal for topic '", topic, "' (", payload_len, " bytes)");
         ProcessTradeSignal(msgpack_payload, payload_len);
      }
   }
}

//+------------------------------------------------------------------+
//| Process trade signal                                              |
//+------------------------------------------------------------------+
void ProcessTradeSignal(uchar &data[], int data_len)
{
   // Parse MessagePack trade signal
   HANDLE_TYPE handle = parse_trade_signal(data, data_len);
   if(handle == 0 || handle == -1)
   {
      Print("ERROR: Failed to parse MessagePack trade signal");
      return;
   }

   // Extract fields from MessagePack
   string action = trade_signal_get_string(handle, "action");
   long ticket_long = trade_signal_get_int(handle, "ticket");
   ulong master_ticket = (ulong)ticket_long;
   string symbol = trade_signal_get_string(handle, "symbol");
   string order_type_str = trade_signal_get_string(handle, "order_type");
   double lots = trade_signal_get_double(handle, "lots");
   double price = trade_signal_get_double(handle, "open_price");
   double sl = trade_signal_get_double(handle, "stop_loss");
   double tp = trade_signal_get_double(handle, "take_profit");
   long magic_long = trade_signal_get_int(handle, "magic_number");
   int magic_number = (int)magic_long;
   string timestamp = trade_signal_get_string(handle, "timestamp");
   string source_account = trade_signal_get_string(handle, "source_account");

   // Find matching config for this master
   int config_index = -1;
   for(int i=0; i<ArraySize(g_configs); i++)
   {
      if(g_configs[i].master_account == source_account)
      {
         config_index = i;
         break;
      }
   }

   if(config_index == -1)
   {
      Print("Trade signal rejected: No active configuration for master ", source_account);
      trade_signal_free(handle);
      return;
   }

   // Check if connected to Master before processing any trades
   if(g_configs[config_index].status != STATUS_CONNECTED)
   {
      Print("Trade signal rejected: Not connected to Master (status=", g_configs[config_index].status, "). Master ticket #", master_ticket);
      trade_signal_free(handle);
      return;
   }

   if(action == "Open" && AllowNewOrders)
   {
      // Apply filtering
      if(!ShouldProcessTrade(symbol, magic_number, g_configs[config_index]))
      {
         Print("Trade filtered out: ", symbol, " magic=", magic_number);
         trade_signal_free(handle);
         return;
      }

      // Apply transformations
      string mapped_symbol = TransformSymbol(symbol, g_configs[config_index].symbol_mappings);
      mapped_symbol = TransformSymbol(mapped_symbol, g_local_mappings); // Apply local mapping
      string transformed_symbol = GetLocalSymbol(mapped_symbol, SymbolPrefix, SymbolSuffix);
      
      // Transform lot size
      double transformed_lots = TransformLotSize(lots, g_configs[config_index].lot_multiplier, transformed_symbol);
      string transformed_order_type = ReverseOrderType(order_type_str, g_configs[config_index].reverse_trade);

      // Open position with transformed values
      OpenPosition(master_ticket, transformed_symbol, transformed_order_type, transformed_lots, price, sl, tp, timestamp, source_account, magic_number);
   }
   else if(action == "Close" && AllowCloseOrders)
   {
      ClosePosition(master_ticket);
      CancelPendingOrder(master_ticket);  // Also cancel any pending orders
   }
   else if(action == "Modify")
   {
      ModifyPosition(master_ticket, sl, tp);
   }

   // Free the handle
   trade_signal_free(handle);
}

//+------------------------------------------------------------------+
//| Open position                                                     |
//+------------------------------------------------------------------+
void OpenPosition(ulong master_ticket, string symbol, string type_str,
                  double lots, double price, double sl, double tp, string timestamp, string source_account, int magic)
{
   if(GetSlaveTicketFromMapping(g_order_map, master_ticket) > 0)
   {
      Print("Already copied master #", master_ticket);
      return;
   }

   // Check signal delay
   datetime signal_time = ParseISO8601(timestamp);
   datetime current_time = TimeGMT();
   int delay_ms = (int)((current_time - signal_time) * 1000);

   if(delay_ms > MaxSignalDelayMs)
   {
      if(!UsePendingOrderForDelayed)
      {
         Print("Signal too old (", delay_ms, "ms > ", MaxSignalDelayMs, "ms). Skipping master #", master_ticket);
         return;
      }
      else
      {
         Print("Signal delayed (", delay_ms, "ms). Using pending order at original price ", price);
         PlacePendingOrder(master_ticket, symbol, type_str, lots, price, sl, tp, source_account, delay_ms, magic);
         return;
      }
   }

   ENUM_ORDER_TYPE order_type = GetOrderTypeEnum(type_str);
   if((int)order_type == -1) return;

   lots = NormalizeDouble(lots, 2);
   price = NormalizeDouble(price, _Digits);
   sl = (sl > 0) ? NormalizeDouble(sl, _Digits) : 0;
   tp = (tp > 0) ? NormalizeDouble(tp, _Digits) : 0;

   // Extract account number and build traceable comment: "M12345#98765"
   string comment = "M" + IntegerToString(master_ticket) + "#" + ExtractAccountNumber(source_account);

   g_trade.SetExpertMagicNumber(magic);
   bool result = false;

   for(int i = 0; i < MaxRetries; i++)
   {
      if(order_type == ORDER_TYPE_BUY)
         result = g_trade.Buy(lots, symbol, 0, sl, tp, comment);
      else if(order_type == ORDER_TYPE_SELL)
         result = g_trade.Sell(lots, symbol, 0, sl, tp, comment);

      if(result)
      {
         ulong ticket = g_trade.ResultOrder();
         Print("Position opened: #", ticket, " from master #", master_ticket, " (delay: ", delay_ms, "ms)");
         AddTicketMapping(g_order_map, master_ticket, ticket);
         break;
      }
      else
      {
         Print("Failed to open position, attempt ", i+1, "/", MaxRetries);
         Sleep(1000);
      }
   }
}

//+------------------------------------------------------------------+
//| Close position                                                    |
//+------------------------------------------------------------------+
void ClosePosition(ulong master_ticket)
{
   ulong slave_ticket = GetSlaveTicketFromMapping(g_order_map, master_ticket);
   if(slave_ticket == 0)
   {
      Print("No slave position for master #", master_ticket);
      return;
   }

   if(!PositionSelectByTicket(slave_ticket))
   {
      Print("Position #", slave_ticket, " not found");
      RemoveTicketMapping(g_order_map, master_ticket);
      return;
   }

   if(g_trade.PositionClose(slave_ticket))
   {
      Print("Position closed: #", slave_ticket);
      RemoveTicketMapping(g_order_map, master_ticket);
   }
   else
   {
      Print("Failed to close position #", slave_ticket);
   }
}

//+------------------------------------------------------------------+
//| Modify position                                                   |
//+------------------------------------------------------------------+
void ModifyPosition(ulong master_ticket, double sl, double tp)
{
   ulong slave_ticket = GetSlaveTicketFromMapping(g_order_map, master_ticket);
   if(slave_ticket == 0) return;

   if(!PositionSelectByTicket(slave_ticket)) return;

   if(g_trade.PositionModify(slave_ticket, sl, tp))
   {
      Print("Position modified: #", slave_ticket);
   }
}

//+------------------------------------------------------------------+
//| Place pending order at original price                            |
//+------------------------------------------------------------------+
void PlacePendingOrder(ulong master_ticket, string symbol, string type_str,
                       double lots, double price, double sl, double tp, string source_account, int delay_ms, int magic)
{
   // Check if pending order already exists
   if(GetPendingTicketFromMapping(g_pending_order_map, master_ticket) > 0)
   {
      Print("Pending order already exists for master #", master_ticket);
      return;
   }

   ENUM_ORDER_TYPE order_type = GetOrderTypeEnum(type_str);
   if((int)order_type == -1) return;

   // Convert to pending order type
   ENUM_ORDER_TYPE pending_type;
   double current_price;

   if(order_type == ORDER_TYPE_BUY)
   {
      current_price = SymbolInfoDouble(symbol, SYMBOL_ASK);
      pending_type = (price < current_price) ? ORDER_TYPE_BUY_LIMIT : ORDER_TYPE_BUY_STOP;
   }
   else
   {
      current_price = SymbolInfoDouble(symbol, SYMBOL_BID);
      pending_type = (price > current_price) ? ORDER_TYPE_SELL_LIMIT : ORDER_TYPE_SELL_STOP;
   }

   lots = NormalizeDouble(lots, 2);

   // Extract account number and build traceable comment: "P12345#98765"
   string comment = "P" + IntegerToString(master_ticket) + "#" + ExtractAccountNumber(source_account);

   g_trade.SetExpertMagicNumber(magic);

   bool result = g_trade.OrderOpen(symbol, pending_type, lots, 0, price, sl, tp,
                                    ORDER_TIME_GTC, 0, comment);

   if(result)
   {
      ulong ticket = g_trade.ResultOrder();
      Print("Pending order placed: #", ticket, " for master #", master_ticket, " at price ", price);
      AddPendingTicketMapping(g_pending_order_map, master_ticket, ticket);
   }
   else
   {
      Print("Failed to place pending order for master #", master_ticket);
   }
}

//+------------------------------------------------------------------+
//| Cancel pending order                                              |
//+------------------------------------------------------------------+
void CancelPendingOrder(ulong master_ticket)
{
   ulong pending_ticket = GetPendingTicketFromMapping(g_pending_order_map, master_ticket);
   if(pending_ticket == 0)
      return;

   if(g_trade.OrderDelete(pending_ticket))
   {
      Print("Pending order cancelled: #", pending_ticket, " for master #", master_ticket);
      RemovePendingTicketMapping(g_pending_order_map, master_ticket);
   }
   else
   {
      Print("Failed to cancel pending order #", pending_ticket);
   }
}

// Ticket mapping functions are now provided by SankeyCopierMapping.mqh

