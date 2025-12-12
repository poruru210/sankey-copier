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

use crate::api::AppState;
use crate::connection_manager::ConnectionManager;
use crate::db::Database;
use crate::models::{
    status_engine::{
        evaluate_master_status, evaluate_member_status, ConnectionSnapshot, MasterIntent,
        SlaveIntent,
    },
    SystemStateSnapshot,
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
