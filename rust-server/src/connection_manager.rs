use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{Duration, Utc};

use crate::models::{EaConnection, ConnectionStatus, RegisterMessage, HeartbeatMessage};

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

    /// EAを登録
    pub async fn register_ea(&self, msg: RegisterMessage) {
        let account_id = msg.account_id.clone();
        let connection: EaConnection = msg.into();

        tracing::info!(
            "EA registered: {} ({:?}, {:?}) - {}@{}",
            account_id,
            connection.ea_type,
            connection.platform,
            connection.account_number,
            connection.broker
        );

        let mut connections = self.connections.write().await;
        connections.insert(account_id, connection);
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

    /// Heartbeatを更新
    pub async fn update_heartbeat(&self, msg: HeartbeatMessage) {
        let account_id = &msg.account_id;
        let mut connections = self.connections.write().await;

        if let Some(conn) = connections.get_mut(account_id) {
            conn.last_heartbeat = Utc::now();
            conn.balance = msg.balance;
            conn.equity = msg.equity;
            conn.status = ConnectionStatus::Online;

            tracing::debug!(
                "Heartbeat received: {} (Balance: {:.2} {}, Equity: {:.2})",
                account_id,
                conn.balance,
                conn.currency,
                conn.equity
            );
        } else {
            tracing::warn!("Heartbeat received from unregistered EA: {}", account_id);
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
    pub async fn check_timeouts(&self) {
        let now = Utc::now();
        let timeout_duration = Duration::seconds(self.timeout_seconds);

        let mut connections = self.connections.write().await;
        let mut timed_out_accounts = Vec::new();

        for (account_id, conn) in connections.iter_mut() {
            if conn.status == ConnectionStatus::Online {
                let elapsed = now.signed_duration_since(conn.last_heartbeat);

                if elapsed > timeout_duration {
                    tracing::warn!(
                        "EA timed out: {} (last heartbeat: {:?} ago)",
                        account_id,
                        elapsed
                    );
                    conn.status = ConnectionStatus::Timeout;
                    timed_out_accounts.push(account_id.clone());
                }
            }
        }

        if !timed_out_accounts.is_empty() {
            tracing::info!("Timed out EAs: {:?}", timed_out_accounts);
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{EaType, Platform};

    fn create_test_register_message(account_id: &str) -> RegisterMessage {
        use chrono::Utc;
        RegisterMessage {
            account_id: account_id.to_string(),
            account_number: 12345,
            broker: "Test Broker".to_string(),
            account_name: "Test Account".to_string(),
            server: "Test-Server".to_string(),
            currency: "USD".to_string(),
            balance: 10000.0,
            equity: 10000.0,
            leverage: 100,
            ea_type: EaType::Master,
            platform: Platform::MT4,
            timestamp: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_register_ea() {
        let manager = ConnectionManager::new(30);
        let msg = create_test_register_message("TEST_001");
        let account_id = msg.account_id.clone();

        manager.register_ea(msg).await;

        let ea = manager.get_ea(&account_id).await;
        assert!(ea.is_some());
        let ea = ea.unwrap();
        assert_eq!(ea.account_id, account_id);
        assert_eq!(ea.status, ConnectionStatus::Online);
    }

    #[tokio::test]
    async fn test_unregister_ea() {
        let manager = ConnectionManager::new(30);
        let msg = create_test_register_message("TEST_001");
        let account_id = msg.account_id.clone();

        manager.register_ea(msg).await;
        manager.unregister_ea(&account_id).await;

        let ea = manager.get_ea(&account_id).await;
        assert!(ea.is_some());
        assert_eq!(ea.unwrap().status, ConnectionStatus::Offline);
    }

    #[tokio::test]
    async fn test_update_heartbeat() {
        let manager = ConnectionManager::new(30);
        let msg = create_test_register_message("TEST_001");
        let account_id = msg.account_id.clone();

        manager.register_ea(msg).await;

        let hb_msg = HeartbeatMessage {
            account_id: account_id.clone(),
            balance: 12000.0,
            equity: 11500.0,
            open_positions: Some(3),
            timestamp: chrono::Utc::now(),
        };
        manager.update_heartbeat(hb_msg).await;

        let ea = manager.get_ea(&account_id).await;
        assert!(ea.is_some());
        let ea = ea.unwrap();
        assert_eq!(ea.balance, 12000.0);
        assert_eq!(ea.equity, 11500.0);
        assert_eq!(ea.status, ConnectionStatus::Online);
    }

    #[tokio::test]
    async fn test_get_all_eas() {
        let manager = ConnectionManager::new(30);

        manager.register_ea(create_test_register_message("TEST_001")).await;
        manager.register_ea(create_test_register_message("TEST_002")).await;

        let eas = manager.get_all_eas().await;
        assert_eq!(eas.len(), 2);
    }

    #[tokio::test]
    async fn test_timeout_check() {
        let manager = ConnectionManager::new(1); // 1 second timeout
        let msg = create_test_register_message("TEST_001");
        let account_id = msg.account_id.clone();

        manager.register_ea(msg).await;

        // Verify initially online
        let ea = manager.get_ea(&account_id).await;
        assert_eq!(ea.unwrap().status, ConnectionStatus::Online);

        // Wait for timeout
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Run timeout check
        manager.check_timeouts().await;

        // Verify timed out
        let ea = manager.get_ea(&account_id).await;
        assert_eq!(ea.unwrap().status, ConnectionStatus::Timeout);
    }

    #[tokio::test]
    async fn test_heartbeat_prevents_timeout() {
        let manager = ConnectionManager::new(2); // 2 second timeout
        let msg = create_test_register_message("TEST_001");
        let account_id = msg.account_id.clone();

        manager.register_ea(msg).await;

        // Send heartbeat after 1 second
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        manager.update_heartbeat(HeartbeatMessage {
            account_id: account_id.clone(),
            balance: 10000.0,
            equity: 10000.0,
            open_positions: Some(0),
            timestamp: chrono::Utc::now(),
        }).await;

        // Wait another second (total 2 seconds, but heartbeat was sent at 1 second)
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        // Run timeout check
        manager.check_timeouts().await;

        // Should still be online because heartbeat was sent within timeout
        let ea = manager.get_ea(&account_id).await;
        assert_eq!(ea.unwrap().status, ConnectionStatus::Online);
    }
}
