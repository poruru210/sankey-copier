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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigMessage {
    pub account_id: String,
    pub master_account: String,
    pub trade_group_id: String,
    #[serde(default = "chrono::Utc::now")]
    pub timestamp: DateTime<Utc>,
}
