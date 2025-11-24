// Location: mt-bridge/src/msgpack/types.rs
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
    pub allowed_magic_numbers: Option<Vec<i32>>,
    #[serde(default)]
    pub blocked_magic_numbers: Option<Vec<i32>>,
}

/// Slave EA configuration message
/// Contains all configuration parameters for a Slave EA
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlaveConfigMessage {
    pub account_id: String,
    pub master_account: String,
    pub timestamp: String, // ISO 8601 format
    pub status: i32, // 0=DISABLED, 1=ENABLED (Master disconnected), 2=CONNECTED (Master connected)
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
}

/// Master EA configuration message
/// Contains configuration parameters for a Master EA
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterConfigMessage {
    pub account_id: String,
    #[serde(default)]
    pub symbol_prefix: Option<String>,
    #[serde(default)]
    pub symbol_suffix: Option<String>,
    pub config_version: u32,
    pub timestamp: String, // ISO 8601 format
}

/// Unregistration message structure
/// Sent when an EA disconnects from the relay server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnregisterMessage {
    pub message_type: String, // "Unregister"
    pub account_id: String,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeSignalMessage {
    pub action: String, // "Open", "Close", "Modify"
    pub ticket: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_type: Option<String>,
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
    pub timestamp: String,
    pub source_account: String,
}
