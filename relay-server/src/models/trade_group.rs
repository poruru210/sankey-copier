// relay-server/src/models/trade_group.rs
//
// TradeGroup model: Represents a Master account and its settings.
// A TradeGroup is identified by the master_account and contains Master-specific
// configuration that applies to all connected Slaves.

use serde::{Deserialize, Serialize};

/// TradeGroup represents a Master account and its configuration.
/// The id field is the master_account itself.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeGroup {
    /// Master account ID (also serves as the TradeGroup ID)
    pub id: String,

    /// Master-specific settings (stored as JSON in DB)
    pub master_settings: MasterSettings,

    /// Timestamp when the TradeGroup was created
    pub created_at: String,

    /// Timestamp when the TradeGroup was last updated
    pub updated_at: String,
}

/// Master-specific settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MasterSettings {
    /// Symbol prefix to remove from Master EA symbols (e.g., "pro.")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol_prefix: Option<String>,

    /// Symbol suffix to remove from Master EA symbols (e.g., ".m")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol_suffix: Option<String>,

    /// Configuration version for tracking updates
    #[serde(default)]
    pub config_version: u32,
}

impl TradeGroup {
    /// Create a new TradeGroup with default settings
    pub fn new(master_account: String) -> Self {
        Self {
            id: master_account,
            master_settings: MasterSettings::default(),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Increment the config version (used when settings change)
    pub fn increment_version(&mut self) {
        self.master_settings.config_version += 1;
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trade_group_creation() {
        let tg = TradeGroup::new("MASTER_001".to_string());

        assert_eq!(tg.id, "MASTER_001");
        assert_eq!(tg.master_settings.config_version, 0);
        assert!(tg.master_settings.symbol_prefix.is_none());
        assert!(tg.master_settings.symbol_suffix.is_none());
    }

    #[test]
    fn test_increment_version() {
        let mut tg = TradeGroup::new("MASTER_001".to_string());
        let initial_version = tg.master_settings.config_version;

        tg.increment_version();

        assert_eq!(tg.master_settings.config_version, initial_version + 1);
    }

    #[test]
    fn test_master_settings_serialization() {
        let settings = MasterSettings {
            symbol_prefix: Some("pro.".to_string()),
            symbol_suffix: Some(".m".to_string()),
            config_version: 1,
        };

        let json = serde_json::to_string(&settings).unwrap();
        let deserialized: MasterSettings = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.symbol_prefix, Some("pro.".to_string()));
        assert_eq!(deserialized.symbol_suffix, Some(".m".to_string()));
        assert_eq!(deserialized.config_version, 1);
    }

    #[test]
    fn test_master_settings_with_null_values() {
        let settings = MasterSettings {
            symbol_prefix: None,
            symbol_suffix: None,
            config_version: 0,
        };

        let json = serde_json::to_string(&settings).unwrap();

        // Should not include null fields in JSON
        assert!(!json.contains("symbol_prefix"));
        assert!(!json.contains("symbol_suffix"));
        assert!(json.contains("config_version"));
    }
}
