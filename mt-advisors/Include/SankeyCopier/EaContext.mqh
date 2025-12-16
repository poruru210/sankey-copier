//+------------------------------------------------------------------+
//|                                      SankeyCopierEaContext.mqh   |
//|                        Copyright 2025, SANKEY Copier Project     |
//|                     EaContextWrapper and common definitions      |
//+------------------------------------------------------------------+
// Renamed from Common.mqh for clarity - contains EaContextWrapper base class
#property copyright "Copyright 2025, SANKEY Copier Project"

#ifndef SANKEY_COPIER_EA_CONTEXT_MQH
#define SANKEY_COPIER_EA_CONTEXT_MQH

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

//--- Include configuration file reader (dynamic port loading)
#include "ConfigFile.mqh"

// Port values are now loaded dynamically from sankey_copier.ini
// 2-port architecture: Receiver (PULL) and Publisher (unified PUB)
// Use the following functions from ConfigFile.mqh:
//   GetReceiverPort()     - Get PUSH socket port (EA -> Server)
//   GetPublisherPort()    - Get SUB socket port (Server -> EA, unified for trades and configs)
//   GetPushAddress()      - Get full "tcp://localhost:port" address for PUSH socket
//   GetTradeSubAddress()  - Get full address for subscription (trades and configs)
//   GetConfigSubAddress() - Alias for GetTradeSubAddress() (same unified PUB socket)

//--- Connection status constants (4 states)
#define STATUS_DISABLED 0         // Slave is disabled
#define STATUS_ENABLED 1          // Slave is enabled, Master disconnected
#define STATUS_CONNECTED 2        // Slave is enabled, Master connected
#define STATUS_NO_CONFIG -1 // No configuration received yet

//--- EA Type Constants
#define EA_TYPE_MASTER "Master"
#define EA_TYPE_SLAVE  "Slave"

//--- Command types matching Rust
#define CMD_NONE 0
#define CMD_OPEN 1
#define CMD_CLOSE 2
#define CMD_MODIFY 3
#define CMD_DELETE 4
#define CMD_UPDATE_UI 5
#define CMD_SEND_SNAPSHOT 6
#define CMD_PROCESS_SNAPSHOT 7

//--- EaCommand structure with MQL4/pack=1 compatibility padding
struct EaCommand {
   int command_type;
   int algo_flags;   // Bit 0: IsDelayed (Replaced _pad1)

   long ticket;
   uchar symbol[32]; // Fixed size string buffer

   int order_type;
   int _pad2;        // Rust alignment matches MQL4 pack(1) manually

   double volume;
   double price;
   double sl;
   double tp;
   long magic;
   double close_ratio;
   long timestamp;
   uchar comment[64];
   uchar source_account[64];
};

//--- C-Compatible Structs for FFI (separated into FFITypes.mqh)
#include "FFITypes.mqh"

//--- DLL Imports (separated into FFIImports.mqh)
#include "FFIImports.mqh"


//+------------------------------------------------------------------+
//| EaContextWrapper: Manages Rust-side EaContext lifetime           |
//+------------------------------------------------------------------+
class EaContextWrapper
{
private:
   HANDLE_TYPE m_context;
   bool        m_initialized;

public:
   EaContextWrapper() : m_context(0), m_initialized(false) {}

   ~EaContextWrapper()
   {
      if(m_initialized && m_context != 0)
      {
         ea_context_free(m_context);
         m_context = 0;
         m_initialized = false;
      }
   }

   bool Initialize(string account_id, string ea_type, string platform, long account_number, 
                   string broker, string account_name, string server, string currency, long leverage)
   {
      if(m_initialized) return true;
      
      m_context = ea_init(account_id, ea_type, platform, account_number, broker, account_name, server, currency, leverage);
      
      if(m_context != 0)
      {
         m_initialized = true;
         return true;
      }
      return false;
   }

   bool Connect(string push_addr, string sub_addr)
   {
      if(!m_initialized) return false;
      return ea_connect(m_context, push_addr, sub_addr) == 1;
   }

   int ManagerTick(double balance, double equity, int open_positions, bool is_trade_allowed)
   {
      if(!m_initialized) return 0;
      return ea_manager_tick(m_context, balance, equity, open_positions, (int)is_trade_allowed);
   }

   bool GetCommand(EaCommand &command)
   {
      if(!m_initialized) return false;
      return ea_get_command(m_context, command) == 1;
   }
   
   bool SubscribeConfig(string topic)
   {
      if(!m_initialized) return false;
      return ea_subscribe_config(m_context, topic) == 1;
   }

   bool IsInitialized() const { return m_initialized; }
   HANDLE_TYPE GetHandle() const { return m_context; }
   
   // Send raw data via context (abstracted PUSH)
   bool SendPush(uchar &data[], int len)
   {
      if(!m_initialized) return false;
      return ea_send_push(m_context, data, len) == 1;
   }

   bool SendRegister(string detected_symbols)
   {
      if(!m_initialized) return false;
      
      uchar buffer[1024];
      int len = ea_send_register(m_context, buffer, 1024, detected_symbols);
      
      if(len > 0)
      {
         return ea_send_push(m_context, buffer, len) == 1;
      }
      return false;
   }

   bool SendHeartbeat(double balance, double equity, int open_positions, bool is_trade_allowed)
   {
      if(!m_initialized) return false;
      
      uchar buffer[1024];
      int len = ea_send_heartbeat(m_context, balance, equity, open_positions, (int)is_trade_allowed, buffer, 1024);
      
      if(len > 0)
      {
         return ea_send_push(m_context, buffer, len) == 1;
      }
      return false;
   }

   bool SendUnregister()
   {
      if(!m_initialized) return false;
      
      uchar buffer[1024];
      int len = ea_send_unregister(m_context, buffer, 1024);
      
      if(len > 0)
      {
         return ea_send_push(m_context, buffer, len) == 1;
      }
      return false;
   }
   
   bool ShouldRequestConfig(bool current_trade_allowed)
   {
      if(!m_initialized) return false;
      return ea_context_should_request_config(m_context, (int)current_trade_allowed) != 0;
   }
   
   void MarkConfigRequested()
   {
      if(m_initialized) ea_context_mark_config_requested(m_context);
   }
   
   void Reset()
   {
       if(m_initialized) ea_context_reset(m_context);
   }

   bool GetGlobalConfig(SGlobalConfig &config)
   {
      if(!m_initialized) return false;
      return ea_context_get_global_config(m_context, config) == 1;
   }

   // NOTE: Master/Slave specific methods moved to:
   // - MasterContext.mqh: GetMasterConfig, GetSyncRequest, SendOpenSignal, SendCloseSignal, SendModifySignal, SendPositionSnapshot
   // - SlaveContext.mqh: GetSlaveConfig, GetPositionSnapshot, GetSymbolMappings, SendSyncRequest, SendRequestConfig
};

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

#endif // SANKEY_COPIER_EA_CONTEXT_MQH
