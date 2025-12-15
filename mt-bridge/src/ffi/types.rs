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

/// SSlaveConfig - Slave configuration for FFI
/// Layout: 8-byte types first, then 4-byte, then byte arrays - no internal padding
/// Total size: 344 bytes (8-byte aligned)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SSlaveConfig {
    // 8-byte fields first (f64) - 48 bytes total
    pub timestamp: i64, // Unix timestamp in milliseconds
    pub lot_multiplier: f64,
    pub source_lot_min: f64,
    pub source_lot_max: f64,
    pub master_equity: f64,
    pub market_sync_max_pips: f64,

    // 4-byte fields (i32/u32) - 48 bytes total
    pub status: i32,
    pub lot_calculation_mode: i32, // 0=Multiplier, 1=MarginRatio
    pub reverse_trade: i32,        // bool
    pub config_version: u32,
    pub sync_mode: i32, // 0=Skip, 1=Limit, 2=Market
    pub limit_order_expiry_min: i32,
    pub max_slippage: i32,
    pub copy_pending_orders: i32, // bool
    pub max_retries: i32,
    pub max_signal_delay_ms: i32,
    pub use_pending_order_for_delayed: i32, // bool
    pub allow_new_orders: i32,              // bool
    // No _reserved needed: 40 + 48 = 88 bytes (divisible by 8)

    // Byte arrays - 256 bytes total
    pub account_id: [u8; MAX_ACCOUNT_ID_LEN],     // 64
    pub master_account: [u8; MAX_ACCOUNT_ID_LEN], // 64
    pub trade_group_id: [u8; MAX_ACCOUNT_ID_LEN], // 64
    pub symbol_prefix: [u8; MAX_SYMBOL_LEN],      // 32
    pub symbol_suffix: [u8; MAX_SYMBOL_LEN],      // 32
}

/// SMasterConfig - Master configuration for FFI
/// Layout: 4-byte types first, then byte arrays
/// Total size: 136 bytes (8-byte aligned)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SMasterConfig {
    // 4-byte fields first
    pub status: i32,
    pub config_version: u32,
    // Byte arrays
    pub account_id: [u8; MAX_ACCOUNT_ID_LEN], // 64
    pub symbol_prefix: [u8; MAX_SYMBOL_LEN],  // 32
    pub symbol_suffix: [u8; MAX_SYMBOL_LEN],  // 32
}

/// SSymbolMapping - Symbol mapping pair for FFI
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SSymbolMapping {
    pub source: [u8; MAX_SYMBOL_LEN],
    pub target: [u8; MAX_SYMBOL_LEN],
}

/// SPositionInfo - Position data for FFI
/// Layout: 8-byte types first, then 4-byte + padding, then byte arrays
/// Total size: 160 bytes (8-byte aligned)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SPositionInfo {
    // 8-byte fields (0-55) - 56 bytes
    pub ticket: i64,       // 0-7
    pub open_time: i64,    // 8-15
    pub magic_number: i64, // 16-23
    pub lots: f64,         // 24-31
    pub open_price: f64,   // 32-39
    pub stop_loss: f64,    // 40-47
    pub take_profit: f64,  // 48-55

    // 4-byte field + padding (56-63) - 8 bytes
    pub order_type: i32,    // 56-59
    pub _reserved: [u8; 4], // 60-63 (explicit padding for 8-byte alignment)

    // Byte arrays (64-159) - 96 bytes
    pub symbol: [u8; MAX_SYMBOL_LEN],   // 64-95 (32 bytes)
    pub comment: [u8; MAX_COMMENT_LEN], // 96-159 (64 bytes)
}

/// SSyncRequest - Sync request for FFI
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SSyncRequest {
    pub slave_account: [u8; MAX_ACCOUNT_ID_LEN],
    pub master_account: [u8; MAX_ACCOUNT_ID_LEN],
    pub last_sync_time: [u8; 64], // Increased to 64 to avoid truncation of ISO8601 strings
}

/// SGlobalConfig - Global configuration for FFI
/// Layout: 4-byte types first, then byte arrays
/// Total size: 136 bytes (8-byte aligned)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SGlobalConfig {
    // 4-byte fields
    pub enabled: i32, // bool
    pub batch_size: i32,
    pub flush_interval_secs: i32,
    pub _reserved: i32, // Padding to align to 8 bytes

    // Byte arrays
    pub endpoint: [u8; MAX_SERVER_LEN], // 64
    pub log_level: [u8; 16],            // 16
    pub timestamp: [u8; 32],            // 32
}

impl Default for SSlaveConfig {
    fn default() -> Self {
        Self {
            // 8-byte fields
            timestamp: 0,
            lot_multiplier: 0.0,
            source_lot_min: 0.0,
            source_lot_max: 0.0,
            master_equity: 0.0,
            market_sync_max_pips: 0.0,
            // 4-byte fields
            status: 0,
            lot_calculation_mode: 0,
            reverse_trade: 0,
            config_version: 0,
            sync_mode: 0,
            limit_order_expiry_min: 0,
            max_slippage: 0,
            copy_pending_orders: 0,
            max_retries: 0,
            max_signal_delay_ms: 0,
            use_pending_order_for_delayed: 0,
            allow_new_orders: 0,
            // Byte arrays
            account_id: [0; MAX_ACCOUNT_ID_LEN],
            master_account: [0; MAX_ACCOUNT_ID_LEN],
            trade_group_id: [0; MAX_ACCOUNT_ID_LEN],
            symbol_prefix: [0; MAX_SYMBOL_LEN],
            symbol_suffix: [0; MAX_SYMBOL_LEN],
        }
    }
}

impl Default for SMasterConfig {
    fn default() -> Self {
        Self {
            status: 0,
            config_version: 0,
            account_id: [0; MAX_ACCOUNT_ID_LEN],
            symbol_prefix: [0; MAX_SYMBOL_LEN],
            symbol_suffix: [0; MAX_SYMBOL_LEN],
        }
    }
}

impl Default for SSymbolMapping {
    fn default() -> Self {
        Self {
            source: [0; MAX_SYMBOL_LEN],
            target: [0; MAX_SYMBOL_LEN],
        }
    }
}

impl Default for SPositionInfo {
    fn default() -> Self {
        Self {
            ticket: 0,
            open_time: 0,
            magic_number: 0,
            lots: 0.0,
            open_price: 0.0,
            stop_loss: 0.0,
            take_profit: 0.0,
            order_type: 0,
            _reserved: [0; 4],
            symbol: [0; MAX_SYMBOL_LEN],
            comment: [0; MAX_COMMENT_LEN],
        }
    }
}

impl Default for SSyncRequest {
    fn default() -> Self {
        Self {
            slave_account: [0; MAX_ACCOUNT_ID_LEN],
            master_account: [0; MAX_ACCOUNT_ID_LEN],
            last_sync_time: [0; 64],
        }
    }
}

impl Default for SGlobalConfig {
    fn default() -> Self {
        Self {
            enabled: 0,
            batch_size: 0,
            flush_interval_secs: 0,
            _reserved: 0,
            endpoint: [0; MAX_SERVER_LEN],
            log_level: [0; 16],
            timestamp: [0; 32],
        }
    }
}

// ============================================================================
// Static Size Assertions
// ============================================================================
// These ensure struct sizes EXACTLY match MQL expectations.
// If any of these fail, the FFI interface WILL be broken.

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn test_struct_exact_sizes() {
        // MQL側とバイト単位で一致しているか確認する

        // SSlaveConfig: 48(i64+f64) + 48(i32) + 256(arr) = 352
        assert_eq!(size_of::<SSlaveConfig>(), 352, "SSlaveConfig size mismatch");

        // SMasterConfig: 8(i32) + 128(arr) = 136
        assert_eq!(
            size_of::<SMasterConfig>(),
            136,
            "SMasterConfig size mismatch"
        );

        // SSymbolMapping: 32 + 32 = 64
        assert_eq!(
            size_of::<SSymbolMapping>(),
            64,
            "SSymbolMapping size mismatch"
        );

        // SPositionInfo: 56(i64/f64) + 4(i32) + 4(pad) + 96(arr) = 160
        assert_eq!(
            size_of::<SPositionInfo>(),
            160,
            "SPositionInfo size mismatch"
        );

        // SSyncRequest: 64 * 3 = 192
        assert_eq!(size_of::<SSyncRequest>(), 192, "SSyncRequest size mismatch");

        // SGlobalConfig: 16(i32) + 64(endpoint) + 16(log) + 32(time) = 128
        assert_eq!(
            size_of::<SGlobalConfig>(),
            128,
            "SGlobalConfig size mismatch"
        );
    }

    #[test]
    fn test_alignment() {
        // アライメントが8バイトであることを確認
        assert_eq!(std::mem::align_of::<SSlaveConfig>(), 8);
        assert_eq!(std::mem::align_of::<SPositionInfo>(), 8);
    }
}
