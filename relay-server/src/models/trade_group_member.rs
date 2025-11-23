// relay-server/src/models/trade_group_member.rs
//
// TradeGroupMember model: Represents a Slave account connected to a Master (TradeGroup).
// Each member has Slave-specific configuration and connection status.

use serde::{Deserialize, Serialize};
use sankey_copier_zmq::{SymbolMapping, TradeFilters};

/// Status constants for TradeGroupMember
pub const STATUS_DISABLED: i32 = 0;
pub const STATUS_ENABLED: i32 = 1;
pub const STATUS_CONNECTED: i32 = 2;

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

/// Slave-specific settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SlaveSettings {
    /// Lot multiplier for trade copying
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
}

impl TradeGroupMember {
    /// Create a new TradeGroupMember with default settings
    pub fn new(id: i32, trade_group_id: String, slave_account: String) -> Self {
        Self {
            id,
            trade_group_id,
            slave_account,
            slave_settings: SlaveSettings::default(),
            status: STATUS_ENABLED,
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
        let member = TradeGroupMember::new(
            1,
            "MASTER_001".to_string(),
            "SLAVE_001".to_string(),
        );

        assert_eq!(member.id, 1);
        assert_eq!(member.trade_group_id, "MASTER_001");
        assert_eq!(member.slave_account, "SLAVE_001");
        assert_eq!(member.status, STATUS_ENABLED);
        assert_eq!(member.slave_settings.config_version, 0);
        assert!(member.slave_settings.lot_multiplier.is_none());
        assert!(!member.slave_settings.reverse_trade);
    }

    #[test]
    fn test_increment_version() {
        let mut member = TradeGroupMember::new(
            1,
            "MASTER_001".to_string(),
            "SLAVE_001".to_string(),
        );
        let initial_version = member.slave_settings.config_version;

        member.increment_version();

        assert_eq!(member.slave_settings.config_version, initial_version + 1);
    }

    #[test]
    fn test_is_enabled() {
        let mut member = TradeGroupMember::new(
            1,
            "MASTER_001".to_string(),
            "SLAVE_001".to_string(),
        );

        member.status = STATUS_DISABLED;
        assert!(!member.is_enabled());

        member.status = STATUS_ENABLED;
        assert!(member.is_enabled());

        member.status = STATUS_CONNECTED;
        assert!(member.is_enabled());
    }

    #[test]
    fn test_is_connected() {
        let mut member = TradeGroupMember::new(
            1,
            "MASTER_001".to_string(),
            "SLAVE_001".to_string(),
        );

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
            lot_multiplier: Some(1.5),
            reverse_trade: true,
            symbol_prefix: None,
            symbol_suffix: None,
            symbol_mappings: vec![
                SymbolMapping {
                    source_symbol: "EURUSD".to_string(),
                    target_symbol: "EURUSDm".to_string(),
                },
            ],
            filters: TradeFilters {
                allowed_symbols: Some(vec!["EURUSD".to_string()]),
                blocked_symbols: None,
                allowed_magic_numbers: None,
                blocked_magic_numbers: None,
            },
            config_version: 1,
        };

        let json = serde_json::to_string(&settings).unwrap();
        let deserialized: SlaveSettings = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.lot_multiplier, Some(1.5));
        assert!(deserialized.reverse_trade);
        assert_eq!(deserialized.symbol_mappings.len(), 1);
        assert_eq!(deserialized.config_version, 1);
    }

    #[test]
    fn test_slave_settings_with_null_values() {
        let settings = SlaveSettings {
            lot_multiplier: None,
            reverse_trade: false,
            symbol_prefix: None,
            symbol_suffix: None,
            symbol_mappings: vec![],
            filters: TradeFilters::default(),
            config_version: 0,
        };

        let json = serde_json::to_string(&settings).unwrap();

        // Should not include null optional fields
        assert!(!json.contains("lot_multiplier"));
        assert!(!json.contains("symbol_prefix"));
        assert!(!json.contains("symbol_suffix"));

        // Should include default/empty fields
        assert!(json.contains("reverse_trade"));
        assert!(json.contains("symbol_mappings"));
        assert!(json.contains("config_version"));
    }
}
