use async_trait::async_trait;
use chrono::{Duration, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::domain::models::{ConnectionStatus, EaConnection, EaType, HeartbeatMessage, Platform};

/// EA connection key: (account_id, ea_type)
/// Allows same account to have both Master and Slave EAs running simultaneously
type ConnectionKey = (String, EaType);

/// EA接続を管理するマネージャー
#[derive(Clone)]
pub struct ConnectionManager {
    connections: Arc<RwLock<HashMap<ConnectionKey, EaConnection>>>,
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

    /// RegisterMessageからEAを登録（is_trade_allowed=false初期値）
    ///
    /// Register専用のメソッド。Heartbeatによる自動登録とは異なり、
    /// is_trade_allowedは初期値falseで設定される。
    /// 最初のHeartbeatで正確なis_trade_allowed値に更新される。
    pub async fn register_ea(&self, msg: &crate::domain::models::RegisterMessage) {
        let ea_type: EaType = msg.ea_type.parse().unwrap_or(EaType::Master);
        let key = (msg.account_id.clone(), ea_type);

        let mut connections = self.connections.write().await;

        // 既に登録済みの場合は更新しない（Heartbeatに任せる）
        if connections.contains_key(&key) {
            tracing::debug!(
                "EA already registered: {} ({}), skipping register",
                msg.account_id,
                ea_type
            );
            return;
        }

        tracing::info!(
            "Registering EA from Register message: {} ({:?}, {:?}) - {}@{}",
            msg.account_id,
            msg.ea_type,
            msg.platform,
            msg.account_number,
            msg.broker
        );

        let now = Utc::now();
        let connection = EaConnection {
            account_id: msg.account_id.clone(),
            ea_type,
            platform: msg.platform.parse().unwrap_or(Platform::MT5),
            account_number: msg.account_number,
            broker: msg.broker.clone(),
            account_name: msg.account_name.clone(),
            server: msg.server.clone(),
            balance: 0.0, // 初期値、Heartbeatで更新
            equity: 0.0,  // 初期値、Heartbeatで更新
            currency: msg.currency.clone(),
            leverage: msg.leverage,
            last_heartbeat: now,
            status: ConnectionStatus::Registered,
            connected_at: now,
            is_trade_allowed: false, // 初期値、最初のHeartbeatで更新
        };

        connections.insert(key, connection);
    }

    /// EAの登録を解除 (特定のEA種別)
    pub async fn unregister_ea(&self, account_id: &str, ea_type: EaType) {
        tracing::info!("EA unregistered: {} ({})", account_id, ea_type);

        let key = (account_id.to_string(), ea_type);
        let mut connections = self.connections.write().await;
        if let Some(conn) = connections.get_mut(&key) {
            conn.status = ConnectionStatus::Offline;
        }
        // Note: オフライン状態で保持（完全削除はしない）
    }

    /// Heartbeatを更新（自動登録機能付き）
    /// Uses (account_id, ea_type) as composite key
    /// Returns true if this was a new registration (auto-registered), false otherwise
    pub async fn update_heartbeat(&self, msg: HeartbeatMessage) -> bool {
        let account_id = &msg.account_id;
        let ea_type: EaType = msg.ea_type.parse().unwrap_or(EaType::Master);
        let key = (account_id.clone(), ea_type);

        let mut connections = self.connections.write().await;

        if let Some(conn) = connections.get_mut(&key) {
            // 既存のEA: ハートビート情報を更新
            conn.last_heartbeat = Utc::now();
            conn.balance = msg.balance;
            conn.equity = msg.equity;
            conn.status = ConnectionStatus::Online;
            conn.is_trade_allowed = msg.is_trade_allowed;
            conn.platform = msg.platform.parse().unwrap_or(conn.platform);

            tracing::debug!(
                "Heartbeat received: {} ({}) (Balance: {:.2} {}, Equity: {:.2}, EA Version: {}, TradeAllowed: {})",
                account_id,
                conn.ea_type,
                conn.balance,
                conn.currency,
                conn.equity,
                msg.version,
                msg.is_trade_allowed
            );
            false
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
                ea_type,
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

            connections.insert(key, connection);
            true
        }
    }

    /// すべてのEA（オンライン・オフライン含む）を取得
    pub async fn get_all_eas(&self) -> Vec<EaConnection> {
        let connections = self.connections.read().await;
        connections.values().cloned().collect()
    }

    /// Master EAを取得
    pub async fn get_master(&self, account_id: &str) -> Option<EaConnection> {
        let connections = self.connections.read().await;
        connections
            .get(&(account_id.to_string(), EaType::Master))
            .cloned()
    }

    /// Slave EAを取得
    pub async fn get_slave(&self, account_id: &str) -> Option<EaConnection> {
        let connections = self.connections.read().await;
        connections
            .get(&(account_id.to_string(), EaType::Slave))
            .cloned()
    }

    /// account_idに紐づく全EAを取得
    pub async fn get_eas_by_account(&self, account_id: &str) -> Vec<EaConnection> {
        let connections = self.connections.read().await;
        connections
            .iter()
            .filter(|((acc_id, _), _)| acc_id == account_id)
            .map(|(_, conn)| conn.clone())
            .collect()
    }

    /// 特定のEAを取得（後方互換: Master優先）
    pub async fn get_ea(&self, account_id: &str) -> Option<EaConnection> {
        // Master優先、なければSlave
        if let Some(conn) = self.get_master(account_id).await {
            return Some(conn);
        }
        self.get_slave(account_id).await
    }

    /// タイムアウトをチェックして、応答のないEAをタイムアウト状態にする
    /// Returns a list of (account_id, ea_type) for timed-out EAs
    pub async fn check_timeouts(&self) -> Vec<(String, EaType)> {
        let now = Utc::now();
        let timeout_duration = Duration::seconds(self.timeout_seconds);

        let mut connections = self.connections.write().await;
        let mut timed_out_accounts = Vec::new();

        for ((account_id, ea_type), conn) in connections.iter_mut() {
            if conn.status == ConnectionStatus::Online
                || conn.status == ConnectionStatus::Registered
            {
                let elapsed = now.signed_duration_since(conn.last_heartbeat);

                if elapsed > timeout_duration {
                    tracing::warn!(
                        "EA timed out: {} (ea_type: {}, last heartbeat: {:?} ago)",
                        account_id,
                        ea_type,
                        elapsed
                    );
                    conn.status = ConnectionStatus::Timeout;
                    timed_out_accounts.push((account_id.clone(), *ea_type));
                }
            }
        }

        if !timed_out_accounts.is_empty() {
            tracing::info!(
                "Timed out EAs: {:?}",
                timed_out_accounts
                    .iter()
                    .map(|(id, t)| format!("{}:{}", id, t))
                    .collect::<Vec<_>>()
            );
        }

        timed_out_accounts
    }
}

// Adapter implementation for Outbound Port
#[async_trait]
impl crate::ports::ConnectionManager for ConnectionManager {
    async fn get_master(&self, account_id: &str) -> Option<EaConnection> {
        self.get_master(account_id).await
    }

    async fn get_slave(&self, account_id: &str) -> Option<EaConnection> {
        self.get_slave(account_id).await
    }

    async fn update_heartbeat(&self, msg: HeartbeatMessage) -> bool {
        self.update_heartbeat(msg).await
    }
}

// ============================================================================
// Monitor Implementation (formerly monitor.rs)
// ============================================================================

/// Trait to handle side effects of timeouts (DB updates, notifications)
#[async_trait]
pub trait TimeoutActionHandler: Send + Sync {
    async fn handle_master_timeout(&self, account_id: &str);
    async fn handle_slave_timeout(&self, account_id: &str);
}

use crate::ports::DisconnectionService;

/// Real implementation with DB and ZMQ dependencies
pub struct RealTimeoutActionHandler {
    disconnection_service: Arc<dyn DisconnectionService>,
}

impl RealTimeoutActionHandler {
    pub fn new(disconnection_service: Arc<dyn DisconnectionService>) -> Self {
        Self {
            disconnection_service,
        }
    }
}

#[async_trait]
impl TimeoutActionHandler for RealTimeoutActionHandler {
    async fn handle_master_timeout(&self, account_id: &str) {
        self.disconnection_service
            .handle_master_offline(account_id)
            .await;
    }

    async fn handle_slave_timeout(&self, account_id: &str) {
        self.disconnection_service
            .handle_slave_offline(account_id)
            .await;
    }
}

/// Monitor for EA connection timeouts
pub struct TimeoutMonitor {
    connection_manager: Arc<ConnectionManager>,
    action_handler: Arc<dyn TimeoutActionHandler>,
    check_interval: std::time::Duration,
}

impl TimeoutMonitor {
    pub fn new(
        connection_manager: Arc<ConnectionManager>,
        action_handler: Arc<dyn TimeoutActionHandler>,
    ) -> Self {
        Self {
            connection_manager,
            action_handler,
            check_interval: std::time::Duration::from_secs(10), // Default 10s interval
        }
    }

    /// Set a custom check interval (useful for tests)
    #[allow(dead_code)]
    pub fn with_check_interval(mut self, interval: std::time::Duration) -> Self {
        self.check_interval = interval;
        self
    }

    /// Start the monitoring loop
    pub async fn run(self) {
        let mut interval = tokio::time::interval(self.check_interval);

        loop {
            interval.tick().await;
            self.check_timeouts().await;
        }
    }

    /// Perform a single check for timeouts (public for testing)
    pub async fn check_timeouts(&self) {
        let timed_out = self.connection_manager.check_timeouts().await;

        // Update database statuses for timed-out EAs
        for (account_id, ea_type) in timed_out {
            match ea_type {
                EaType::Master => {
                    self.action_handler.handle_master_timeout(&account_id).await;
                }
                EaType::Slave => {
                    self.action_handler.handle_slave_timeout(&account_id).await;
                }
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    pub fn create_test_heartbeat_message(account_id: &str, ea_type: &str) -> HeartbeatMessage {
        HeartbeatMessage {
            message_type: "Heartbeat".to_string(),
            account_id: account_id.to_string(),
            balance: 10000.0,
            equity: 10000.0,
            open_positions: 0,
            timestamp: chrono::Utc::now().to_rfc3339(),
            version: "test".to_string(),
            ea_type: ea_type.to_string(),
            platform: "MT4".to_string(),
            account_number: 12345,
            broker: "Test Broker".to_string(),
            account_name: "Test Account".to_string(),
            server: "Test-Server".to_string(),
            currency: "USD".to_string(),
            leverage: 100,
            is_trade_allowed: true,
            symbol_prefix: None,
            symbol_suffix: None,
            symbol_map: None,
        }
    }

    #[tokio::test]
    async fn test_unregister_ea() {
        let manager = ConnectionManager::new(30);
        let msg = create_test_heartbeat_message("TEST_001", "Master");
        let account_id = msg.account_id.clone();

        // Auto-register via heartbeat
        manager.update_heartbeat(msg).await;

        // Verify registered
        let ea = manager.get_master(&account_id).await;
        assert!(ea.is_some());
        assert_eq!(ea.unwrap().status, ConnectionStatus::Online);

        // Unregister with ea_type
        manager.unregister_ea(&account_id, EaType::Master).await;

        let ea = manager.get_master(&account_id).await;
        assert!(ea.is_some());
        assert_eq!(ea.unwrap().status, ConnectionStatus::Offline);
    }

    #[tokio::test]
    async fn test_update_heartbeat() {
        let manager = ConnectionManager::new(30);
        let account_id = "TEST_001".to_string();

        // First heartbeat: auto-registers the EA
        let hb_msg = create_test_heartbeat_message(&account_id, "Master");
        manager.update_heartbeat(hb_msg).await;

        // Second heartbeat: updates balance and equity
        let mut hb_msg2 = create_test_heartbeat_message(&account_id, "Master");
        hb_msg2.balance = 12000.0;
        hb_msg2.equity = 11500.0;
        manager.update_heartbeat(hb_msg2).await;

        let ea = manager.get_master(&account_id).await;
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
        manager
            .update_heartbeat(create_test_heartbeat_message("TEST_001", "Master"))
            .await;
        manager
            .update_heartbeat(create_test_heartbeat_message("TEST_002", "Master"))
            .await;

        let eas = manager.get_all_eas().await;
        assert_eq!(eas.len(), 2);
    }

    #[tokio::test]
    async fn test_timeout_check() {
        let manager = ConnectionManager::new(1); // 1 second timeout
        let msg = create_test_heartbeat_message("TEST_001", "Master");
        let account_id = msg.account_id.clone();

        // Auto-register via heartbeat
        manager.update_heartbeat(msg).await;

        // Verify initially online
        let ea = manager.get_master(&account_id).await;
        assert_eq!(ea.unwrap().status, ConnectionStatus::Online);

        // Wait for timeout
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Run timeout check
        let timed_out = manager.check_timeouts().await;

        // Verify one EA timed out
        assert_eq!(timed_out.len(), 1);
        assert_eq!(timed_out[0].0, account_id);
        assert_eq!(timed_out[0].1, EaType::Master);

        // Verify timed out status
        let ea = manager.get_master(&account_id).await;
        assert_eq!(ea.unwrap().status, ConnectionStatus::Timeout);
    }

    #[tokio::test]
    async fn test_heartbeat_prevents_timeout() {
        let manager = ConnectionManager::new(2); // 2 second timeout
        let msg = create_test_heartbeat_message("TEST_001", "Master");
        let account_id = msg.account_id.clone();

        // Auto-register via heartbeat
        manager.update_heartbeat(msg).await;

        // Send heartbeat after 1 second
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        manager
            .update_heartbeat(create_test_heartbeat_message(&account_id, "Master"))
            .await;

        // Wait another second (total 2 seconds, but heartbeat was sent at 1 second)
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        // Run timeout check
        let timed_out = manager.check_timeouts().await;

        // Should not have timed out because heartbeat was sent within timeout
        assert_eq!(timed_out.len(), 0);

        // Should still be online
        let ea = manager.get_master(&account_id).await;
        assert_eq!(ea.unwrap().status, ConnectionStatus::Online);
    }

    #[tokio::test]
    async fn test_heartbeat_auto_registration() {
        let manager = ConnectionManager::new(30);

        // Send heartbeat without prior registration
        let mut hb_msg = create_test_heartbeat_message("TEST_NEW", "Slave");
        hb_msg.balance = 15000.0;
        hb_msg.equity = 15500.0;
        hb_msg.platform = "MT5".to_string();
        hb_msg.account_number = 67890;
        hb_msg.broker = "New Broker".to_string();
        hb_msg.currency = "EUR".to_string();
        hb_msg.leverage = 200;

        manager.update_heartbeat(hb_msg).await;

        // Verify EA was auto-registered as Slave
        let ea = manager.get_slave("TEST_NEW").await;
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

        // get_ea should NOT find Slave (Master優先)
        let ea_via_get_ea = manager.get_ea("TEST_NEW").await;
        // Since there's no Master, get_ea should fall back to Slave
        assert!(ea_via_get_ea.is_some());
        assert_eq!(ea_via_get_ea.unwrap().ea_type, EaType::Slave);
    }

    #[tokio::test]
    async fn test_same_account_master_and_slave() {
        let manager = ConnectionManager::new(30);
        let account_id = "DUAL_TEST";

        // Register Master EA
        manager
            .update_heartbeat(create_test_heartbeat_message(account_id, "Master"))
            .await;

        // Register Slave EA (same account_id)
        let mut slave_hb = create_test_heartbeat_message(account_id, "Slave");
        slave_hb.balance = 20000.0; // Different balance to distinguish
        manager.update_heartbeat(slave_hb).await;

        // Both should be registered
        let all_eas = manager.get_all_eas().await;
        assert_eq!(
            all_eas.len(),
            2,
            "Both Master and Slave should be registered"
        );

        // get_master returns Master
        let master = manager.get_master(account_id).await;
        assert!(master.is_some());
        assert_eq!(master.as_ref().unwrap().ea_type, EaType::Master);
        assert_eq!(master.as_ref().unwrap().balance, 10000.0);

        // get_slave returns Slave
        let slave = manager.get_slave(account_id).await;
        assert!(slave.is_some());
        assert_eq!(slave.as_ref().unwrap().ea_type, EaType::Slave);
        assert_eq!(slave.as_ref().unwrap().balance, 20000.0);

        // get_ea returns Master (後方互換: Master優先)
        let ea = manager.get_ea(account_id).await;
        assert!(ea.is_some());
        assert_eq!(ea.unwrap().ea_type, EaType::Master);

        // get_eas_by_account returns both
        let eas = manager.get_eas_by_account(account_id).await;
        assert_eq!(eas.len(), 2);
    }

    #[tokio::test]
    async fn test_unregister_one_ea_keeps_other() {
        let manager = ConnectionManager::new(30);
        let account_id = "DUAL_UNREG";

        // Register both Master and Slave
        manager
            .update_heartbeat(create_test_heartbeat_message(account_id, "Master"))
            .await;
        manager
            .update_heartbeat(create_test_heartbeat_message(account_id, "Slave"))
            .await;

        assert_eq!(manager.get_all_eas().await.len(), 2);

        // Unregister only Master
        manager.unregister_ea(account_id, EaType::Master).await;

        // Master should be Offline
        let master = manager.get_master(account_id).await;
        assert_eq!(master.unwrap().status, ConnectionStatus::Offline);

        // Slave should still be Online
        let slave = manager.get_slave(account_id).await;
        assert_eq!(slave.unwrap().status, ConnectionStatus::Online);
    }

    #[tokio::test]
    async fn test_register_ea_state_transition() {
        let manager = ConnectionManager::new(30);
        let account_id = "TRANSITION_TEST";

        // 1. Explicit Register
        let register_msg = crate::domain::models::RegisterMessage {
            message_type: "Register".to_string(),
            account_id: account_id.to_string(),
            ea_type: "Master".to_string(),
            platform: "MT5".to_string(),
            account_number: 12345,
            broker: "Test Broker".to_string(),
            account_name: "Test Account".to_string(),
            server: "Test-Server".to_string(),
            currency: "USD".to_string(),
            leverage: 100,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        manager.register_ea(&register_msg).await;

        // Verify status is Registered (NOT Online)
        let ea = manager.get_master(account_id).await;
        assert!(ea.is_some());
        let ea = ea.unwrap();
        assert_eq!(ea.status, ConnectionStatus::Registered);
        assert!(!ea.is_trade_allowed); // Default is false

        // 2. Heartbeat (Transition to Online)
        let hb_msg = create_test_heartbeat_message(account_id, "Master");
        manager.update_heartbeat(hb_msg).await;

        // Verify status is Online
        let ea = manager.get_master(account_id).await;
        assert!(ea.is_some());
        let ea = ea.unwrap();
        assert_eq!(ea.status, ConnectionStatus::Online);
        assert!(ea.is_trade_allowed); // Heartbeat updates this to true
    }

    // Mock handler to capture actions for Monitor tests
    struct MockTimeoutActionHandler {
        master_timeouts: Arc<Mutex<Vec<String>>>,
        slave_timeouts: Arc<Mutex<Vec<String>>>,
    }

    impl MockTimeoutActionHandler {
        fn new() -> Self {
            Self {
                master_timeouts: Arc::new(Mutex::new(Vec::new())),
                slave_timeouts: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    #[async_trait]
    impl TimeoutActionHandler for MockTimeoutActionHandler {
        async fn handle_master_timeout(&self, account_id: &str) {
            self.master_timeouts
                .lock()
                .unwrap()
                .push(account_id.to_string());
        }

        async fn handle_slave_timeout(&self, account_id: &str) {
            self.slave_timeouts
                .lock()
                .unwrap()
                .push(account_id.to_string());
        }
    }

    #[tokio::test]
    async fn test_monitor_detects_timeouts() {
        // Setup ConnectionManager with a short timeout
        let cm = Arc::new(ConnectionManager::new(1)); // 1 second timeout for EAs

        let msg = crate::domain::models::HeartbeatMessage {
            account_id: "master_1".to_string(),
            ea_type: "MASTER".to_string(),
            platform: "MT5".to_string(),
            version: "1.0.0".to_string(),
            symbol_prefix: None,
            symbol_suffix: None,
            message_type: "HEARTBEAT".to_string(),
            balance: 10000.0,
            equity: 10000.0,
            open_positions: 0,
            timestamp: "2023-01-01T00:00:00Z".to_string(),
            account_number: 123456,
            broker: "DemoBroker".to_string(),
            account_name: "DemoUser".to_string(),
            server: "DemoServer".to_string(),
            currency: "USD".to_string(),
            leverage: 500,
            is_trade_allowed: true,
            symbol_map: None,
        };
        cm.update_heartbeat(msg).await;

        // We need to wait > 1s for it to timeout
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

        let mock_handler = Arc::new(MockTimeoutActionHandler::new());
        let monitor = TimeoutMonitor::new(cm.clone(), mock_handler.clone());

        // Run check
        monitor.check_timeouts().await;

        // Verify Master timeout handler was called
        let timeouts = mock_handler.master_timeouts.lock().unwrap();
        assert_eq!(timeouts.len(), 1);
        assert_eq!(timeouts[0], "master_1");
    }
}
