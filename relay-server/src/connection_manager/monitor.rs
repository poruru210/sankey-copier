use crate::connection_manager::ConnectionManager;
use crate::db::Database;
use crate::message_handler::unregister::{notify_slave_offline, notify_slaves_master_offline};
use crate::models::EaType;
use crate::runtime_status_updater::RuntimeStatusMetrics;
use crate::zeromq::ZmqConfigPublisher;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

/// Monitor for EA connection timeouts
pub struct TimeoutMonitor {
    connection_manager: Arc<ConnectionManager>,
    db: Arc<Database>,
    publisher: Arc<ZmqConfigPublisher>,
    broadcast_tx: broadcast::Sender<String>,
    metrics: Arc<RuntimeStatusMetrics>,
    check_interval: Duration,
}

impl TimeoutMonitor {
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
                    self.handle_master_timeout(&account_id).await;
                }
                EaType::Slave => {
                    self.handle_slave_timeout(&account_id).await;
                }
            }
        }
    }

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
        // Slave timed out - update runtime status and notify WebSocket
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

#[cfg(test)]
mod tests {
    // TODO: Add unit tests with mocks
}
