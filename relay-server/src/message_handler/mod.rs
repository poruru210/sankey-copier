//! Message handler module
//!
//! Coordinates trade copying logic by handling incoming ZMQ messages and routing
//! them to appropriate handlers.

use std::sync::Arc;
use tokio::sync::broadcast;

use crate::{
    connection_manager::ConnectionManager,
    db::Database,
    engine::CopyEngine,
    models::WarningCode,
    runtime_status_updater::{RuntimeStatusMetrics, RuntimeStatusUpdater},
    victoria_logs::VLogsController,
    zeromq::{ZmqConfigPublisher, ZmqMessage},
};

// Handler submodules
mod config_request;
mod heartbeat;
mod position_snapshot;
mod register;
mod sync_request;
mod trade_signal;
pub(crate) mod unregister;

#[cfg(test)]
mod tests;

/// Handles incoming ZMQ messages and coordinates trade copying logic
pub struct MessageHandler {
    connection_manager: Arc<ConnectionManager>,
    copy_engine: Arc<CopyEngine>,
    broadcast_tx: broadcast::Sender<String>,
    db: Arc<Database>,
    /// Unified ZMQ publisher for all outgoing messages (trade signals + config)
    publisher: Arc<ZmqConfigPublisher>,
    /// VictoriaLogs controller for EA config broadcasting
    vlogs_controller: Option<VLogsController>,
    runtime_status_metrics: Arc<RuntimeStatusMetrics>,
}

impl MessageHandler {
    pub fn new(
        connection_manager: Arc<ConnectionManager>,
        copy_engine: Arc<CopyEngine>,
        broadcast_tx: broadcast::Sender<String>,
        db: Arc<Database>,
        publisher: Arc<ZmqConfigPublisher>,
        vlogs_controller: Option<VLogsController>,
        runtime_status_metrics: Arc<RuntimeStatusMetrics>,
    ) -> Self {
        Self {
            connection_manager,
            copy_engine,
            broadcast_tx,
            db,
            publisher,
            vlogs_controller,
            runtime_status_metrics,
        }
    }

    /// Process a single ZMQ message
    pub async fn handle_message(&self, msg: ZmqMessage) {
        tracing::info!("[ZMQ] Received message: {:?}", std::mem::discriminant(&msg));
        match msg {
            ZmqMessage::RequestConfig(req_msg) => self.handle_request_config(req_msg).await,
            ZmqMessage::Unregister(unreg_msg) => self.handle_unregister(unreg_msg).await,
            ZmqMessage::Heartbeat(hb_msg) => self.handle_heartbeat(hb_msg).await,
            ZmqMessage::TradeSignal(signal) => self.handle_trade_signal(signal).await,
            ZmqMessage::Register(reg_msg) => self.handle_register(reg_msg).await,
            // Position sync protocol messages
            ZmqMessage::PositionSnapshot(snapshot) => self.handle_position_snapshot(snapshot).await,
            ZmqMessage::SyncRequest(request) => self.handle_sync_request(request).await,
        }
    }

    fn runtime_status_updater(&self) -> RuntimeStatusUpdater {
        RuntimeStatusUpdater::with_metrics(
            self.db.clone(),
            self.connection_manager.clone(),
            self.runtime_status_metrics.clone(),
        )
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn log_slave_runtime_trace(
    source: &'static str,
    master_account: &str,
    slave_account: &str,
    previous_status: i32,
    new_status: i32,
    allow_new_orders: bool,
    warning_codes: &[WarningCode],
    cluster_size: usize,
    masters_all_connected: bool,
) {
    tracing::event!(
        target: "status_engine",
        tracing::Level::INFO,
        source,
        master = %master_account,
        slave = %slave_account,
        previous_status = previous_status,
        status = new_status,
        status_changed = previous_status != new_status,
        allow_new_orders = allow_new_orders,
        warning_count = warning_codes.len(),
        cluster_size = cluster_size,
        masters_all_connected = masters_all_connected,
        warnings = ?warning_codes,
        "slave runtime evaluation"
    );
}
