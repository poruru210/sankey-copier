mod config_publisher;

use crate::models::{
    HeartbeatMessage, PositionSnapshotMessage, RequestConfigMessage, SyncRequestMessage,
    TradeSignal, UnregisterMessage,
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

// Export the unified publisher with both names
// ZmqPublisher is the primary name (2-port architecture)
// ZmqConfigPublisher is kept for backward compatibility
#[allow(unused_imports)]
pub use config_publisher::{ZmqConfigPublisher, ZmqPublisher, SendFailure};

pub enum ZmqMessage {
    TradeSignal(TradeSignal),
    Unregister(UnregisterMessage),
    Heartbeat(HeartbeatMessage),
    RequestConfig(RequestConfigMessage),
    // Position sync protocol messages
    PositionSnapshot(PositionSnapshotMessage),
    SyncRequest(SyncRequestMessage),
}

/// Helper struct to determine message type from MessagePack data
#[derive(Debug, Deserialize)]
struct MessageTypeDiscriminator {
    message_type: Option<String>,
    action: Option<String>,
}

// Flexible struct to extract partial heartbeat data
#[derive(Debug, Deserialize)]
struct FlexibleHeartbeat {
    #[serde(default)]
    account_id: Option<String>,
}

pub struct ZmqServer {
    context: Arc<zmq::Context>,
    rx_sender: mpsc::UnboundedSender<ZmqMessage>,
    shutdown: Arc<AtomicBool>,
}

impl ZmqServer {
    pub fn new(rx_sender: mpsc::UnboundedSender<ZmqMessage>) -> Result<Self> {
        let context = Arc::new(zmq::Context::new());
        Ok(Self {
            context,
            rx_sender,
            shutdown: Arc::new(AtomicBool::new(false)),
        })
    }

    pub async fn start_receiver(&self, bind_address: &str) -> Result<JoinHandle<()>> {
        let socket = self
            .context
            .socket(zmq::PULL)
            .context("Failed to create ZMQ PULL socket")?;

        socket
            .bind(bind_address)
            .context(format!("Failed to bind to {}", bind_address))?;

        // Set receive timeout to allow periodic shutdown checks
        socket
            .set_rcvtimeo(100)
            .context("Failed to set receive timeout")?;

        tracing::info!("ZeroMQ receiver started on {}", bind_address);

        let tx = self.rx_sender.clone();
        let shutdown = self.shutdown.clone();

        // Run ZMQ in blocking thread since it's not async
        let handle = tokio::task::spawn_blocking(move || {
            while !shutdown.load(Ordering::Relaxed) {
                match socket.recv_bytes(0) {
                    Err(zmq::Error::EAGAIN) => {
                        // Timeout - continue checking shutdown flag
                        continue;
                    }
                    Ok(bytes) => {
                        // First, peek at the message to determine its type
                        match rmp_serde::from_slice::<MessageTypeDiscriminator>(&bytes) {
                            Ok(discriminator) => {
                                // Check message_type field first
                                if let Some(msg_type) = discriminator.message_type {
                                    match msg_type.as_str() {
                                        "RequestConfig" => {
                                            match rmp_serde::from_slice::<RequestConfigMessage>(
                                                &bytes,
                                            ) {
                                                Ok(req) => {
                                                    tracing::debug!(
                                                        "Received RequestConfig message: {:?}",
                                                        req
                                                    );
                                                    if let Err(e) =
                                                        tx.send(ZmqMessage::RequestConfig(req))
                                                    {
                                                        tracing::error!(
                                                            "Failed to send message to channel: {}",
                                                            e
                                                        );
                                                    }
                                                }
                                                Err(e) => {
                                                    tracing::error!("Failed to deserialize RequestConfig message: {}", e);
                                                }
                                            }
                                        }
                                        "Unregister" => {
                                            match rmp_serde::from_slice::<UnregisterMessage>(&bytes)
                                            {
                                                Ok(unreg) => {
                                                    tracing::debug!(
                                                        "Received Unregister message: {:?}",
                                                        unreg
                                                    );
                                                    if let Err(e) =
                                                        tx.send(ZmqMessage::Unregister(unreg))
                                                    {
                                                        tracing::error!(
                                                            "Failed to send message to channel: {}",
                                                            e
                                                        );
                                                    }
                                                }
                                                Err(e) => {
                                                    tracing::error!("Failed to deserialize Unregister message: {}", e);
                                                }
                                            }
                                        }
                                        "Heartbeat" => {
                                            match rmp_serde::from_slice::<HeartbeatMessage>(&bytes)
                                            {
                                                Ok(hb) => {
                                                    if let Err(e) =
                                                        tx.send(ZmqMessage::Heartbeat(hb))
                                                    {
                                                        tracing::error!(
                                                            "Failed to send message to channel: {}",
                                                            e
                                                        );
                                                    }
                                                }
                                                Err(e) => {
                                                    // Try to extract account_id from MessagePack for better error reporting
                                                    match rmp_serde::from_slice::<FlexibleHeartbeat>(
                                                        &bytes,
                                                    ) {
                                                        Ok(partial) => {
                                                            let acc = partial
                                                                .account_id
                                                                .as_deref()
                                                                .unwrap_or("unknown");
                                                            tracing::error!(
                                                                "Failed to deserialize Heartbeat message from EA [account_id: {}]: {}",
                                                                acc, e
                                                            );
                                                        }
                                                        Err(parse_err) => {
                                                            // Cannot extract any info - log raw bytes
                                                            let bytes_preview = if bytes.len() > 32
                                                            {
                                                                format!("{:02x?}...", &bytes[..32])
                                                            } else {
                                                                format!("{:02x?}", bytes)
                                                            };
                                                            tracing::error!(
                                                                "Failed to deserialize Heartbeat message: {} (data: {}, parse error: {})",
                                                                e, bytes_preview, parse_err
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        // Position sync protocol messages
                                        "PositionSnapshot" => {
                                            match rmp_serde::from_slice::<PositionSnapshotMessage>(
                                                &bytes,
                                            ) {
                                                Ok(snapshot) => {
                                                    tracing::debug!(
                                                        "Received PositionSnapshot from {}: {} positions",
                                                        snapshot.source_account,
                                                        snapshot.positions.len()
                                                    );
                                                    if let Err(e) = tx.send(
                                                        ZmqMessage::PositionSnapshot(snapshot),
                                                    ) {
                                                        tracing::error!(
                                                            "Failed to send PositionSnapshot to channel: {}",
                                                            e
                                                        );
                                                    }
                                                }
                                                Err(e) => {
                                                    tracing::error!(
                                                        "Failed to deserialize PositionSnapshot message: {}",
                                                        e
                                                    );
                                                }
                                            }
                                        }
                                        "SyncRequest" => {
                                            match rmp_serde::from_slice::<SyncRequestMessage>(
                                                &bytes,
                                            ) {
                                                Ok(req) => {
                                                    tracing::debug!(
                                                        "Received SyncRequest from {} for master {}",
                                                        req.slave_account,
                                                        req.master_account
                                                    );
                                                    if let Err(e) =
                                                        tx.send(ZmqMessage::SyncRequest(req))
                                                    {
                                                        tracing::error!(
                                                            "Failed to send SyncRequest to channel: {}",
                                                            e
                                                        );
                                                    }
                                                }
                                                Err(e) => {
                                                    tracing::error!(
                                                        "Failed to deserialize SyncRequest message: {}",
                                                        e
                                                    );
                                                }
                                            }
                                        }
                                        _ => {
                                            tracing::warn!("Unknown message_type: {}", msg_type);
                                        }
                                    }
                                } else if discriminator.action.is_some() {
                                    // Message has 'action' field - it's a TradeSignal
                                    match rmp_serde::from_slice::<TradeSignal>(&bytes) {
                                        Ok(signal) => {
                                            if let Err(e) = tx.send(ZmqMessage::TradeSignal(signal))
                                            {
                                                tracing::error!(
                                                    "Failed to send signal to channel: {}",
                                                    e
                                                );
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!(
                                                "Failed to deserialize TradeSignal: {}",
                                                e
                                            );
                                        }
                                    }
                                } else {
                                    tracing::error!(
                                        "Message has neither message_type nor action field"
                                    );
                                }
                            }
                            Err(e) => {
                                tracing::error!(
                                    "Failed to deserialize message discriminator: {}",
                                    e
                                );
                                tracing::debug!(
                                    "Raw message bytes (first 100): {:?}",
                                    &bytes[..bytes.len().min(100)]
                                );
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to receive ZMQ message: {}", e);
                        break;
                    }
                }
            }

            // Explicitly drop socket before context is destroyed (per ZeroMQ guide)
            drop(socket);
            tracing::info!("ZMQ receiver shut down cleanly");
        });

        Ok(handle)
    }

    /// Shutdown the ZMQ receiver gracefully
    #[allow(dead_code)]
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }
}

/// Generic message for ZMQ publishing
/// DEPRECATED: Use ZmqPublisher for unified publishing in 2-port architecture
#[allow(dead_code)]
struct PublishMessage<T> {
    topic: String,
    payload: T,
}

/// Generic ZeroMQ publisher using PUB/SUB pattern
/// DEPRECATED: Use ZmqPublisher (from config_publisher) for unified publishing in 2-port architecture
#[allow(dead_code)]
pub struct GenericZmqPublisher<T: Serialize + Clone + Send + 'static> {
    tx: mpsc::UnboundedSender<PublishMessage<T>>,
    _handle: JoinHandle<()>,
}

#[allow(dead_code)]
impl<T: Serialize + Clone + Send + 'static> GenericZmqPublisher<T> {
    pub fn new(bind_address: &str) -> Result<Self> {
        let context = zmq::Context::new();
        let socket = context
            .socket(zmq::PUB)
            .context("Failed to create ZMQ PUB socket")?;

        socket
            .bind(bind_address)
            .context(format!("Failed to bind to {}", bind_address))?;

        tracing::info!("ZeroMQ publisher (PUB) bound to {}", bind_address);

        let (tx, mut rx) = mpsc::unbounded_channel::<PublishMessage<T>>();

        // Spawn dedicated task for ZMQ sending
        let handle = tokio::task::spawn_blocking(move || {
            while let Some(msg) = rx.blocking_recv() {
                // PUB/SUBパターンでは、トピックをメッセージの先頭に付加
                match rmp_serde::to_vec_named(&msg.payload) {
                    Ok(msgpack) => {
                        // トピック + スペース + メッセージ
                        let mut message = msg.topic.as_bytes().to_vec();
                        message.push(b' '); // スペースで区切る
                        message.extend_from_slice(&msgpack);

                        if let Err(e) = socket.send(&message, 0) {
                            tracing::error!("Failed to send ZMQ message: {}", e);
                        } else {
                            tracing::debug!(
                                "Sent message to topic '{}': {} bytes (topic: {} bytes, payload: {} bytes)",
                                msg.topic,
                                message.len(),
                                msg.topic.len() + 1, // +1 for space
                                msgpack.len()
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to serialize message: {}", e);
                    }
                }
            }

            // Explicitly drop socket before context is destroyed (per ZeroMQ guide)
            drop(socket);
            drop(context);
            tracing::info!("ZMQ publisher shut down cleanly");
        });

        Ok(Self {
            tx,
            _handle: handle,
        })
    }

    /// Publish a message to a specific topic
    pub async fn publish(&self, topic: &str, payload: &T) -> Result<()> {
        let msg = PublishMessage {
            topic: topic.to_string(),
            payload: payload.clone(),
        };

        self.tx
            .send(msg)
            .map_err(|e| anyhow::anyhow!("Failed to send message to ZMQ publisher task: {}", e))?;

        Ok(())
    }
}

/// Type alias for trade signal publisher
/// DEPRECATED: Use ZmqPublisher::send_trade_signal() in 2-port architecture
#[allow(dead_code)]
pub type ZmqSender = GenericZmqPublisher<TradeSignal>;

/// Extension methods for ZmqSender to maintain existing API
/// DEPRECATED: Use ZmqPublisher::send_trade_signal() in 2-port architecture
#[allow(dead_code)]
impl ZmqSender {
    pub async fn send_signal(&self, trade_group_id: &str, signal: &TradeSignal) -> Result<()> {
        self.publish(trade_group_id, signal).await
    }
}

#[cfg(test)]
mod tests;
