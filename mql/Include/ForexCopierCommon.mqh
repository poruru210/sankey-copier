//+------------------------------------------------------------------+
//|                                        ForexCopierCommon.mqh    |
//|                        Copyright 2025, Forex Copier Project      |
//|                     Common definitions and utilities              |
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, Forex Copier Project"

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

//--- Import Rust ZeroMQ DLL
#import "forex_copier_zmq.dll"
   int    zmq_context_create();
   void   zmq_context_destroy(int context);
   int    zmq_socket_create(int context, int socket_type);
   void   zmq_socket_destroy(int socket);
   int    zmq_socket_bind(int socket, string address);
   int    zmq_socket_connect(int socket, string address);
   int    zmq_socket_send(int socket, string message);
   int    zmq_socket_send_binary(int socket, uchar &data[], int len);
   int    zmq_socket_receive(int socket, uchar &buffer[], int buffer_size);
   int    zmq_socket_subscribe_all(int socket);
   int    zmq_socket_subscribe(int socket, string topic);

   // MessagePack serialization functions
   int    serialize_register(string message_type, string account_id, string ea_type,
                             string platform, long account_number, string broker,
                             string account_name, string server, double balance,
                             double equity, string currency, long leverage, string timestamp);
   int    serialize_unregister(string message_type, string account_id, string timestamp);
   int    serialize_heartbeat(string message_type, string account_id, double balance,
                              double equity, int open_positions, string timestamp);
   int    serialize_trade_signal(string action, long ticket, string symbol, string order_type,
                                 double lots, double open_price, double stop_loss, double take_profit,
                                 long magic_number, string comment, string timestamp, string source_account);
   uchar* get_serialized_buffer();
   int    copy_serialized_buffer(uchar &dest[], int max_len);

   // Config message parsing (existing)
   #ifdef IS_MT5
      long   parse_message(uchar &data[], int data_len);
      string config_get_string(long handle, string field_name);
      double config_get_double(long handle, string field_name);
      int    config_get_bool(long handle, string field_name);
      int    config_get_int(long handle, string field_name);
      void   config_free(long handle);

      // Trade signal parsing
      long   parse_trade_signal(uchar &data[], int data_len);
      string trade_signal_get_string(long handle, string field_name);
      double trade_signal_get_double(long handle, string field_name);
      long   trade_signal_get_int(long handle, string field_name);
      void   trade_signal_free(long handle);
   #else
      int    parse_message(uchar &data[], int data_len);
      string config_get_string(int handle, string field_name);
      double config_get_double(int handle, string field_name);
      int    config_get_bool(int handle, string field_name);
      int    config_get_int(int handle, string field_name);
      void   config_free(int handle);

      // Trade signal parsing
      int    parse_trade_signal(uchar &data[], int data_len);
      string trade_signal_get_string(int handle, string field_name);
      double trade_signal_get_double(int handle, string field_name);
      long   trade_signal_get_int(int handle, string field_name);
      void   trade_signal_free(int handle);
   #endif
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
