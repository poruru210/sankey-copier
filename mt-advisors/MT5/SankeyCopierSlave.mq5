//+------------------------------------------------------------------+
//|                                        SankeyCopierSlave.mq5      |
//|                        Copyright 2025, SANKEY Copier Project      |
//|                                                                  |
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, SANKEY Copier Project"
#property link      ""
#property version   "1.00"  // VERSION_PLACEHOLDER
#property icon      "app.ico"

#include <Trade/Trade.mqh>

//--- Include common headers
#include <SankeyCopier/Common.mqh>
#include <SankeyCopier/Zmq.mqh>
#include <SankeyCopier/Mapping.mqh>
#include <SankeyCopier/GridPanel.mqh>
#include <SankeyCopier/Messages.mqh>
#include <SankeyCopier/Trade.mqh>

//--- Input parameters
input string   TradeServerAddress = "tcp://localhost:5556";  // Trade signal channel
input string   ConfigServerAddress = "tcp://localhost:5557"; // Configuration channel
input int      Slippage = 3;
input int      MaxRetries = 3;
input bool     AllowNewOrders = true;
input bool     AllowCloseOrders = true;
input int      MaxSignalDelayMs = 5000;                      // Maximum allowed signal delay (milliseconds)
input bool     UsePendingOrderForDelayed = false;            // Use pending order for delayed signals
input bool     ShowConfigPanel = true;                       // Show configuration panel on chart
input int      PanelWidth = 280;                             // Configuration panel width (pixels)

//--- Global variables
string      AccountID;                  // Auto-generated from broker + account number
HANDLE_TYPE g_zmq_context = -1;
HANDLE_TYPE g_zmq_trade_socket = -1;    // Socket for receiving trade signals
HANDLE_TYPE g_zmq_config_socket = -1;   // Socket for receiving configuration
CTrade      g_trade;
bool        g_initialized = false;
datetime    g_last_heartbeat = 0;
string      g_current_master = "";      // Currently configured master account
string      g_trade_group_id = "";      // Current trade group subscription
bool        g_config_requested = false; // Track if config has been requested

// Ticket mapping arrays (structures defined in SankeyCopierMapping.mqh)
TicketMapping g_order_map[];
PendingTicketMapping g_pending_order_map[];

//--- Extended configuration variables (from ConfigMessage)
bool           g_config_enabled = true;          // Whether copying is enabled
double         g_config_lot_multiplier = 1.0;    // Lot multiplier (default 1.0)
bool           g_config_reverse_trade = false;   // Reverse trades (Buy<->Sell)
SymbolMapping  g_symbol_mappings[];              // Symbol mappings

//--- Configuration panel
CGridPanel     g_config_panel;                   // Grid panel for displaying configuration
TradeFilters   g_filters;                        // Trade filters
int            g_config_version = 0;             // Configuration version

//+------------------------------------------------------------------+
//| Expert initialization function                                     |
//+------------------------------------------------------------------+
int OnInit()
{
   Print("=== SankeyCopier Slave EA (MT5) Starting ===");

   // Auto-generate AccountID from broker name and account number
   AccountID = GenerateAccountID();
   Print("Auto-generated AccountID: ", AccountID);

   // Initialize ZMQ context
   g_zmq_context = InitializeZmqContext();
   if(g_zmq_context < 0)
      return INIT_FAILED;

   // Create and connect trade signal socket (SUB to port 5556)
   g_zmq_trade_socket = CreateAndConnectZmqSocket(g_zmq_context, ZMQ_SUB, TradeServerAddress, "Slave Trade SUB");
   if(g_zmq_trade_socket < 0)
   {
      CleanupZmqContext(g_zmq_context);
      return INIT_FAILED;
   }

   // Create and connect config socket (SUB to port 5557)
   g_zmq_config_socket = CreateAndConnectZmqSocket(g_zmq_context, ZMQ_SUB, ConfigServerAddress, "Slave Config SUB");
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
   ArrayResize(g_symbol_mappings, 0);
   ArrayResize(g_filters.allowed_symbols, 0);
   ArrayResize(g_filters.blocked_symbols, 0);
   ArrayResize(g_filters.allowed_magic_numbers, 0);
   ArrayResize(g_filters.blocked_magic_numbers, 0);

   g_initialized = true;

   // Set up timer for heartbeat and config messages (1 second interval)
   EventSetTimer(1);

   // Initialize configuration panel (Grid Panel)
   if(ShowConfigPanel)
   {
      g_config_panel.InitializeSlavePanel("SankeyCopierPanel_", PanelWidth);

      // Update panel immediately with current values to avoid showing "N/A"
      // (OnTimer will also update these every second, but this provides instant feedback)
      g_config_panel.UpdateStatusRow(g_config_enabled);
      g_config_panel.UpdateMasterRow(g_current_master == "" ? "N/A" : g_current_master);
      g_config_panel.UpdateLotMultiplierRow(g_config_lot_multiplier);
      g_config_panel.UpdateReverseRow(g_config_reverse_trade);
      g_config_panel.UpdateVersionRow(g_config_version);
      g_config_panel.UpdateSymbolCountRow(ArraySize(g_symbol_mappings));
   }

   return INIT_SUCCEEDED;
}

//+------------------------------------------------------------------+
//| Expert deinitialization function                                  |
//+------------------------------------------------------------------+
void OnDeinit(const int reason)
{
   // Send unregister message to server
   SendUnregistrationMessage(g_zmq_context, "tcp://localhost:5555", AccountID);

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

   // Send heartbeat every HEARTBEAT_INTERVAL_SECONDS
   datetime now = TimeLocal();

   if(now - g_last_heartbeat >= HEARTBEAT_INTERVAL_SECONDS)
   {
      bool heartbeat_sent = SendHeartbeatMessage(g_zmq_context, "tcp://localhost:5555", AccountID, "Slave", "MT5");

      if(heartbeat_sent)
      {
         g_last_heartbeat = TimeLocal();

         // On first successful heartbeat, request configuration from server
         if(!g_config_requested)
         {
            Print("[INFO] First heartbeat successful, requesting configuration...");
            if(SendRequestConfigMessage(g_zmq_context, "tcp://localhost:5555", AccountID))
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
         ProcessConfigMessage(msgpack_payload, payload_len, g_current_master, g_trade_group_id,
                             g_config_enabled, g_config_lot_multiplier, g_config_reverse_trade,
                             g_config_version, g_symbol_mappings, g_filters, g_zmq_trade_socket);

         // Update configuration panel
         if(ShowConfigPanel)
         {
            g_config_panel.UpdateStatusRow(g_config_enabled);
            g_config_panel.UpdateMasterRow(g_current_master == "" ? "N/A" : g_current_master);
            g_config_panel.UpdateLotMultiplierRow(g_config_lot_multiplier);
            g_config_panel.UpdateReverseRow(g_config_reverse_trade);
            g_config_panel.UpdateVersionRow(g_config_version);
            g_config_panel.UpdateSymbolCountRow(ArraySize(g_symbol_mappings));
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
      // PUB/SUB format: topic(trade_group_id) + space + MessagePack payload
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

   if(action == "Open" && AllowNewOrders)
   {
      // Apply filtering
      if(!ShouldProcessTrade(symbol, magic_number, g_config_enabled, g_filters))
      {
         Print("Trade filtered out: ", symbol, " magic=", magic_number);
         trade_signal_free(handle);
         return;
      }

      // Apply transformations
      string transformed_symbol = TransformSymbol(symbol, g_symbol_mappings);
      double transformed_lots = TransformLotSize(lots, g_config_lot_multiplier);
      string transformed_order_type = ReverseOrderType(order_type_str, g_config_reverse_trade);

      // Open position with transformed values
      OpenPosition(master_ticket, transformed_symbol, transformed_order_type, transformed_lots, price, sl, tp, timestamp, source_account);
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
                  double lots, double price, double sl, double tp, string timestamp, string source_account)
{
   if(GetSlaveTicketFromMapping(g_order_map, master_ticket) > 0)
   {
      Print("Already copied master #", master_ticket);
      return;
   }

   // Check signal delay
   datetime signal_time = ParseISO8601(timestamp);
   datetime current_time = TimeCurrent();
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
         PlacePendingOrder(master_ticket, symbol, type_str, lots, price, sl, tp, source_account, delay_ms);
         return;
      }
   }

   ENUM_ORDER_TYPE order_type = GetOrderTypeEnum(type_str);
   if((int)order_type == -1) return;

   lots = NormalizeDouble(lots, 2);

   // Extract account number and build traceable comment: "M12345#98765"
   string comment = "M" + IntegerToString(master_ticket) + "#" + ExtractAccountNumber(source_account);

   g_trade.SetExpertMagicNumber(0);
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
                       double lots, double price, double sl, double tp, string source_account, int delay_ms)
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

   g_trade.SetExpertMagicNumber(0);

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

