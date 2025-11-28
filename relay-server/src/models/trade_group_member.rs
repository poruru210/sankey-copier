// relay-server/src/models/trade_group_member.rs
//
// TradeGroupMember model: Represents a Slave account connected to a Master (TradeGroup).
// Each member has Slave-specific configuration and connection status.

use sankey_copier_zmq::{SymbolMapping, TradeFilters};
use serde::{Deserialize, Serialize};

/// Status constants for TradeGroupMember
#[allow(dead_code)]
pub const STATUS_DISABLED: i32 = 0;
#[allow(dead_code)]
pub const STATUS_ENABLED: i32 = 1;
#[allow(dead_code)]
pub const STATUS_CONNECTED: i32 = 2;
#[allow(dead_code)]
pub const STATUS_REMOVED: i32 = 4;

/// TradeGroupMember represents a Slave account connected to a TradeGroup (Master)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeGroupMember {
    /// Unique ID for backward compatibility with REST API
    pub id: i32,

    /// TradeGroup ID (master_account)
    pub trade_group_id: String,

    /// Slave account ID
    pub slave_account: String,

    /// Slave-specific settings (stored as JSON in DB)
    pub slave_settings: SlaveSettings,

    /// Connection status: 0=DISABLED, 1=ENABLED, 2=CONNECTED
    pub status: i32,

    /// Timestamp when the member was created
    pub created_at: String,

    /// Timestamp when the member was last updated
    pub updated_at: String,
}

/// Lot calculation mode
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LotCalculationMode {
    #[default]
    Multiplier,
    MarginRatio,
}

impl From<LotCalculationMode> for sankey_copier_zmq::LotCalculationMode {
    fn from(mode: LotCalculationMode) -> Self {
        match mode {
            LotCalculationMode::Multiplier => sankey_copier_zmq::LotCalculationMode::Multiplier,
            LotCalculationMode::MarginRatio => sankey_copier_zmq::LotCalculationMode::MarginRatio,
        }
    }
}

impl From<SyncMode> for sankey_copier_zmq::SyncMode {
    fn from(mode: SyncMode) -> Self {
        match mode {
            SyncMode::Skip => sankey_copier_zmq::SyncMode::Skip,
            SyncMode::LimitOrder => sankey_copier_zmq::SyncMode::LimitOrder,
            SyncMode::MarketOrder => sankey_copier_zmq::SyncMode::MarketOrder,
        }
    }
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

/// Slave-specific settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SlaveSettings {
    /// Lot calculation mode: "multiplier" (fixed) or "margin_ratio" (equity-based)
    #[serde(default)]
    pub lot_calculation_mode: LotCalculationMode,

    /// Lot multiplier for trade copying (used when mode is "multiplier")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lot_multiplier: Option<f64>,

    /// Reverse trade direction (buy → sell, sell → buy)
    #[serde(default)]
    pub reverse_trade: bool,

    /// Symbol prefix (currently in DB but not used by Slave EA - TODO Phase 2)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol_prefix: Option<String>,

    /// Symbol suffix (currently in DB but not used by Slave EA - TODO Phase 2)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol_suffix: Option<String>,

    /// Symbol mappings for converting Master symbols to Slave symbols
    #[serde(default)]
    pub symbol_mappings: Vec<SymbolMapping>,

    /// Trade filters (allowed/blocked symbols and magic numbers)
    #[serde(default)]
    pub filters: TradeFilters,

    /// Configuration version for tracking updates
    #[serde(default)]
    pub config_version: u32,

    /// Minimum lot size filter: skip trades with lot smaller than this value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_lot_min: Option<f64>,

    /// Maximum lot size filter: skip trades with lot larger than this value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_lot_max: Option<f64>,

    // === Open Sync Policy Settings ===
    /// Sync mode for existing positions when slave connects
    #[serde(default)]
    pub sync_mode: SyncMode,

    /// Time limit for limit orders in minutes (0 = GTC, Good Till Cancelled)
    /// Used when sync_mode = LimitOrder
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_order_expiry_min: Option<i32>,

    /// Max price deviation in pips for market order sync (skip if exceeded)
    /// Used when sync_mode = MarketOrder
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market_sync_max_pips: Option<f64>,

    /// Maximum allowed slippage in points when opening positions (default: 30)
    #[serde(skip_serializing_if = "Option::is_none")]
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
}

fn default_max_retries() -> i32 {
    3
}

fn default_max_signal_delay_ms() -> i32 {
    5000
}

#[allow(dead_code)]
impl TradeGroupMember {
    /// Create a new TradeGroupMember with default settings
    /// NOTE: Initial status is DISABLED - user must explicitly enable
    pub fn new(id: i32, trade_group_id: String, slave_account: String) -> Self {
        Self {
            id,
            trade_group_id,
            slave_account,
            slave_settings: SlaveSettings::default(),
            status: STATUS_DISABLED,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Increment the config version (used when settings change)
    pub fn increment_version(&mut self) {
        self.slave_settings.config_version += 1;
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }

    /// Check if the member is enabled (status > 0)
    pub fn is_enabled(&self) -> bool {
        self.status > STATUS_DISABLED
    }

    /// Check if the member is connected (status == 2)
    pub fn is_connected(&self) -> bool {
        self.status == STATUS_CONNECTED
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_member_creation() {
        let member = TradeGroupMember::new(1, "MASTER_001".to_string(), "SLAVE_001".to_string());

        assert_eq!(member.id, 1);
        assert_eq!(member.trade_group_id, "MASTER_001");
        assert_eq!(member.slave_account, "SLAVE_001");
        assert_eq!(member.status, STATUS_DISABLED); // Initial status is DISABLED
        assert_eq!(member.slave_settings.config_version, 0);
        assert!(member.slave_settings.lot_multiplier.is_none());
        assert!(!member.slave_settings.reverse_trade);
    }

    #[test]
    fn test_increment_version() {
        let mut member =
            TradeGroupMember::new(1, "MASTER_001".to_string(), "SLAVE_001".to_string());
        let initial_version = member.slave_settings.config_version;

        member.increment_version();

        assert_eq!(member.slave_settings.config_version, initial_version + 1);
    }

    #[test]
    fn test_is_enabled() {
        let mut member =
            TradeGroupMember::new(1, "MASTER_001".to_string(), "SLAVE_001".to_string());

        member.status = STATUS_DISABLED;
        assert!(!member.is_enabled());

        member.status = STATUS_ENABLED;
        assert!(member.is_enabled());

        member.status = STATUS_CONNECTED;
        assert!(member.is_enabled());
    }

    #[test]
    fn test_is_connected() {
        let mut member =
            TradeGroupMember::new(1, "MASTER_001".to_string(), "SLAVE_001".to_string());

        member.status = STATUS_DISABLED;
        assert!(!member.is_connected());

        member.status = STATUS_ENABLED;
        assert!(!member.is_connected());

        member.status = STATUS_CONNECTED;
        assert!(member.is_connected());
    }

    #[test]
    fn test_slave_settings_serialization() {
        let settings = SlaveSettings {
            lot_calculation_mode: LotCalculationMode::Multiplier,
            lot_multiplier: Some(1.5),
            reverse_trade: true,
            symbol_prefix: None,
            symbol_suffix: None,
            symbol_mappings: vec![SymbolMapping {
                source_symbol: "EURUSD".to_string(),
                target_symbol: "EURUSDm".to_string(),
            }],
            filters: TradeFilters {
                allowed_symbols: Some(vec!["EURUSD".to_string()]),
                blocked_symbols: None,
                allowed_magic_numbers: None,
                blocked_magic_numbers: None,
            },
            config_version: 1,
            source_lot_min: Some(0.01),
            source_lot_max: Some(10.0),
            sync_mode: SyncMode::Skip,
            limit_order_expiry_min: None,
            market_sync_max_pips: None,
            max_slippage: None,
            copy_pending_orders: false,
            max_retries: 5,
            max_signal_delay_ms: 3000,
            use_pending_order_for_delayed: true,
        };

        let json = serde_json::to_string(&settings).unwrap();
        let deserialized: SlaveSettings = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.lot_multiplier, Some(1.5));
        assert!(deserialized.reverse_trade);
        assert_eq!(deserialized.symbol_mappings.len(), 1);
        assert_eq!(deserialized.config_version, 1);
        assert_eq!(deserialized.source_lot_min, Some(0.01));
        assert_eq!(deserialized.source_lot_max, Some(10.0));
        assert_eq!(deserialized.max_retries, 5);
        assert_eq!(deserialized.max_signal_delay_ms, 3000);
        assert!(deserialized.use_pending_order_for_delayed);
    }

    #[test]
    fn test_slave_settings_with_null_values() {
        let settings = SlaveSettings {
            lot_calculation_mode: LotCalculationMode::default(),
            lot_multiplier: None,
            reverse_trade: false,
            symbol_prefix: None,
            symbol_suffix: None,
            symbol_mappings: vec![],
            filters: TradeFilters::default(),
            config_version: 0,
            source_lot_min: None,
            source_lot_max: None,
            sync_mode: SyncMode::Skip,
            limit_order_expiry_min: None,
            market_sync_max_pips: None,
            max_slippage: None,
            copy_pending_orders: false,
            max_retries: 3,
            max_signal_delay_ms: 5000,
            use_pending_order_for_delayed: false,
        };

        let json = serde_json::to_string(&settings).unwrap();

        // Should not include null optional fields
        assert!(!json.contains("\"lot_multiplier\""));
        assert!(!json.contains("\"symbol_prefix\""));
        assert!(!json.contains("\"symbol_suffix\""));
        assert!(!json.contains("\"source_lot_min\""));
        assert!(!json.contains("\"source_lot_max\""));

        // Should include default/empty fields
        assert!(json.contains("reverse_trade"));
        assert!(json.contains("symbol_mappings"));
        assert!(json.contains("config_version"));
        assert!(json.contains("max_retries"));
        assert!(json.contains("max_signal_delay_ms"));
    }

    #[test]
    fn test_lot_calculation_mode_serialization() {
        let mode = LotCalculationMode::MarginRatio;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"margin_ratio\"");

        let mode2 = LotCalculationMode::Multiplier;
        let json2 = serde_json::to_string(&mode2).unwrap();
        assert_eq!(json2, "\"multiplier\"");
    }

    #[test]
    fn test_sync_mode_serialization() {
        let mode = SyncMode::Skip;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"skip\"");

        let mode2 = SyncMode::LimitOrder;
        let json2 = serde_json::to_string(&mode2).unwrap();
        assert_eq!(json2, "\"limit_order\"");

        let mode3 = SyncMode::MarketOrder;
        let json3 = serde_json::to_string(&mode3).unwrap();
        assert_eq!(json3, "\"market_order\"");
    }
}
