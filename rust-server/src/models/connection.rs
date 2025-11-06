use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
    pub leverage: i32,
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

/// プラットフォームの種類
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Platform {
    MT4,
    MT5,
}

/// 接続状態
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ConnectionStatus {
    Online,
    Offline,
    Timeout,
}

/// メッセージタイプ
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "message_type")]
pub enum MessageType {
    Register(RegisterMessage),
    Unregister(UnregisterMessage),
    Heartbeat(HeartbeatMessage),
    TradeSignal(crate::models::TradeSignal),
}

/// EA登録メッセージ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterMessage {
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
    pub leverage: i32,
    #[serde(default = "chrono::Utc::now")]
    pub timestamp: DateTime<Utc>,
}

impl From<RegisterMessage> for EaConnection {
    fn from(msg: RegisterMessage) -> Self {
        let now = Utc::now();
        Self {
            account_id: msg.account_id,
            ea_type: msg.ea_type,
            platform: msg.platform,
            account_number: msg.account_number,
            broker: msg.broker,
            account_name: msg.account_name,
            server: msg.server,
            balance: msg.balance,
            equity: msg.equity,
            currency: msg.currency,
            leverage: msg.leverage,
            last_heartbeat: now,
            status: ConnectionStatus::Online,
            connected_at: now,
        }
    }
}

/// EA登録解除メッセージ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnregisterMessage {
    pub account_id: String,
    #[serde(default = "chrono::Utc::now")]
    pub timestamp: DateTime<Utc>,
}

/// Heartbeatメッセージ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatMessage {
    pub account_id: String,
    pub balance: f64,
    pub equity: f64,
    pub open_positions: Option<i32>,
    #[serde(default = "chrono::Utc::now")]
    pub timestamp: DateTime<Utc>,
}

/// 設定配信メッセージ
///
/// SlaveEAに完全な設定情報を配信するためのメッセージ。
/// CopySettingsの全フィールドを含み、EA側でフィルタリングと変換を実行可能にします。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigMessage {
    // 既存のフィールド
    pub account_id: String,
    pub master_account: String,
    pub trade_group_id: String,
    #[serde(default = "chrono::Utc::now")]
    pub timestamp: DateTime<Utc>,

    // 新規追加: 完全な設定情報
    /// コピーが有効かどうか
    pub enabled: bool,

    /// ロット倍率（nullの場合は1.0として扱う）
    pub lot_multiplier: Option<f64>,

    /// トレードを反転するか（Buy→Sell, Sell→Buy）
    pub reverse_trade: bool,

    /// シンボルマッピング（元シンボル → 変換後シンボル）
    pub symbol_mappings: Vec<crate::models::SymbolMapping>,

    /// トレードフィルター
    pub filters: crate::models::TradeFilters,

    /// 設定バージョン（将来の互換性のため）
    pub config_version: u32,
}

impl From<crate::models::CopySettings> for ConfigMessage {
    fn from(settings: crate::models::CopySettings) -> Self {
        Self {
            // 既存のフィールド
            account_id: settings.slave_account.clone(),
            master_account: settings.master_account.clone(),
            trade_group_id: settings.master_account, // master_accountと同じ
            timestamp: chrono::Utc::now(),

            // 新しいフィールド
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
            symbol_mappings: vec![
                SymbolMapping {
                    source_symbol: "EURUSD".to_string(),
                    target_symbol: "EURUSDm".to_string(),
                },
            ],
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
        assert_eq!(config.filters.allowed_magic_numbers.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_config_message_serialization() {
        let config = ConfigMessage {
            account_id: "TEST_001".to_string(),
            master_account: "MASTER_001".to_string(),
            trade_group_id: "MASTER_001".to_string(),
            timestamp: chrono::Utc::now(),
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

        let json = serde_json::to_string(&config).unwrap();

        // Verify JSON contains all key fields
        assert!(json.contains("\"account_id\""));
        assert!(json.contains("\"master_account\""));
        assert!(json.contains("\"enabled\""));
        assert!(json.contains("\"lot_multiplier\""));
        assert!(json.contains("\"reverse_trade\""));
        assert!(json.contains("\"symbol_mappings\""));
        assert!(json.contains("\"filters\""));
        assert!(json.contains("\"config_version\""));

        // Verify deserialization works
        let deserialized: ConfigMessage = serde_json::from_str(&json).unwrap();
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
        let json = serde_json::to_string(&config).unwrap();

        // Verify null handling
        assert_eq!(config.lot_multiplier, None);
        assert!(json.contains("\"lot_multiplier\":null"));

        // Verify deserialization handles nulls
        let deserialized: ConfigMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.lot_multiplier, None);
    }
}
