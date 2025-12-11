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
use crate::models::EaConnection;

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
}

impl SnapshotBroadcaster {
    /// Create a new SnapshotBroadcaster
    pub fn new(tx: broadcast::Sender<String>, connection_manager: Arc<ConnectionManager>) -> Self {
        Self {
            subscriber_count: Arc::new(AtomicUsize::new(0)),
            task_handle: Arc::new(Mutex::new(None)),
            tx,
            connection_manager,
        }
    }

    /// Called when a WebSocket client connects
    pub async fn on_connect(&self) {
        let prev_count = self.subscriber_count.fetch_add(1, Ordering::SeqCst);

        if prev_count == 0 {
            // First subscriber: start the timer task
            tracing::info!("First WebSocket subscriber connected, starting snapshot timer");

            let tx = self.tx.clone();
            let connection_manager = self.connection_manager.clone();

            let handle = tokio::spawn(async move {
                snapshot_broadcast_loop(tx, connection_manager).await;
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
}

/// Background task that broadcasts connection snapshots at regular intervals
async fn snapshot_broadcast_loop(
    tx: broadcast::Sender<String>,
    connection_manager: Arc<ConnectionManager>,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(SNAPSHOT_INTERVAL_SECS));

    loop {
        interval.tick().await;

        // Fetch all connections
        let connections: Vec<EaConnection> = connection_manager.get_all_eas().await;

        // Serialize and broadcast
        match serde_json::to_string(&connections) {
            Ok(json) => {
                let message = format!("connections_snapshot:{}", json);
                if tx.send(message).is_err() {
                    // No receivers (shouldn't happen in normal operation)
                    tracing::warn!("No WebSocket receivers for snapshot broadcast");
                }
            }
            Err(e) => {
                tracing::error!("Failed to serialize connections snapshot: {}", e);
            }
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
