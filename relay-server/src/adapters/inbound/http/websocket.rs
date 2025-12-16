//! WebSocket handler for real-time updates
//!
//! Provides WebSocket endpoint for broadcasting real-time updates
//! to connected clients. Implements on-demand snapshot broadcasting
//! that only runs when there are active WebSocket subscribers.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{ws::WebSocket, ws::WebSocketUpgrade, State},
    response::Response,
};
use tokio::sync::{broadcast, Mutex};
use tokio::task::JoinHandle;

use crate::adapters::inbound::http::AppState;
use crate::adapters::infrastructure::connection_manager::ConnectionManager;
use crate::adapters::outbound::persistence::Database;
use crate::domain::models::SystemStateSnapshot;
use crate::domain::services::status_calculator::{
    evaluate_master_status, evaluate_member_status, ConnectionSnapshot, MasterIntent, SlaveIntent,
};

/// Interval for snapshot broadcasts (in seconds)
const SNAPSHOT_INTERVAL_SECS: u64 = 3;

/// On-demand snapshot broadcaster that manages subscriber count and timer task.
///
/// When the first WebSocket client connects, the snapshot timer starts.
/// When the last WebSocket client disconnects, the timer stops.
/// This ensures zero resource usage when no UI clients are connected.
#[derive(Clone)]
pub struct SnapshotBroadcaster {
    /// Number of active WebSocket subscribers
    subscriber_count: Arc<AtomicUsize>,
    /// Handle to the background timer task (None when no subscribers)
    task_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    /// Broadcast channel for sending snapshots
    tx: broadcast::Sender<String>,
    /// Connection manager for fetching EA data
    connection_manager: Arc<ConnectionManager>,
    /// Database for fetching config
    db: Arc<Database>,
}

impl SnapshotBroadcaster {
    /// Create a new SnapshotBroadcaster
    pub fn new(
        tx: broadcast::Sender<String>,
        connection_manager: Arc<ConnectionManager>,
        db: Arc<Database>,
    ) -> Self {
        Self {
            subscriber_count: Arc::new(AtomicUsize::new(0)),
            task_handle: Arc::new(Mutex::new(None)),
            tx,
            connection_manager,
            db,
        }
    }

    /// Called when a WebSocket client connects
    pub async fn on_connect(&self) {
        let prev_count = self.subscriber_count.fetch_add(1, Ordering::SeqCst);

        if prev_count == 0 {
            // First subscriber: start the timer task
            tracing::info!("First WebSocket subscriber connected, starting snapshot timer");

            // Clone dependencies for the task
            let broadcaster = self.clone();

            let handle = tokio::spawn(async move {
                let mut interval =
                    tokio::time::interval(Duration::from_secs(SNAPSHOT_INTERVAL_SECS));
                loop {
                    interval.tick().await;
                    broadcaster.build_and_broadcast_snapshot().await;
                }
            });

            let mut task_handle = self.task_handle.lock().await;
            *task_handle = Some(handle);
        } else {
            tracing::debug!("WebSocket subscriber connected, total: {}", prev_count + 1);
        }
    }

    /// Called when a WebSocket client disconnects
    pub async fn on_disconnect(&self) {
        let prev_count = self.subscriber_count.fetch_sub(1, Ordering::SeqCst);

        if prev_count == 1 {
            // Last subscriber: stop the timer task
            tracing::info!("Last WebSocket subscriber disconnected, stopping snapshot timer");

            let mut task_handle = self.task_handle.lock().await;
            if let Some(handle) = task_handle.take() {
                handle.abort();
            }
        } else {
            tracing::debug!(
                "WebSocket subscriber disconnected, remaining: {}",
                prev_count - 1
            );
        }
    }

    /// Get current subscriber count (for monitoring/debugging)
    #[allow(dead_code)]
    pub fn subscriber_count(&self) -> usize {
        self.subscriber_count.load(Ordering::SeqCst)
    }

    /// Trigger an immediate snapshot broadcast (e.g., after toggle)
    pub async fn broadcast_now(&self) {
        // Only broadcast if there are subscribers to avoid wasted work
        if self.subscriber_count.load(Ordering::SeqCst) > 0 {
            self.build_and_broadcast_snapshot().await;
        }
    }

    /// Build the full system snapshot (with runtime status) and broadcast it
    async fn build_and_broadcast_snapshot(&self) {
        // 1. Fetch raw data from in-memory and DB
        let connections = self.connection_manager.get_all_eas().await;

        let trade_groups_result = self.db.list_trade_groups().await;
        let members_result = self.db.get_all_members().await;

        if let (Ok(mut trade_groups), Ok(mut members)) = (trade_groups_result, members_result) {
            // 2. Evaluate runtime status for TradeGroups (Masters)
            // Use HashMap for O(1) lookups during member evaluation
            use std::collections::HashMap;
            let mut master_results = HashMap::new();

            for tg in &mut trade_groups {
                // Get Master connection status from connections list
                let master_conn = connections.iter().find(|c| c.account_id == tg.id);
                let snapshot = ConnectionSnapshot {
                    connection_status: master_conn.map(|c| c.status),
                    is_trade_allowed: master_conn.map(|c| c.is_trade_allowed).unwrap_or(false),
                };

                let result = evaluate_master_status(
                    MasterIntent {
                        web_ui_enabled: tg.master_settings.enabled,
                    },
                    snapshot,
                );

                // Populate runtime fields on TradeGroup
                tg.master_runtime_status = result.status;
                tg.master_warning_codes = result.warning_codes.clone();

                master_results.insert(tg.id.clone(), result);
            }

            // 3. Evaluate runtime status for Members (Slaves)
            for member in &mut members {
                // Get Slave connection status
                let slave_conn = connections
                    .iter()
                    .find(|c| c.account_id == member.slave_account);
                let slave_snapshot = ConnectionSnapshot {
                    connection_status: slave_conn.map(|c| c.status),
                    is_trade_allowed: slave_conn.map(|c| c.is_trade_allowed).unwrap_or(false),
                };

                // Get result of the specific Master this member is connected to
                let master_result = master_results
                    .get(&member.trade_group_id)
                    .cloned()
                    .unwrap_or_default();

                let result = evaluate_member_status(
                    SlaveIntent {
                        web_ui_enabled: member.enabled_flag,
                    },
                    slave_snapshot,
                    &master_result,
                );

                // Populate runtime fields on Member
                member.status = result.status;
                member.warning_codes = result.warning_codes;
            }

            // 4. Construct Snapshot
            let snapshot = SystemStateSnapshot {
                connections, // The original connections list (status is sufficient here)
                trade_groups,
                members,
            };

            // 5. Serialize and Broadcast
            match serde_json::to_string(&snapshot) {
                Ok(json) => {
                    let message = format!("system_snapshot:{}", json);
                    if self.tx.send(message).is_err() {
                        tracing::warn!("No WebSocket receivers for system snapshot");
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to serialize system snapshot: {}", e);
                }
            }
        } else {
            tracing::error!("Failed to fetch data for system snapshot");
        }
    }
}

// Adapter implementation for Outbound Port
use async_trait::async_trait;

#[async_trait]
impl crate::ports::UpdateBroadcaster for SnapshotBroadcaster {
    async fn broadcast_snapshot(&self) {
        self.broadcast_now().await;
    }

    async fn broadcast_ea_disconnected(&self, account_id: &str) {
        let msg = format!("ea_disconnected:{}", account_id);
        let _ = self.tx.send(msg);
    }

    async fn broadcast_settings_updated(&self, json: &str) {
        let msg = format!("settings_updated:{}", json);
        let _ = self.tx.send(msg);
    }
}

/// WebSocket upgrade handler
pub async fn websocket_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(|socket| handle_websocket(socket, state))
}

/// Handle WebSocket connection
async fn handle_websocket(mut socket: WebSocket, state: AppState) {
    // Register subscriber
    state.snapshot_broadcaster.on_connect().await;

    // Subscribe to broadcast channel
    let mut rx = state.tx.subscribe();

    // Relay messages to WebSocket client
    while let Ok(msg) = rx.recv().await {
        if socket
            .send(axum::extract::ws::Message::Text(msg))
            .await
            .is_err()
        {
            break;
        }
    }

    // Unregister subscriber on disconnect
    state.snapshot_broadcaster.on_disconnect().await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::infrastructure::connection_manager::ConnectionManager;
    use crate::adapters::outbound::persistence::Database;
    use crate::domain::models::HeartbeatMessage;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::broadcast;

    /// Create a test HeartbeatMessage
    fn create_test_heartbeat(account_id: &str, ea_type: &str) -> HeartbeatMessage {
        HeartbeatMessage {
            message_type: "Heartbeat".to_string(),
            account_id: account_id.to_string(),
            balance: 10000.0,
            equity: 10000.0,
            open_positions: 0,
            timestamp: chrono::Utc::now().to_rfc3339(),
            version: "test".to_string(),
            ea_type: ea_type.to_string(),
            platform: "MT5".to_string(),
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

    /// Test that subscriber count starts at zero
    #[tokio::test]
    async fn test_snapshot_broadcaster_initial_state() {
        let (tx, _rx) = broadcast::channel(100);
        let connection_manager = Arc::new(ConnectionManager::new(30));
        let db = Arc::new(Database::new("sqlite::memory:").await.unwrap());
        let broadcaster = SnapshotBroadcaster::new(tx, connection_manager, db);

        assert_eq!(broadcaster.subscriber_count(), 0);
    }

    /// Test that on_connect increments subscriber count
    #[tokio::test]
    async fn test_snapshot_broadcaster_on_connect() {
        let (tx, _rx) = broadcast::channel(100);
        let connection_manager = Arc::new(ConnectionManager::new(30));
        let db = Arc::new(Database::new("sqlite::memory:").await.unwrap());
        let broadcaster = SnapshotBroadcaster::new(tx, connection_manager, db);

        broadcaster.on_connect().await;
        assert_eq!(broadcaster.subscriber_count(), 1);

        broadcaster.on_connect().await;
        assert_eq!(broadcaster.subscriber_count(), 2);
    }

    /// Test that on_disconnect decrements subscriber count
    #[tokio::test]
    async fn test_snapshot_broadcaster_on_disconnect() {
        let (tx, _rx) = broadcast::channel(100);
        let connection_manager = Arc::new(ConnectionManager::new(30));
        let db = Arc::new(Database::new("sqlite::memory:").await.unwrap());
        let broadcaster = SnapshotBroadcaster::new(tx, connection_manager, db);

        broadcaster.on_connect().await;
        broadcaster.on_connect().await;
        assert_eq!(broadcaster.subscriber_count(), 2);

        broadcaster.on_disconnect().await;
        assert_eq!(broadcaster.subscriber_count(), 1);

        broadcaster.on_disconnect().await;
        assert_eq!(broadcaster.subscriber_count(), 0);
    }

    /// Test that timer task starts on first subscriber and broadcasts snapshots
    #[tokio::test]
    async fn test_snapshot_broadcaster_sends_snapshot() {
        let (tx, mut rx) = broadcast::channel(100);
        let connection_manager = Arc::new(ConnectionManager::new(30));
        let db = Arc::new(Database::new("sqlite::memory:").await.unwrap());
        let broadcaster = SnapshotBroadcaster::new(tx, connection_manager.clone(), db);

        // Add a test connection to the manager
        let heartbeat = create_test_heartbeat("TEST_123", "Master");
        connection_manager.update_heartbeat(heartbeat).await;

        // Connect a subscriber (starts the timer)
        broadcaster.on_connect().await;

        // Wait for the first snapshot (should come within 3 seconds + some buffer)
        let result = tokio::time::timeout(Duration::from_secs(5), rx.recv()).await;

        // Disconnect to stop the timer
        broadcaster.on_disconnect().await;

        // Verify we received a snapshot
        assert!(result.is_ok(), "Should receive a snapshot within timeout");
        let message = result.unwrap().unwrap();
        // Changed from connections_snapshot to system_snapshot
        assert!(
            message.starts_with("system_snapshot:"),
            "Message should be a system snapshot, got: {}",
            &message[..message.len().min(50)]
        );

        // Parse the snapshot and verify content
        let json_part = message.strip_prefix("system_snapshot:").unwrap();
        let snapshot: serde_json::Value = serde_json::from_str(json_part).unwrap();
        let connections = snapshot["connections"].as_array().unwrap();
        assert_eq!(connections.len(), 1);
        assert_eq!(connections[0]["account_id"], "TEST_123");
    }

    /// Test that timer task stops when last subscriber disconnects
    #[tokio::test]
    async fn test_snapshot_broadcaster_timer_stops() {
        let (tx, mut rx) = broadcast::channel(100);
        let connection_manager = Arc::new(ConnectionManager::new(30));
        let db = Arc::new(Database::new("sqlite::memory:").await.unwrap());
        let broadcaster = SnapshotBroadcaster::new(tx, connection_manager, db);

        // Connect and immediately disconnect
        broadcaster.on_connect().await;
        broadcaster.on_disconnect().await;

        // Wait a bit for the timer to potentially send something
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Drain any pending messages
        while rx.try_recv().is_ok() {}

        // Wait another interval - should NOT receive any more snapshots
        let result = tokio::time::timeout(Duration::from_secs(4), rx.recv()).await;

        // The timer should be stopped, so we should timeout
        assert!(
            result.is_err(),
            "Should not receive snapshots after last subscriber disconnects"
        );
    }
}
