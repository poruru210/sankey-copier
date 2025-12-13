// mt-bridge/src/ffi_types.rs
//
// C-Compatible Data Structures for FFI
//
// These structs are designed to be safe for passing across the FFI boundary.
// Strings are represented as fixed-size byte arrays (UTF-8, null-terminated).

// Constants for string lengths
pub const MAX_ACCOUNT_ID_LEN: usize = 64;
pub const MAX_SYMBOL_LEN: usize = 32;
pub const MAX_COMMENT_LEN: usize = 64;
pub const MAX_BROKER_LEN: usize = 64;
pub const MAX_SERVER_LEN: usize = 64;

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct CSlaveConfig {
    pub account_id: [u8; MAX_ACCOUNT_ID_LEN],
    pub master_account: [u8; MAX_ACCOUNT_ID_LEN],
    pub trade_group_id: [u8; MAX_ACCOUNT_ID_LEN],

    pub status: i32,
    pub lot_calculation_mode: i32, // 0=Multiplier, 1=MarginRatio
    pub lot_multiplier: f64,
    pub reverse_trade: i32, // bool

    pub symbol_prefix: [u8; MAX_SYMBOL_LEN],
    pub symbol_suffix: [u8; MAX_SYMBOL_LEN],

    pub config_version: u32,
    pub source_lot_min: f64,
    pub source_lot_max: f64,
    pub master_equity: f64,

    pub sync_mode: i32, // 0=Skip, 1=Limit, 2=Market
    pub limit_order_expiry_min: i32,
    pub market_sync_max_pips: f64,
    pub max_slippage: i32,
    pub copy_pending_orders: i32, // bool

    pub max_retries: i32,
    pub max_signal_delay_ms: i32,
    pub use_pending_order_for_delayed: i32, // bool
    pub allow_new_orders: i32,              // bool
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct CMasterConfig {
    pub account_id: [u8; MAX_ACCOUNT_ID_LEN],
    pub status: i32,
    pub symbol_prefix: [u8; MAX_SYMBOL_LEN],
    pub symbol_suffix: [u8; MAX_SYMBOL_LEN],
    pub config_version: u32,
    // Timestamp usually not critical for logic, but can add if needed
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct CSymbolMapping {
    pub source: [u8; MAX_SYMBOL_LEN],
    pub target: [u8; MAX_SYMBOL_LEN],
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct CPositionInfo {
    pub ticket: i64,
    pub symbol: [u8; MAX_SYMBOL_LEN],
    pub order_type: i32, // Mapped to Rust/MQL enum value
    pub lots: f64,
    pub open_price: f64,
    pub open_time: i64, // Unix timestamp
    pub stop_loss: f64,
    pub take_profit: f64,
    pub magic_number: i64,
    pub comment: [u8; MAX_COMMENT_LEN],
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct CSyncRequest {
    pub slave_account: [u8; MAX_ACCOUNT_ID_LEN],
    pub master_account: [u8; MAX_ACCOUNT_ID_LEN],
    pub last_sync_time: [u8; 64], // Increased to 64 to avoid truncation of ISO8601 strings
}

impl Default for CSlaveConfig {
    fn default() -> Self {
        Self {
            account_id: [0; MAX_ACCOUNT_ID_LEN],
            master_account: [0; MAX_ACCOUNT_ID_LEN],
            trade_group_id: [0; MAX_ACCOUNT_ID_LEN],
            status: 0,
            lot_calculation_mode: 0,
            lot_multiplier: 0.0,
            reverse_trade: 0,
            symbol_prefix: [0; MAX_SYMBOL_LEN],
            symbol_suffix: [0; MAX_SYMBOL_LEN],
            config_version: 0,
            source_lot_min: 0.0,
            source_lot_max: 0.0,
            master_equity: 0.0,
            sync_mode: 0,
            limit_order_expiry_min: 0,
            market_sync_max_pips: 0.0,
            max_slippage: 0,
            copy_pending_orders: 0,
            max_retries: 0,
            max_signal_delay_ms: 0,
            use_pending_order_for_delayed: 0,
            allow_new_orders: 0,
        }
    }
}

impl Default for CMasterConfig {
    fn default() -> Self {
        Self {
            account_id: [0; MAX_ACCOUNT_ID_LEN],
            status: 0,
            symbol_prefix: [0; MAX_SYMBOL_LEN],
            symbol_suffix: [0; MAX_SYMBOL_LEN],
            config_version: 0,
        }
    }
}

impl Default for CSymbolMapping {
    fn default() -> Self {
        Self {
            source: [0; MAX_SYMBOL_LEN],
            target: [0; MAX_SYMBOL_LEN],
        }
    }
}

impl Default for CPositionInfo {
    fn default() -> Self {
        Self {
            ticket: 0,
            symbol: [0; MAX_SYMBOL_LEN],
            order_type: 0,
            lots: 0.0,
            open_price: 0.0,
            open_time: 0,
            stop_loss: 0.0,
            take_profit: 0.0,
            magic_number: 0,
            comment: [0; MAX_COMMENT_LEN],
        }
    }
}

impl Default for CSyncRequest {
    fn default() -> Self {
        Self {
            slave_account: [0; MAX_ACCOUNT_ID_LEN],
            master_account: [0; MAX_ACCOUNT_ID_LEN],
            last_sync_time: [0; 64],
        }
    }
}
