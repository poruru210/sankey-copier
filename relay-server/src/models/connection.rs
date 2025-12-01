use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

// Re-export shared message types from DLL
pub use sankey_copier_zmq::{
    HeartbeatMessage, PositionSnapshotMessage, RequestConfigMessage, SyncRequestMessage,
    UnregisterMessage,
};

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
    pub is_trade_allowed: bool, // MT auto-trading enabled state
}

/// EAの種類
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum EaType {
    Master,
    Slave,
}

impl FromStr for EaType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Master" => Ok(EaType::Master),
            "Slave" => Ok(EaType::Slave),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for EaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EaType::Master => write!(f, "Master"),
            EaType::Slave => write!(f, "Slave"),
        }
    }
}

/// プラットフォームの種類
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Platform {
    MT4,
    MT5,
}

impl FromStr for Platform {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "MT4" => Ok(Platform::MT4),
            "MT5" => Ok(Platform::MT5),
            _ => Err(()),
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

// Re-export SlaveConfigMessage from DLL
pub use sankey_copier_zmq::SlaveConfigMessage;

impl EaConnection {
    /// Calculate if trade is effectively allowed based on both flag and connection status
    pub fn is_effective_trade_allowed(&self) -> bool {
        self.is_trade_allowed && self.status == ConnectionStatus::Online
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sankey_copier_zmq::TradeFilters;

    #[test]
    fn test_is_effective_trade_allowed() {
        let base_conn = EaConnection {
            account_id: "TEST".to_string(),
            ea_type: EaType::Master,
            platform: Platform::MT5,
            account_number: 123456,
            broker: "Broker".to_string(),
            account_name: "Name".to_string(),
            server: "Server".to_string(),
            balance: 1000.0,
            equity: 1000.0,
            currency: "USD".to_string(),
            leverage: 100,
            last_heartbeat: Utc::now(),
            status: ConnectionStatus::Online,
            connected_at: Utc::now(),
            is_trade_allowed: true,
        };

        // Case 1: Online + Allowed -> True
        let mut conn = base_conn.clone();
        conn.status = ConnectionStatus::Online;
        conn.is_trade_allowed = true;
        assert!(conn.is_effective_trade_allowed(), "Online + Allowed should be True");

        // Case 2: Online + Not Allowed -> False
        let mut conn = base_conn.clone();
        conn.status = ConnectionStatus::Online;
        conn.is_trade_allowed = false;
        assert!(!conn.is_effective_trade_allowed(), "Online + Not Allowed should be False");

        // Case 3: Timeout + Allowed -> False
        let mut conn = base_conn.clone();
        conn.status = ConnectionStatus::Timeout;
        conn.is_trade_allowed = true;
        assert!(!conn.is_effective_trade_allowed(), "Timeout + Allowed should be False");

        // Case 4: Timeout + Not Allowed -> False
        let mut conn = base_conn.clone();
        conn.status = ConnectionStatus::Timeout;
        conn.is_trade_allowed = false;
        assert!(!conn.is_effective_trade_allowed(), "Timeout + Not Allowed should be False");

        // Case 5: Offline + Allowed -> False
        let mut conn = base_conn.clone();
        conn.status = ConnectionStatus::Offline;
        conn.is_trade_allowed = true;
        assert!(!conn.is_effective_trade_allowed(), "Offline + Allowed should be False");

        // Case 6: Offline + Not Allowed -> False
        let mut conn = base_conn.clone();
        conn.status = ConnectionStatus::Offline;
        conn.is_trade_allowed = false;
        assert!(!conn.is_effective_trade_allowed(), "Offline + Not Allowed should be False");
    }

    #[test]
    fn test_config_message_serialization() {
        let config = SlaveConfigMessage {
            account_id: "TEST_001".to_string(),
            master_account: "MASTER_001".to_string(),
            timestamp: "2023-01-01T00:00:00Z".to_string(),
            trade_group_id: "MASTER_001".to_string(),
            status: 2, // STATUS_CONNECTED
            lot_calculation_mode: sankey_copier_zmq::LotCalculationMode::default(),
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
            symbol_prefix: None,
            symbol_suffix: None,
            source_lot_min: None,
            source_lot_max: None,
            master_equity: None,
            // Open Sync Policy defaults
            sync_mode: sankey_copier_zmq::SyncMode::default(),
            limit_order_expiry_min: None,
            market_sync_max_pips: None,
            max_slippage: None,
            copy_pending_orders: false,
            // Trade Execution defaults
            max_retries: 3,
            max_signal_delay_ms: 5000,
            use_pending_order_for_delayed: false,
            allow_new_orders: true,
        };

        let msgpack = rmp_serde::to_vec_named(&config).unwrap();

        // Verify deserialization works
        let deserialized: SlaveConfigMessage = rmp_serde::from_slice(&msgpack).unwrap();
        assert_eq!(deserialized.account_id, "TEST_001");
        assert_eq!(deserialized.status, 2);
        assert_eq!(deserialized.config_version, 1);
    }
}
