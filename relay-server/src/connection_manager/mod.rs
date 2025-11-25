use chrono::{Duration, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::models::{ConnectionStatus, EaConnection, EaType, HeartbeatMessage, Platform};

/// EA接続を管理するマネージャー
#[derive(Clone)]
pub struct ConnectionManager {
    connections: Arc<RwLock<HashMap<String, EaConnection>>>,
    timeout_seconds: i64,
}

impl ConnectionManager {
    /// 新しいConnectionManagerを作成
    pub fn new(timeout_seconds: i64) -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            timeout_seconds,
        }
    }

    /// EAの登録を解除
    pub async fn unregister_ea(&self, account_id: &str) {
        tracing::info!("EA unregistered: {}", account_id);

        let mut connections = self.connections.write().await;
        if let Some(conn) = connections.get_mut(account_id) {
            conn.status = ConnectionStatus::Offline;
        }
        // Note: オフライン状態で保持（完全削除はしない）
        // 完全削除する場合: connections.remove(account_id);
    }

    /// Heartbeatを更新（自動登録機能付き）
    pub async fn update_heartbeat(&self, msg: HeartbeatMessage) {
        let account_id = &msg.account_id;
        let mut connections = self.connections.write().await;

        if let Some(conn) = connections.get_mut(account_id) {
            // 既存のEA: ハートビート情報を更新
            // IMPORTANT: EA type can change (e.g., Master -> Slave if reconfigured)
            conn.last_heartbeat = Utc::now();
            conn.balance = msg.balance;
            conn.equity = msg.equity;
            conn.status = ConnectionStatus::Online;
            conn.is_trade_allowed = msg.is_trade_allowed;
            conn.ea_type = msg.ea_type.parse().unwrap_or(conn.ea_type); // Update EA type
            conn.platform = msg.platform.parse().unwrap_or(conn.platform); // Update platform

            tracing::debug!(
                "Heartbeat received: {} (EA Type: {}, Balance: {:.2} {}, Equity: {:.2}, EA Version: {}, TradeAllowed: {})",
                account_id,
                conn.ea_type,
                conn.balance,
                conn.currency,
                conn.equity,
                msg.version,
                msg.is_trade_allowed
            );
        } else {
            // 未登録のEA: Heartbeatの情報から自動登録
            tracing::info!(
                "Auto-registering EA from heartbeat: {} ({:?}, {:?}) - {}@{}",
                account_id,
                msg.ea_type,
                msg.platform,
                msg.account_number,
                msg.broker
            );

            let now = Utc::now();
            let connection = EaConnection {
                account_id: msg.account_id.clone(),
                ea_type: msg.ea_type.parse().unwrap_or(EaType::Master),
                platform: msg.platform.parse().unwrap_or(Platform::MT5),
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
                is_trade_allowed: msg.is_trade_allowed,
            };

            connections.insert(account_id.clone(), connection);
        }
    }

    /// すべてのEA（オンライン・オフライン含む）を取得
    pub async fn get_all_eas(&self) -> Vec<EaConnection> {
        let connections = self.connections.read().await;
        connections.values().cloned().collect()
    }

    /// 特定のEAを取得
    pub async fn get_ea(&self, account_id: &str) -> Option<EaConnection> {
        let connections = self.connections.read().await;
        connections.get(account_id).cloned()
    }

    /// タイムアウトをチェックして、応答のないEAをタイムアウト状態にする
    /// Returns a list of (account_id, ea_type) for timed-out EAs
    pub async fn check_timeouts(&self) -> Vec<(String, EaType)> {
        let now = Utc::now();
        let timeout_duration = Duration::seconds(self.timeout_seconds);

        let mut connections = self.connections.write().await;
        let mut timed_out_accounts = Vec::new();

        for (account_id, conn) in connections.iter_mut() {
            if conn.status == ConnectionStatus::Online {
                let elapsed = now.signed_duration_since(conn.last_heartbeat);

                if elapsed > timeout_duration {
                    tracing::warn!(
                        "EA timed out: {} (ea_type: {}, last heartbeat: {:?} ago)",
                        account_id,
                        conn.ea_type,
                        elapsed
                    );
                    conn.status = ConnectionStatus::Timeout;
                    timed_out_accounts.push((account_id.clone(), conn.ea_type));
                }
            }
        }

        if !timed_out_accounts.is_empty() {
            tracing::info!(
                "Timed out EAs: {:?}",
                timed_out_accounts
                    .iter()
                    .map(|(id, _)| id)
                    .collect::<Vec<_>>()
            );
        }

        timed_out_accounts
    }
}

#[cfg(test)]
mod tests;
