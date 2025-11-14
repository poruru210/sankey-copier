use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{Duration, Utc};

use crate::models::{EaConnection, ConnectionStatus, HeartbeatMessage, EaType, Platform};

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
            // 既存のEA: ハートビート情報のみ更新
            conn.last_heartbeat = Utc::now();
            conn.balance = msg.balance;
            conn.equity = msg.equity;
            conn.status = ConnectionStatus::Online;

            tracing::debug!(
                "Heartbeat received: {} (Balance: {:.2} {}, Equity: {:.2}, EA Version: {})",
                account_id,
                conn.balance,
                conn.currency,
                conn.equity,
                msg.version
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
                ea_type: EaType::from_str(&msg.ea_type).unwrap_or(EaType::Master),
                platform: Platform::from_str(&msg.platform).unwrap_or(Platform::MT5),
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

    fn create_test_heartbeat_message(account_id: &str) -> HeartbeatMessage {
        HeartbeatMessage {
            message_type: "Heartbeat".to_string(),
            account_id: account_id.to_string(),
            balance: 10000.0,
            equity: 10000.0,
            open_positions: 0,
            timestamp: chrono::Utc::now().to_rfc3339(),
            version: "test".to_string(),
            ea_type: "Master".to_string(),
            platform: "MT4".to_string(),
            account_number: 12345,
            broker: "Test Broker".to_string(),
            account_name: "Test Account".to_string(),
            server: "Test-Server".to_string(),
            currency: "USD".to_string(),
            leverage: 100,
        }
    }

    #[tokio::test]
    async fn test_unregister_ea() {
        let manager = ConnectionManager::new(30);
        let msg = create_test_heartbeat_message("TEST_001");
        let account_id = msg.account_id.clone();

        // Auto-register via heartbeat
        manager.update_heartbeat(msg).await;

        // Verify registered
        let ea = manager.get_ea(&account_id).await;
        assert!(ea.is_some());
        assert_eq!(ea.unwrap().status, ConnectionStatus::Online);

        // Unregister
        manager.unregister_ea(&account_id).await;

        let ea = manager.get_ea(&account_id).await;
        assert!(ea.is_some());
        assert_eq!(ea.unwrap().status, ConnectionStatus::Offline);
    }

    #[tokio::test]
    async fn test_update_heartbeat() {
        let manager = ConnectionManager::new(30);
        let account_id = "TEST_001".to_string();

        // First heartbeat: auto-registers the EA
        let hb_msg = HeartbeatMessage {
            message_type: "Heartbeat".to_string(),
            account_id: account_id.clone(),
            balance: 10000.0,
            equity: 10000.0,
            open_positions: 0,
            timestamp: chrono::Utc::now().to_rfc3339(),
            version: "test".to_string(),
            ea_type: "Master".to_string(),
            platform: "MT4".to_string(),
            account_number: 12345,
            broker: "Test Broker".to_string(),
            account_name: "Test Account".to_string(),
            server: "Test-Server".to_string(),
            currency: "USD".to_string(),
            leverage: 100,
        };
        manager.update_heartbeat(hb_msg).await;

        // Second heartbeat: updates balance and equity
        let hb_msg2 = HeartbeatMessage {
            message_type: "Heartbeat".to_string(),
            account_id: account_id.clone(),
            balance: 12000.0,
            equity: 11500.0,
            open_positions: 3,
            timestamp: chrono::Utc::now().to_rfc3339(),
            version: "test".to_string(),
            ea_type: "Master".to_string(),
            platform: "MT4".to_string(),
            account_number: 12345,
            broker: "Test Broker".to_string(),
            account_name: "Test Account".to_string(),
            server: "Test-Server".to_string(),
            currency: "USD".to_string(),
            leverage: 100,
        };
        manager.update_heartbeat(hb_msg2).await;

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

        // Auto-register two EAs via heartbeat
        manager.update_heartbeat(create_test_heartbeat_message("TEST_001")).await;
        manager.update_heartbeat(create_test_heartbeat_message("TEST_002")).await;

        let eas = manager.get_all_eas().await;
        assert_eq!(eas.len(), 2);
    }

    #[tokio::test]
    async fn test_timeout_check() {
        let manager = ConnectionManager::new(1); // 1 second timeout
        let msg = create_test_heartbeat_message("TEST_001");
        let account_id = msg.account_id.clone();

        // Auto-register via heartbeat
        manager.update_heartbeat(msg).await;

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
        let msg = create_test_heartbeat_message("TEST_001");
        let account_id = msg.account_id.clone();

        // Auto-register via heartbeat
        manager.update_heartbeat(msg).await;

        // Send heartbeat after 1 second
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        manager.update_heartbeat(HeartbeatMessage {
            message_type: "Heartbeat".to_string(),
            account_id: account_id.clone(),
            balance: 10000.0,
            equity: 10000.0,
            open_positions: 0,
            timestamp: chrono::Utc::now().to_rfc3339(),
            version: "test".to_string(),
            ea_type: "Master".to_string(),
            platform: "MT4".to_string(),
            account_number: 12345,
            broker: "Test Broker".to_string(),
            account_name: "Test Account".to_string(),
            server: "Test-Server".to_string(),
            currency: "USD".to_string(),
            leverage: 100,
        }).await;

        // Wait another second (total 2 seconds, but heartbeat was sent at 1 second)
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        // Run timeout check
        manager.check_timeouts().await;

        // Should still be online because heartbeat was sent within timeout
        let ea = manager.get_ea(&account_id).await;
        assert_eq!(ea.unwrap().status, ConnectionStatus::Online);
    }

    #[tokio::test]
    async fn test_heartbeat_auto_registration() {
        let manager = ConnectionManager::new(30);

        // Send heartbeat without prior registration
        let hb_msg = HeartbeatMessage {
            message_type: "Heartbeat".to_string(),
            account_id: "TEST_NEW".to_string(),
            balance: 15000.0,
            equity: 15500.0,
            open_positions: 2,
            timestamp: chrono::Utc::now().to_rfc3339(),
            version: "test123".to_string(),
            ea_type: "Slave".to_string(),
            platform: "MT5".to_string(),
            account_number: 67890,
            broker: "New Broker".to_string(),
            account_name: "New Account".to_string(),
            server: "NewServer-Live".to_string(),
            currency: "EUR".to_string(),
            leverage: 200,
        };

        manager.update_heartbeat(hb_msg).await;

        // Verify EA was auto-registered
        let ea = manager.get_ea("TEST_NEW").await;
        assert!(ea.is_some(), "EA should be auto-registered from heartbeat");

        let ea = ea.unwrap();
        assert_eq!(ea.account_id, "TEST_NEW");
        assert_eq!(ea.ea_type, EaType::Slave);
        assert_eq!(ea.platform, Platform::MT5);
        assert_eq!(ea.account_number, 67890);
        assert_eq!(ea.broker, "New Broker");
        assert_eq!(ea.balance, 15000.0);
        assert_eq!(ea.equity, 15500.0);
        assert_eq!(ea.currency, "EUR");
        assert_eq!(ea.leverage, 200);
        assert_eq!(ea.status, ConnectionStatus::Online);
    }
}
