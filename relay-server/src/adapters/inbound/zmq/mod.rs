//! Message handler module
//!
//! Coordinates trade copying logic by handling incoming ZMQ messages and routing
//! them to appropriate handlers.

use std::sync::Arc;
use tokio::sync::broadcast;

use crate::{
    adapters::infrastructure::connection_manager::ConnectionManager,
    adapters::outbound::messaging::{ZmqConfigPublisher, ZmqMessage},
    adapters::outbound::observability::victoria_logs::VLogsController,
    adapters::outbound::persistence::Database,
    application::runtime_status_updater::{RuntimeStatusMetrics, RuntimeStatusUpdater},
    domain::services::copy_engine::CopyEngine,
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
pub(crate) mod test_helpers;

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

    /// Status service for heartbeat processing (Hexagonal Architecture)
    status_service: crate::application::StatusService,
    /// Service for handling disconnection events
    disconnection_service: Arc<dyn crate::ports::DisconnectionService>,
    /// Application configuration (for symbol mappings)
    #[allow(dead_code)]
    config: Arc<crate::config::Config>,
}

impl MessageHandler {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        connection_manager: Arc<ConnectionManager>,
        copy_engine: Arc<CopyEngine>,
        broadcast_tx: broadcast::Sender<String>,
        db: Arc<Database>,
        publisher: Arc<ZmqConfigPublisher>,
        vlogs_controller: Option<VLogsController>,
        runtime_status_metrics: Arc<RuntimeStatusMetrics>,
        status_service: crate::application::StatusService,
        disconnection_service: Arc<dyn crate::ports::DisconnectionService>,
        config: Arc<crate::config::Config>,
    ) -> Self {
        Self {
            connection_manager: connection_manager.clone(),
            copy_engine,
            broadcast_tx: broadcast_tx.clone(),
            db: db.clone(),
            publisher,
            vlogs_controller,
            runtime_status_metrics,

            status_service,
            disconnection_service,
            config,
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
