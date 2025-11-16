use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// Re-export shared message types from DLL
pub use sankey_copier_zmq::{HeartbeatMessage, RequestConfigMessage, UnregisterMessage};

/// EA接続情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EaConnection {
    pub account_id: String,
    pub ea_type: EaType,
    pub platform: Platform,
    pub account_number: i64,
    pub broker: String,
    pub account_name: String,
    pub server: String,
    pub balance: f64,
    pub equity: f64,
    pub currency: String,
    pub leverage: i64,
    pub last_heartbeat: DateTime<Utc>,
    pub status: ConnectionStatus,
    pub connected_at: DateTime<Utc>,
}

/// EAの種類
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum EaType {
    Master,
    Slave,
}

impl EaType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Master" => Some(EaType::Master),
            "Slave" => Some(EaType::Slave),
            _ => None,
        }
    }
}

/// プラットフォームの種類
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Platform {
    MT4,
    MT5,
}

impl Platform {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "MT4" => Some(Platform::MT4),
            "MT5" => Some(Platform::MT5),
            _ => None,
        }
    }
}

/// 接続状態
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ConnectionStatus {
    Online,
    Offline,
    Timeout,
}

// Re-export ConfigMessage from DLL
pub use sankey_copier_zmq::ConfigMessage;

/// Convert CopySettings to ConfigMessage
impl From<crate::models::CopySettings> for ConfigMessage {
    fn from(settings: crate::models::CopySettings) -> Self {
        Self {
            account_id: settings.slave_account.clone(),
            master_account: settings.master_account.clone(),
            trade_group_id: settings.master_account, // master_accountと同じ
            timestamp: chrono::Utc::now().to_rfc3339(),
            enabled: settings.enabled,
            lot_multiplier: settings.lot_multiplier,
            reverse_trade: settings.reverse_trade,
            symbol_mappings: settings.symbol_mappings,
            filters: settings.filters,
            config_version: 1, // 初期バージョン
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{CopySettings, SymbolMapping, TradeFilters};

    #[test]
    fn test_config_message_from_copy_settings() {
        let settings = CopySettings {
            id: 1,
            enabled: true,
            master_account: "MASTER_001".to_string(),
            slave_account: "SLAVE_001".to_string(),
            lot_multiplier: Some(1.5),
            reverse_trade: false,
            symbol_mappings: vec![],
            filters: TradeFilters {
                allowed_symbols: None,
                blocked_symbols: None,
                allowed_magic_numbers: None,
                blocked_magic_numbers: None,
            },
        };

        let config: ConfigMessage = settings.into();

        assert_eq!(config.account_id, "SLAVE_001");
        assert_eq!(config.master_account, "MASTER_001");
        assert_eq!(config.trade_group_id, "MASTER_001");
        assert_eq!(config.enabled, true);
        assert_eq!(config.lot_multiplier, Some(1.5));
        assert_eq!(config.reverse_trade, false);
        assert_eq!(config.config_version, 1);
        assert_eq!(config.symbol_mappings.len(), 0);
    }

    #[test]
    fn test_config_message_with_mappings_and_filters() {
        let settings = CopySettings {
            id: 2,
            enabled: false,
            master_account: "MASTER_002".to_string(),
            slave_account: "SLAVE_002".to_string(),
            lot_multiplier: None,
            reverse_trade: true,
            symbol_mappings: vec![SymbolMapping {
                source_symbol: "EURUSD".to_string(),
                target_symbol: "EURUSDm".to_string(),
            }],
            filters: TradeFilters {
                allowed_symbols: Some(vec!["EURUSD".to_string(), "GBPUSD".to_string()]),
                blocked_symbols: None,
                allowed_magic_numbers: Some(vec![123, 456]),
                blocked_magic_numbers: None,
            },
        };

        let config: ConfigMessage = settings.into();

        assert_eq!(config.enabled, false);
        assert_eq!(config.lot_multiplier, None);
        assert_eq!(config.reverse_trade, true);
        assert_eq!(config.symbol_mappings.len(), 1);
        assert_eq!(config.symbol_mappings[0].source_symbol, "EURUSD");
        assert_eq!(config.filters.allowed_symbols.as_ref().unwrap().len(), 2);
        assert_eq!(
            config.filters.allowed_magic_numbers.as_ref().unwrap().len(),
            2
        );
    }

    #[test]
    fn test_config_message_serialization() {
        let config = ConfigMessage {
            account_id: "TEST_001".to_string(),
            master_account: "MASTER_001".to_string(),
            trade_group_id: "MASTER_001".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            enabled: true,
            lot_multiplier: Some(2.0),
            reverse_trade: false,
            symbol_mappings: vec![],
            filters: TradeFilters {
                allowed_symbols: None,
                blocked_symbols: None,
                allowed_magic_numbers: None,
                blocked_magic_numbers: None,
            },
            config_version: 1,
        };

        let msgpack = rmp_serde::to_vec_named(&config).unwrap();

        // Verify deserialization works
        let deserialized: ConfigMessage = rmp_serde::from_slice(&msgpack).unwrap();
        assert_eq!(deserialized.account_id, "TEST_001");
        assert_eq!(deserialized.enabled, true);
        assert_eq!(deserialized.config_version, 1);
    }

    #[test]
    fn test_config_message_with_null_values() {
        let settings = CopySettings {
            id: 3,
            enabled: true,
            master_account: "MASTER_003".to_string(),
            slave_account: "SLAVE_003".to_string(),
            lot_multiplier: None,
            reverse_trade: false,
            symbol_mappings: vec![],
            filters: TradeFilters {
                allowed_symbols: None,
                blocked_symbols: None,
                allowed_magic_numbers: None,
                blocked_magic_numbers: None,
            },
        };

        let config: ConfigMessage = settings.into();
        let msgpack = rmp_serde::to_vec_named(&config).unwrap();

        // Verify null handling
        assert_eq!(config.lot_multiplier, None);

        // Verify deserialization handles nulls
        let deserialized: ConfigMessage = rmp_serde::from_slice(&msgpack).unwrap();
        assert_eq!(deserialized.lot_multiplier, None);
    }
}
