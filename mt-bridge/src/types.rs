// Location: mt-bridge/src/types.rs
// Purpose: Type definitions for MessagePack messages exchanged between EA and relay-server
// Why: Centralized type definitions for all configuration and trade signal messages

use serde::{Deserialize, Serialize};

/// Symbol mapping structure
/// Maps source symbols to target symbols for cross-broker trading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolMapping {
    pub source_symbol: String,
    pub target_symbol: String,
}

/// Trade filters structure
/// Defines allowed/blocked symbols and magic numbers for trade filtering
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TradeFilters {
    #[serde(default)]
    pub allowed_symbols: Option<Vec<String>>,
    #[serde(default)]
    pub blocked_symbols: Option<Vec<String>>,
    #[serde(default)]
    pub allowed_magic_numbers: Option<Vec<i64>>,
    #[serde(default)]
    pub blocked_magic_numbers: Option<Vec<i64>>,
}

/// Lot calculation mode
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LotCalculationMode {
    #[default]
    Multiplier,
    MarginRatio,
}

/// Sync mode for existing positions when slave connects
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SyncMode {
    /// Do not sync existing positions (only copy new trades)
    #[default]
    Skip,
    /// Sync using limit orders at Master's open price
    LimitOrder,
    /// Sync using market orders with max price deviation check
    MarketOrder,
}

/// Slave EA configuration message
/// Contains all configuration parameters for a Slave EA
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlaveConfigMessage {
    pub account_id: String,
    pub master_account: String,
    pub timestamp: String,      // ISO 8601 format
    pub trade_group_id: String, // Used for ZMQ subscription
    pub status: i32, // 0=DISABLED, 1=ENABLED (Master disconnected), 2=CONNECTED (Master connected)
    #[serde(default)]
    pub lot_calculation_mode: LotCalculationMode,
    #[serde(default)]
    pub lot_multiplier: Option<f64>,
    pub reverse_trade: bool,
    #[serde(default)]
    pub symbol_prefix: Option<String>,
    #[serde(default)]
    pub symbol_suffix: Option<String>,
    pub symbol_mappings: Vec<SymbolMapping>,
    pub filters: TradeFilters,
    pub config_version: u32,
    /// Minimum lot size filter: skip trades with lot smaller than this value
    #[serde(default)]
    pub source_lot_min: Option<f64>,
    /// Maximum lot size filter: skip trades with lot larger than this value
    #[serde(default)]
    pub source_lot_max: Option<f64>,
    /// Master's current equity (for margin_ratio mode calculation)
    #[serde(default)]
    pub master_equity: Option<f64>,

    // === Open Sync Policy Settings ===
    /// Sync mode for existing positions when slave connects
    #[serde(default)]
    pub sync_mode: SyncMode,
    /// Time limit for limit orders in minutes (0 = GTC, Good Till Cancelled)
    /// Used when sync_mode = LimitOrder
    #[serde(default)]
    pub limit_order_expiry_min: Option<i32>,
    /// Max price deviation in pips for market order sync (skip if exceeded)
    /// Used when sync_mode = MarketOrder
    #[serde(default)]
    pub market_sync_max_pips: Option<f64>,
    /// Maximum allowed slippage in points when opening positions (default: 30)
    #[serde(default)]
    pub max_slippage: Option<i32>,
    /// Whether to copy pending orders (limit/stop orders) in addition to market orders
    #[serde(default)]
    pub copy_pending_orders: bool,

    // === Trade Execution Settings ===
    /// Maximum number of order retries on failure (default: 3)
    #[serde(default = "default_max_retries")]
    pub max_retries: i32,
    /// Maximum allowed signal delay in milliseconds (default: 5000)
    #[serde(default = "default_max_signal_delay_ms")]
    pub max_signal_delay_ms: i32,
    /// Use pending order for delayed signals instead of skipping
    #[serde(default)]
    pub use_pending_order_for_delayed: bool,
    /// Allow opening new orders (derived from status: true when status > 0)
    #[serde(default = "default_allow_new_orders")]
    pub allow_new_orders: bool,
    /// Detailed warning codes to help UI/EA show root causes (empty when healthy)
    #[serde(default)]
    pub warning_codes: Vec<WarningCode>,
}

fn default_max_retries() -> i32 {
    3
}

fn default_max_signal_delay_ms() -> i32 {
    5000
}

fn default_allow_new_orders() -> bool {
    true
}

/// Master EA configuration message
/// Contains configuration parameters for a Master EA
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterConfigMessage {
    pub account_id: String,
    /// Master status: 0=DISABLED, 2=CONNECTED
    /// Note: Master has no ENABLED state. It's either DISABLED or CONNECTED.
    pub status: i32,
    pub symbol_prefix: Option<String>,
    pub symbol_suffix: Option<String>,
    pub config_version: u32,
    pub timestamp: String, // ISO 8601 format
    /// Warning codes describing why master is disabled (if any)
    #[serde(default)]
    pub warning_codes: Vec<WarningCode>,
}

/// Warning codes that describe why a runtime status is degraded/disabled.
/// Lower priority values indicate higher severity (displayed first in UI).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum WarningCode {
    SlaveWebUiDisabled,
    SlaveOffline,
    SlaveAutoTradingDisabled,
    MasterWebUiDisabled,
    MasterOffline,
    MasterAutoTradingDisabled,
    MasterClusterDegraded,
    NoMasterAssigned,
}

impl WarningCode {
    /// Returns the display priority of this warning code.
    /// Lower values = higher priority (should be displayed first).
    /// Slave-side issues have higher priority than Master-side issues.
    pub fn priority(&self) -> u8 {
        match self {
            // Slave-side issues (highest priority - user can fix these directly)
            WarningCode::SlaveWebUiDisabled => 10,
            WarningCode::SlaveOffline => 20,
            WarningCode::SlaveAutoTradingDisabled => 30,
            // Master-side issues (medium priority)
            WarningCode::MasterWebUiDisabled => 40,
            WarningCode::MasterOffline => 50,
            WarningCode::MasterAutoTradingDisabled => 60,
            // Configuration issues (lowest priority)
            WarningCode::NoMasterAssigned => 70,
            WarningCode::MasterClusterDegraded => 80,
        }
    }

    /// Sort warning codes by priority (lowest priority value first).
    pub fn sort_by_priority(codes: &mut [WarningCode]) {
        codes.sort_by_key(|c| c.priority());
    }
}

/// Unregistration message structure
/// Sent when an EA disconnects from the relay server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnregisterMessage {
    pub message_type: String, // "Unregister"
    pub account_id: String,
    pub timestamp: String,
    #[serde(default)]
    pub ea_type: Option<String>, // "Master" or "Slave" (optional for backward compatibility)
}

/// Registration message structure
/// Sent when an EA connects to register itself with the relay server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterMessage {
    pub message_type: String, // "Register"
    pub account_id: String,
    pub ea_type: String,  // "Master" or "Slave"
    pub platform: String, // "MT4" or "MT5"
    pub account_number: i64,
    pub broker: String,
    pub account_name: String,
    pub server: String,
    pub currency: String,
    pub leverage: i64,
    pub timestamp: String,
}

/// Request configuration message structure (for Slave EAs)
/// Sent to request latest configuration from relay server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestConfigMessage {
    pub message_type: String, // "RequestConfig"
    pub account_id: String,
    pub timestamp: String,
    pub ea_type: String, // "Master" or "Slave"
}

/// Heartbeat message structure (includes all EA information for auto-registration)
/// Sent periodically to maintain connection and provide EA status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatMessage {
    pub message_type: String, // "Heartbeat"
    pub account_id: String,
    pub balance: f64,
    pub equity: f64,
    pub open_positions: i32,
    pub timestamp: String,
    pub version: String, // Build version information
    // EA identification fields (for auto-registration)
    pub ea_type: String,  // "Master" or "Slave"
    pub platform: String, // "MT4" or "MT5"
    pub account_number: i64,
    pub broker: String,
    pub account_name: String,
    pub server: String,
    pub currency: String,
    pub leverage: i64,
    // Auto-trading state (IsTradeAllowed)
    pub is_trade_allowed: bool,
    // Symbol configuration
    #[serde(default)]
    pub symbol_prefix: Option<String>,
    #[serde(default)]
    pub symbol_suffix: Option<String>,
    #[serde(default)]
    pub symbol_map: Option<String>,
}

/// Trade signal message structure
/// Represents a trade action (Open, Close, Modify) to be copied
/// This is the shared type used across mt-bridge, relay-server, and e2e-tests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeSignal {
    pub action: crate::constants::TradeAction,
    pub ticket: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_type: Option<crate::constants::OrderType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lots: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open_price: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_loss: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub take_profit: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub magic_number: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub source_account: String,
    /// Close ratio for partial close (0.0-1.0)
    /// None or 1.0 = full close, 0.0 < ratio < 1.0 = partial close
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub close_ratio: Option<f64>,
}

// =============================================================================
// Position Sync Protocol Messages
// =============================================================================

/// Position information for sync protocol
/// Represents a single open position on Master EA
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionInfo {
    pub ticket: i64,
    pub symbol: String,
    pub order_type: String, // "Buy", "Sell", "BuyLimit", etc.
    pub lots: f64,
    pub open_price: f64,
    pub open_time: String, // ISO 8601 format
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_loss: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub take_profit: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub magic_number: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

/// Position snapshot message (Master → Slave via Relay)
/// Sent when Master restarts or in response to SyncRequest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionSnapshotMessage {
    pub message_type: String, // "PositionSnapshot"
    pub source_account: String,
    pub positions: Vec<PositionInfo>,
    pub timestamp: String, // ISO 8601 format
}

/// Sync request message (Slave → Master via Relay)
/// Sent when Slave starts up and needs to sync with Master
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequestMessage {
    pub message_type: String, // "SyncRequest"
    pub slave_account: String,
    pub master_account: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_sync_time: Option<String>, // ISO 8601 format, if known
    pub timestamp: String,
}

// =============================================================================
// VictoriaLogs Configuration Message
// =============================================================================

/// VictoriaLogs configuration message
/// Broadcasted to all EAs on "config/global" topic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VLogsConfigMessage {
    /// Whether VictoriaLogs logging is enabled
    pub enabled: bool,
    /// VictoriaLogs endpoint URL
    pub endpoint: String,
    /// Number of log entries to batch before sending
    pub batch_size: i32,
    /// Interval in seconds between automatic flushes
    pub flush_interval_secs: i32,
    /// Minimum log level to output: "DEBUG", "INFO", "WARN", "ERROR"
    /// Logs below this level will be ignored by EAs
    #[serde(default)]
    pub log_level: String,
    /// Timestamp when this config was sent (ISO 8601)
    pub timestamp: String,
}

impl Default for SlaveConfigMessage {
    fn default() -> Self {
        Self {
            account_id: String::new(),
            master_account: String::new(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            trade_group_id: String::new(),
            status: 0,
            lot_calculation_mode: LotCalculationMode::default(),
            lot_multiplier: None,
            reverse_trade: false,
            symbol_prefix: None,
            symbol_suffix: None,
            symbol_mappings: Vec::new(),
            filters: TradeFilters::default(),
            config_version: 0,
            source_lot_min: None,
            source_lot_max: None,
            master_equity: None,
            sync_mode: SyncMode::default(),
            limit_order_expiry_min: None,
            market_sync_max_pips: None,
            max_slippage: None,
            copy_pending_orders: false,
            max_retries: default_max_retries(),
            max_signal_delay_ms: default_max_signal_delay_ms(),
            use_pending_order_for_delayed: false,
            allow_new_orders: default_allow_new_orders(),
            warning_codes: Vec::new(),
        }
    }
}

impl Default for TradeSignal {
    fn default() -> Self {
        Self {
            action: crate::constants::TradeAction::Open,
            ticket: 0,
            symbol: None,
            order_type: None,
            lots: None,
            open_price: None,
            stop_loss: None,
            take_profit: None,
            magic_number: None,
            comment: None,
            timestamp: chrono::Utc::now(),
            source_account: String::new(),
            close_ratio: None,
        }
    }
}
