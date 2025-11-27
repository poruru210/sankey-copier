//+------------------------------------------------------------------+
//|                                        SankeyCopierCommon.mqh    |
//|                        Copyright 2025, SANKEY Copier Project      |
//|                     Common definitions and utilities              |
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, SANKEY Copier Project"

#ifndef SANKEY_COPIER_COMMON_MQH
#define SANKEY_COPIER_COMMON_MQH

//--- Platform detection and type aliases
#ifdef __MQL5__
   #define IS_MT5
   #define TICKET_TYPE ulong
   #define HANDLE_TYPE long
#else
   #define IS_MT4
   #define TICKET_TYPE int
   #define HANDLE_TYPE int
#endif

//--- ZeroMQ socket types
#define ZMQ_PULL 7
#define ZMQ_PUSH 8
#define ZMQ_SUB  2

//--- Common constants
#define HEARTBEAT_INTERVAL_SECONDS 30
#define MESSAGE_BUFFER_SIZE 4096
#define SPACE_CHAR 32

//--- Port Definitions
#define PORT_RELAY_PULL       5555  // Heartbeat, ConfigReq, Unregister (Client -> Server)
#define PORT_RELAY_PUB_TRADE  5556  // Trade Signals (Server -> Client)
#define PORT_RELAY_PUB_CONFIG 5557  // Config Updates (Server -> Client)

//--- Default Addresses
#define DEFAULT_ADDR_PULL       "tcp://localhost:5555"
#define DEFAULT_ADDR_PUB_TRADE  "tcp://localhost:5556"
#define DEFAULT_ADDR_PUB_CONFIG "tcp://localhost:5557"

//--- Connection status constants (4 states)
#define STATUS_DISABLED 0         // Slave is disabled
#define STATUS_ENABLED 1          // Slave is enabled, Master disconnected
#define STATUS_CONNECTED 2        // Slave is enabled, Master connected
#define STATUS_NO_CONFIGURATION 3 // No configuration received yet

//--- Import Rust ZeroMQ DLL
#import "sankey_copier_zmq.dll"
   HANDLE_TYPE zmq_context_create();
   void        zmq_context_destroy(HANDLE_TYPE context);
   HANDLE_TYPE zmq_socket_create(HANDLE_TYPE context, int socket_type);
   void        zmq_socket_destroy(HANDLE_TYPE socket);
   int         zmq_socket_bind(HANDLE_TYPE socket, string address);
   int         zmq_socket_connect(HANDLE_TYPE socket, string address);
   int         zmq_socket_send(HANDLE_TYPE socket, string message);
   int         zmq_socket_send_binary(HANDLE_TYPE socket, uchar &data[], int len);
   int         zmq_socket_receive(HANDLE_TYPE socket, uchar &buffer[], int buffer_size);
   int         zmq_socket_subscribe_all(HANDLE_TYPE socket);
   int         zmq_socket_subscribe(HANDLE_TYPE socket, string topic);

   // MessagePack serialization functions
   int    serialize_request_config(string message_type, string account_id, string timestamp, string ea_type);

   int    serialize_unregister(string message_type, string account_id, string timestamp);
   int serialize_heartbeat(string message_type, string account_id, double balance, double equity,
                          int open_positions, string timestamp, string ea_type, string platform,
                          long account_number, string broker, string account_name, string server,
                          string currency, long leverage, int is_trade_allowed,
                          string symbol_prefix, string symbol_suffix, string symbol_map);
   int    serialize_trade_signal(string action, long ticket, string symbol, string order_type,
                                 double lots, double open_price, double stop_loss, double take_profit,
                                 long magic_number, string comment, string timestamp, string source_account,
                                 double close_ratio);
   // Note: get_serialized_buffer() uses pointer syntax not supported in MQL4/MQL5
   // Use copy_serialized_buffer() instead
   int    copy_serialized_buffer(uchar &dest[], int max_len);

   // Slave config message parsing
   HANDLE_TYPE parse_slave_config(uchar &data[], int data_len);
   string      slave_config_get_string(HANDLE_TYPE handle, string field_name);
   double      slave_config_get_double(HANDLE_TYPE handle, string field_name);
   int         slave_config_get_bool(HANDLE_TYPE handle, string field_name);
   int         slave_config_get_int(HANDLE_TYPE handle, string field_name);
   void        slave_config_free(HANDLE_TYPE handle);

   // Slave config symbol mappings array access
   int         slave_config_get_symbol_mappings_count(HANDLE_TYPE handle);
   string      slave_config_get_symbol_mapping_source(HANDLE_TYPE handle, int index);
   string      slave_config_get_symbol_mapping_target(HANDLE_TYPE handle, int index);

   // Slave config allowed magic numbers filter access
   int         slave_config_get_allowed_magic_count(HANDLE_TYPE handle);
   int         slave_config_get_allowed_magic_at(HANDLE_TYPE handle, int index);

   // Master config message parsing
   HANDLE_TYPE parse_master_config(uchar &data[], int data_len);
   string      master_config_get_string(HANDLE_TYPE handle, string field_name);
   int         master_config_get_int(HANDLE_TYPE handle, string field_name);
   void        master_config_free(HANDLE_TYPE handle);

   // Trade signal parsing
   HANDLE_TYPE parse_trade_signal(uchar &data[], int data_len);
   string      trade_signal_get_string(HANDLE_TYPE handle, string field_name);
   double      trade_signal_get_double(HANDLE_TYPE handle, string field_name);
   long        trade_signal_get_int(HANDLE_TYPE handle, string field_name);
   void        trade_signal_free(HANDLE_TYPE handle);

   // Position snapshot parsing (Slave receives from Master)
   HANDLE_TYPE parse_position_snapshot(uchar &data[], int data_len);
   string      position_snapshot_get_string(HANDLE_TYPE handle, string field_name);
   int         position_snapshot_get_positions_count(HANDLE_TYPE handle);
   string      position_snapshot_get_position_string(HANDLE_TYPE handle, int index, string field_name);
   double      position_snapshot_get_position_double(HANDLE_TYPE handle, int index, string field_name);
   long        position_snapshot_get_position_int(HANDLE_TYPE handle, int index, string field_name);
   void        position_snapshot_free(HANDLE_TYPE handle);

   // SyncRequest creation (Slave sends to Master)
   int         create_sync_request(string slave_account, string master_account, uchar &output[], int output_len);

   // SyncRequest parsing (Master receives from Slave)
   HANDLE_TYPE parse_sync_request(uchar &data[], int data_len);
   string      sync_request_get_string(HANDLE_TYPE handle, string field_name);
   void        sync_request_free(HANDLE_TYPE handle);

   // Position snapshot builder (Master sends to Slave)
   HANDLE_TYPE create_position_snapshot_builder(string source_account);
   int         position_snapshot_builder_add_position(HANDLE_TYPE handle, long ticket, string symbol, string order_type,
                                                       double lots, double open_price, double stop_loss, double take_profit,
                                                       long magic_number, string open_time);
   int         position_snapshot_builder_serialize(HANDLE_TYPE handle, uchar &output[], int output_len);
   void        position_snapshot_builder_free(HANDLE_TYPE handle);
#import

//--- Common structures
struct SymbolMapping {
    string source_symbol;
    string target_symbol;
};

struct TradeFilters {
    string allowed_symbols[];
    string blocked_symbols[];
    int    allowed_magic_numbers[];
    int    blocked_magic_numbers[];
};

// Note: Slave-specific types (CopyConfig, LOT_CALC_MODE_*) moved to SlaveTypes.mqh

//+------------------------------------------------------------------+
//| Generate AccountID from broker and account number                |
//+------------------------------------------------------------------+
string GenerateAccountID()
{
   #ifdef IS_MT5
      string broker = AccountInfoString(ACCOUNT_COMPANY);
      long account_number = AccountInfoInteger(ACCOUNT_LOGIN);
   #else
      string broker = AccountCompany();
      int account_number = AccountNumber();
   #endif

   // Replace spaces and special characters with underscores
   StringReplace(broker, " ", "_");
   StringReplace(broker, ".", "_");
   StringReplace(broker, "-", "_");

   // Format: broker_accountnumber
   return broker + "_" + IntegerToString(account_number);
}

//+------------------------------------------------------------------+
//| Format timestamp to ISO 8601 format                              |
//+------------------------------------------------------------------+
string FormatTimestampISO8601(datetime time)
{
   string timestamp = TimeToString(time, TIME_DATE | TIME_SECONDS);
   StringReplace(timestamp, ".", "-");
   StringReplace(timestamp, " ", "T");
   timestamp += "Z";
   return timestamp;
}

//+------------------------------------------------------------------+
//| Parse ISO 8601 timestamp to datetime                            |
//| Format: "2025-01-15T10:30:45Z"                                  |
//| Returns: datetime value, or 0 if parsing fails or invalid format|
//+------------------------------------------------------------------+
datetime ParseISO8601(string timestamp)
{
   // Remove 'Z' suffix if present
   string ts = timestamp;
   StringReplace(ts, "Z", "");

   // Replace 'T' with space for parsing
   StringReplace(ts, "T", " ");

   // Parse components: "2025-01-15 10:30:45"
   if(StringLen(ts) < 19) return 0;

   int year = (int)StringToInteger(StringSubstr(ts, 0, 4));
   int month = (int)StringToInteger(StringSubstr(ts, 5, 2));
   int day = (int)StringToInteger(StringSubstr(ts, 8, 2));
   int hour = (int)StringToInteger(StringSubstr(ts, 11, 2));
   int minute = (int)StringToInteger(StringSubstr(ts, 14, 2));
   int second = (int)StringToInteger(StringSubstr(ts, 17, 2));

   // Construct datetime
   MqlDateTime dt;
   dt.year = year;
   dt.mon = month;
   dt.day = day;
   dt.hour = hour;
   dt.min = minute;
   dt.sec = second;

   return StructToTime(dt);
}

//+------------------------------------------------------------------+
//| Extract account number from source_account string                |
//| Example: "IC_Markets_98765" -> "98765"                          |
//| Returns: Account number or original string if no underscore found|
//+------------------------------------------------------------------+
string ExtractAccountNumber(string source_account)
{
   // Find the last underscore in the account string
   // Assume account number is after the last underscore and up to 15 chars from end
   int last_underscore = StringFind(source_account, "_", StringLen(source_account) - 15);

   if(last_underscore > 0)
      return StringSubstr(source_account, last_underscore + 1);

   // If no underscore found, return the original string
   return source_account;
}

//+------------------------------------------------------------------+
//| Get current positions count                                      |
//+------------------------------------------------------------------+
int GetOpenPositionsCount()
{
   #ifdef IS_MT5
      return PositionsTotal();
   #else
      int count = 0;
      for(int i = 0; i < OrdersTotal(); i++)
      {
         if(OrderSelect(i, SELECT_BY_POS, MODE_TRADES))
            count++;
      }
      return count;
   #endif
}

//+------------------------------------------------------------------+
//| Get account balance                                              |
//+------------------------------------------------------------------+
double GetAccountBalance()
{
   #ifdef IS_MT5
      return AccountInfoDouble(ACCOUNT_BALANCE);
   #else
      return AccountBalance();
   #endif
}

//+------------------------------------------------------------------+
//| Get account equity                                               |
//+------------------------------------------------------------------+
double GetAccountEquity()
{
   #ifdef IS_MT5
      return AccountInfoDouble(ACCOUNT_EQUITY);
   #else
      return AccountEquity();
   #endif
}

//+------------------------------------------------------------------+
//| Get account currency                                             |
//+------------------------------------------------------------------+
string GetAccountCurrency()
{
   #ifdef IS_MT5
      return AccountInfoString(ACCOUNT_CURRENCY);
   #else
      return AccountCurrency();
   #endif
}

//+------------------------------------------------------------------+
//| Get account leverage                                             |
//+------------------------------------------------------------------+
long GetAccountLeverage()
{
   #ifdef IS_MT5
      return AccountInfoInteger(ACCOUNT_LEVERAGE);
   #else
      return AccountLeverage();
   #endif
}

//+------------------------------------------------------------------+
//| Get account login number                                         |
//+------------------------------------------------------------------+
long GetAccountNumber()
{
   #ifdef IS_MT5
      return AccountInfoInteger(ACCOUNT_LOGIN);
   #else
      return AccountNumber();
   #endif
}

//+------------------------------------------------------------------+
//| Get broker name                                                  |
//+------------------------------------------------------------------+
string GetBrokerName()
{
   #ifdef IS_MT5
      return AccountInfoString(ACCOUNT_COMPANY);
   #else
      return AccountCompany();
   #endif
}

//+------------------------------------------------------------------+
//| Get account name                                                 |
//+------------------------------------------------------------------+
string GetAccountName()
{
   #ifdef IS_MT5
      return AccountInfoString(ACCOUNT_NAME);
   #else
      return AccountName();
   #endif
}

//+------------------------------------------------------------------+
//| Get server name                                                  |
//+------------------------------------------------------------------+
string GetServerName()
{
   #ifdef IS_MT5
      return AccountInfoString(ACCOUNT_SERVER);
   #else
      return AccountServer();
   #endif
}

//+------------------------------------------------------------------+
//| Get string representation of order type                          |
//| Returns PascalCase format matching relay-server's OrderType enum |
//+------------------------------------------------------------------+
string GetOrderTypeString(int type)
{
   #ifdef IS_MT5
      if(type == ORDER_TYPE_BUY) return "Buy";
      if(type == ORDER_TYPE_SELL) return "Sell";
      if(type == ORDER_TYPE_BUY_LIMIT) return "BuyLimit";
      if(type == ORDER_TYPE_SELL_LIMIT) return "SellLimit";
      if(type == ORDER_TYPE_BUY_STOP) return "BuyStop";
      if(type == ORDER_TYPE_SELL_STOP) return "SellStop";
   #else
      if(type == OP_BUY) return "Buy";
      if(type == OP_SELL) return "Sell";
      if(type == OP_BUYLIMIT) return "BuyLimit";
      if(type == OP_SELLLIMIT) return "SellLimit";
      if(type == OP_BUYSTOP) return "BuyStop";
      if(type == OP_SELLSTOP) return "SellStop";
   #endif
   return "Unknown";
}

//+------------------------------------------------------------------+
//| Get enum order type from string                                  |
//| Accepts PascalCase format from relay-server's OrderType enum     |
//+------------------------------------------------------------------+
ENUM_ORDER_TYPE GetOrderTypeEnum(string type_str)
{
   if(type_str == "Buy") return ORDER_TYPE_BUY;
   if(type_str == "Sell") return ORDER_TYPE_SELL;
   if(type_str == "BuyLimit") return ORDER_TYPE_BUY_LIMIT;
   if(type_str == "SellLimit") return ORDER_TYPE_SELL_LIMIT;
   if(type_str == "BuyStop") return ORDER_TYPE_BUY_STOP;
   if(type_str == "SellStop") return ORDER_TYPE_SELL_STOP;

   return (ENUM_ORDER_TYPE)-1;
}

#endif // SANKEY_COPIER_COMMON_MQH
