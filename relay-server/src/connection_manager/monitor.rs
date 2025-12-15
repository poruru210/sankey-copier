use crate::connection_manager::ConnectionManager;
use crate::adapters::outbound::persistence::Database;
use crate::adapters::inbound::zmq::unregister::{notify_slave_offline, notify_slaves_master_offline};
use crate::models::EaType;
use crate::runtime_status_updater::RuntimeStatusMetrics;
use crate::adapters::outbound::messaging::ZmqConfigPublisher;
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

/// Trait to handle side effects of timeouts (DB updates, notifications)
#[async_trait]
pub trait TimeoutActionHandler: Send + Sync {
    async fn handle_master_timeout(&self, account_id: &str);
    async fn handle_slave_timeout(&self, account_id: &str);
}

/// Real implementation with DB and ZMQ dependencies
pub struct RealTimeoutActionHandler {
    connection_manager: Arc<ConnectionManager>,
    db: Arc<Database>,
    publisher: Arc<ZmqConfigPublisher>,
    broadcast_tx: broadcast::Sender<String>,
    metrics: Arc<RuntimeStatusMetrics>,
}

impl RealTimeoutActionHandler {
    pub fn new(
        connection_manager: Arc<ConnectionManager>,
        db: Arc<Database>,
        publisher: Arc<ZmqConfigPublisher>,
        broadcast_tx: broadcast::Sender<String>,
        metrics: Arc<RuntimeStatusMetrics>,
    ) -> Self {
        Self {
            connection_manager,
            db,
            publisher,
            broadcast_tx,
            metrics,
        }
    }
}

#[async_trait]
impl TimeoutActionHandler for RealTimeoutActionHandler {
    async fn handle_master_timeout(&self, account_id: &str) {
        match self.db.update_master_statuses_enabled(account_id).await {
            Ok(count) if count > 0 => {
                tracing::info!(
                    "Master {} timed out: updated {} settings to ENABLED",
                    account_id,
                    count
                );
            }
            Ok(_) => {
                // No settings updated
            }
            Err(e) => {
                tracing::error!("Failed to update master statuses for {}: {}", account_id, e);
            }
        }

        notify_slaves_master_offline(
            &self.connection_manager,
            &self.db,
            &self.publisher,
            &self.broadcast_tx,
            self.metrics.clone(),
            account_id,
        )
        .await;
    }

    async fn handle_slave_timeout(&self, account_id: &str) {
        notify_slave_offline(
            &self.connection_manager,
            &self.db,
            &self.broadcast_tx,
            self.metrics.clone(),
            account_id,
        )
        .await;
    }
}

/// Monitor for EA connection timeouts
pub struct TimeoutMonitor {
    connection_manager: Arc<ConnectionManager>,
    action_handler: Arc<dyn TimeoutActionHandler>,
    check_interval: Duration,
}

impl TimeoutMonitor {
    pub fn new(
        connection_manager: Arc<ConnectionManager>,
        action_handler: Arc<dyn TimeoutActionHandler>,
    ) -> Self {
        Self {
            connection_manager,
            action_handler,
            check_interval: Duration::from_secs(10), // Default 10s interval
        }
    }

    /// Set a custom check interval (useful for tests)
    #[allow(dead_code)]
    pub fn with_check_interval(mut self, interval: Duration) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;
    // use crate::models::{ConnectionStatus, EaConnection, Platform}; // Removed unused imports
    use chrono::Utc;
    use std::sync::Mutex;

    // Mock handler to capture actions
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

        // Register a Master EA that is "old"
        let _old_time = Utc::now() - chrono::Duration::seconds(5);

        // Inject directly into CM (using some internal knowledge or helper if available,
        // but robustly we rely on check_timeouts logic which we can't easily inject into
        // without mod visibility. Assuming check_timeouts works, we test the integration.)
        // Since we can't easily inject into CM without exposing internals, let's use the public API
        // register_ea / update_heartbeat but lie about the time? No, update_heartbeat uses Utc::now().
        //
        // Workaround: We will rely on ConnectionManager::check_timeouts returning values.
        // But optimizing this test requires CM to allow injecting "now".
        // Instead, let's verify that IF check_timeouts returns something, the handler is called.
        // We can't easily force CM to timeout without waiting.
        //
        // ALTERNATIVE: Use a Real ConnectionManager helper to inject state?
        // Let's assume for this unit test we mostly want to verify the wiring.
        // We can't easily mock ConnectionManager itself because it's a struct, not a trait.
        // However, we can use the fact that `check_timeouts` is what we call.
        //
        // Actually, we can just verify the logic of `check_timeouts` inside `TimeoutMonitor`.
        // But `TimeoutMonitor` calls `self.connection_manager.check_timeouts`.
        // So we need CM to return something.
        //
        // Let's try to add a helper to CM for testing or just accept we need to wait 2 seconds.

        let msg = crate::models::HeartbeatMessage {
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
        tokio::time::sleep(Duration::from_millis(1100)).await;

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
