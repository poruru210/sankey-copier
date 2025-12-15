//+------------------------------------------------------------------+
//|                                                    FFITypes.mqh |
//|                        Copyright 2025, SANKEY Copier Project    |
//|                                                                  |
//| C-Compatible Structs for FFI (matching mt-bridge/src/ffi_types.rs)
//| These definitions must stay in sync with the Rust ffi_types.rs.
//|
//| ALIGNMENT RULES:
//| - Both sides use natural C alignment (no pack directives)
//| - Fields ordered: 8-byte types first, then 4-byte, then byte arrays
//| - Explicit _reserved padding ensures 8-byte aligned total size
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, SANKEY Copier Project"
#property strict

#ifndef FFI_TYPES_MQH
#define FFI_TYPES_MQH

//+------------------------------------------------------------------+
//| SSlaveConfig - Slave configuration                               |
//| Fields: f64 first, then i32/u32, then byte arrays                |
//| Total size: 344 bytes (8-byte aligned)                           |
//+------------------------------------------------------------------+
struct SSlaveConfig {
    // 8-byte fields first (f64) - 40 bytes
    double lot_multiplier;
    double source_lot_min;
    double source_lot_max;
    double master_equity;
    double market_sync_max_pips;

    // 4-byte fields (int/uint) - 48 bytes (40+48=88, divisible by 8)
    int status;
    int lot_calculation_mode;
    int reverse_trade;
    uint config_version;
    int sync_mode;
    int limit_order_expiry_min;
    int max_slippage;
    int copy_pending_orders;
    int max_retries;
    int max_signal_delay_ms;
    int use_pending_order_for_delayed;
    int allow_new_orders;

    // Byte arrays - 256 bytes
    uchar account_id[64];
    uchar master_account[64];
    uchar trade_group_id[64];
    uchar symbol_prefix[32];
    uchar symbol_suffix[32];
};

//+------------------------------------------------------------------+
//| SMasterConfig - Master configuration                             |
//| Fields: i32/u32 first, then byte arrays                          |
//| Total size: 136 bytes (8-byte aligned)                           |
//+------------------------------------------------------------------+
struct SMasterConfig {
    // 4-byte fields first
    int status;
    uint config_version;
    // Byte arrays
    uchar account_id[64];
    uchar symbol_prefix[32];
    uchar symbol_suffix[32];
};

//+------------------------------------------------------------------+
//| SSymbolMapping - Symbol mapping pair                             |
//| Total size: 64 bytes                                             |
//+------------------------------------------------------------------+
struct SSymbolMapping {
    uchar source[32];
    uchar target[32];
};

//+------------------------------------------------------------------+
//| SPositionInfo - Position data for sync                         |
//| Fields: 8-byte first, then 4-byte + padding, then byte arrays    |
//| Total size: 160 bytes (8-byte aligned)                           |
//+------------------------------------------------------------------+
struct SPositionInfo {
    // 8-byte fields (0-55) - 56 bytes
    long ticket;        // 0-7
    long open_time;     // 8-15
    long magic_number;  // 16-23
    double lots;        // 24-31
    double open_price;  // 32-39
    double stop_loss;   // 40-47
    double take_profit; // 48-55

    // 4-byte field + padding (56-63) - 8 bytes
    int order_type;      // 56-59
    uchar _reserved[4];  // 60-63 (explicit padding for 8-byte alignment)

    // Byte arrays (64-159) - 96 bytes
    uchar symbol[32];    // 64-95
    uchar comment[64];   // 96-159
};

//+------------------------------------------------------------------+
//| SSyncRequest - Sync request from slave to master                 |
//| Total size: 192 bytes                                            |
//+------------------------------------------------------------------+
struct SSyncRequest {
    uchar slave_account[64];
    uchar master_account[64];
    uchar last_sync_time[64];
};

//+------------------------------------------------------------------+
//| SGlobalConfig - Global configuration                             |
//| Broadcasted on config/global                                     |
//| Total size: 136 bytes                                            |
//+------------------------------------------------------------------+
struct SGlobalConfig {
    // 4-byte fields
    int enabled;
    int batch_size;
    int flush_interval_secs;
    int _reserved;

    // Byte arrays
    uchar endpoint[64];
    uchar log_level[16];
    uchar timestamp[32];
};

#endif // FFI_TYPES_MQH
