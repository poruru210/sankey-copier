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
    zeromq::{ZmqConfigPublisher, ZmqMessage, ZmqSender},
};

// Handler submodules
mod config_request;
mod heartbeat;
mod position_snapshot;
mod sync_request;
mod trade_signal;
mod unregister;

#[cfg(test)]
mod tests;

/// Handles incoming ZMQ messages and coordinates trade copying logic
pub struct MessageHandler {
    connection_manager: Arc<ConnectionManager>,
    copy_engine: Arc<CopyEngine>,
    zmq_sender: Arc<ZmqSender>,
    broadcast_tx: broadcast::Sender<String>,
    db: Arc<Database>,
    config_sender: Arc<ZmqConfigPublisher>,
}

impl MessageHandler {
    pub fn new(
        connection_manager: Arc<ConnectionManager>,
        copy_engine: Arc<CopyEngine>,
        zmq_sender: Arc<ZmqSender>,
        broadcast_tx: broadcast::Sender<String>,
        db: Arc<Database>,
        config_sender: Arc<ZmqConfigPublisher>,
    ) -> Self {
        Self {
            connection_manager,
            copy_engine,
            zmq_sender,
            broadcast_tx,
            db,
            config_sender,
        }
    }

    /// Process a single ZMQ message
    pub async fn handle_message(&self, msg: ZmqMessage) {
        match msg {
            ZmqMessage::RequestConfig(req_msg) => self.handle_request_config(req_msg).await,
            ZmqMessage::Unregister(unreg_msg) => self.handle_unregister(unreg_msg).await,
            ZmqMessage::Heartbeat(hb_msg) => self.handle_heartbeat(hb_msg).await,
            ZmqMessage::TradeSignal(signal) => self.handle_trade_signal(signal).await,
            // Position sync protocol messages
            ZmqMessage::PositionSnapshot(snapshot) => self.handle_position_snapshot(snapshot).await,
            ZmqMessage::SyncRequest(request) => self.handle_sync_request(request).await,
        }
    }
}
