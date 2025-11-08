mod config_publisher;

use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};
use tokio::sync::mpsc;
use std::sync::Arc;
use crate::models::{TradeSignal, ConfigMessage, RegisterMessage, UnregisterMessage, HeartbeatMessage};

pub use config_publisher::ZmqConfigPublisher;

pub enum ZmqMessage {
    TradeSignal(TradeSignal),
    Register(RegisterMessage),
    Unregister(UnregisterMessage),
    Heartbeat(HeartbeatMessage),
}

/// Helper struct to determine message type from MessagePack data
#[derive(Debug, Deserialize)]
struct MessageTypeDiscriminator {
    message_type: Option<String>,
    action: Option<String>,
}

pub struct ZmqServer {
    context: Arc<zmq::Context>,
    rx_sender: mpsc::UnboundedSender<ZmqMessage>,
}

impl ZmqServer {
    pub fn new(rx_sender: mpsc::UnboundedSender<ZmqMessage>) -> Result<Self> {
        let context = Arc::new(zmq::Context::new());
        Ok(Self {
            context,
            rx_sender,
        })
    }

    pub async fn start_receiver(&self, bind_address: &str) -> Result<()> {
        let socket = self.context.socket(zmq::PULL)
            .context("Failed to create ZMQ PULL socket")?;

        socket.bind(bind_address)
            .context(format!("Failed to bind to {}", bind_address))?;

        tracing::info!("ZeroMQ receiver started on {}", bind_address);

        let tx = self.rx_sender.clone();

        // Run ZMQ in blocking thread since it's not async
        tokio::task::spawn_blocking(move || {
            loop {
                match socket.recv_bytes(0) {
                    Ok(bytes) => {
                        // First, peek at the message to determine its type
                        match rmp_serde::from_slice::<MessageTypeDiscriminator>(&bytes) {
                            Ok(discriminator) => {
                                // Check message_type field first
                                if let Some(msg_type) = discriminator.message_type {
                                    match msg_type.as_str() {
                                        "Register" => {
                                            match rmp_serde::from_slice::<RegisterMessage>(&bytes) {
                                                Ok(reg) => {
                                                    tracing::debug!("Received Register message: {:?}", reg);
                                                    if let Err(e) = tx.send(ZmqMessage::Register(reg)) {
                                                        tracing::error!("Failed to send message to channel: {}", e);
                                                    }
                                                }
                                                Err(e) => {
                                                    tracing::error!("Failed to deserialize Register message: {}", e);
                                                }
                                            }
                                        }
                                        "Unregister" => {
                                            match rmp_serde::from_slice::<UnregisterMessage>(&bytes) {
                                                Ok(unreg) => {
                                                    tracing::debug!("Received Unregister message: {:?}", unreg);
                                                    if let Err(e) = tx.send(ZmqMessage::Unregister(unreg)) {
                                                        tracing::error!("Failed to send message to channel: {}", e);
                                                    }
                                                }
                                                Err(e) => {
                                                    tracing::error!("Failed to deserialize Unregister message: {}", e);
                                                }
                                            }
                                        }
                                        "Heartbeat" => {
                                            match rmp_serde::from_slice::<HeartbeatMessage>(&bytes) {
                                                Ok(hb) => {
                                                    tracing::debug!("Received Heartbeat message from: {}", hb.account_id);
                                                    if let Err(e) = tx.send(ZmqMessage::Heartbeat(hb)) {
                                                        tracing::error!("Failed to send message to channel: {}", e);
                                                    }
                                                }
                                                Err(e) => {
                                                    tracing::error!("Failed to deserialize Heartbeat message: {}", e);
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
                                            tracing::debug!("Received TradeSignal: {:?}", signal);
                                            if let Err(e) = tx.send(ZmqMessage::TradeSignal(signal)) {
                                                tracing::error!("Failed to send signal to channel: {}", e);
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!("Failed to deserialize TradeSignal: {}", e);
                                        }
                                    }
                                } else {
                                    tracing::error!("Message has neither message_type nor action field");
                                }
                            }
                            Err(e) => {
                                tracing::error!("Failed to deserialize message discriminator: {}", e);
                                tracing::debug!("Raw message bytes (first 100): {:?}", &bytes[..bytes.len().min(100)]);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to receive ZMQ message: {}", e);
                    }
                }
            }
        });

        Ok(())
    }
}

/// Generic message for ZMQ publishing
struct PublishMessage<T> {
    topic: String,
    payload: T,
}

/// Generic ZeroMQ publisher using PUB/SUB pattern
pub struct ZmqPublisher<T: Serialize + Clone + Send + 'static> {
    tx: mpsc::UnboundedSender<PublishMessage<T>>,
}

impl<T: Serialize + Clone + Send + 'static> ZmqPublisher<T> {
    pub fn new(bind_address: &str) -> Result<Self> {
        let context = zmq::Context::new();
        let socket = context.socket(zmq::PUB)
            .context("Failed to create ZMQ PUB socket")?;

        socket.bind(bind_address)
            .context(format!("Failed to bind to {}", bind_address))?;

        tracing::info!("ZeroMQ publisher (PUB) bound to {}", bind_address);

        let (tx, mut rx) = mpsc::unbounded_channel::<PublishMessage<T>>();

        // Spawn dedicated task for ZMQ sending
        tokio::task::spawn_blocking(move || {
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
        });

        Ok(Self { tx })
    }

    /// Publish a message to a specific topic
    pub async fn publish(&self, topic: &str, payload: &T) -> Result<()> {
        let msg = PublishMessage {
            topic: topic.to_string(),
            payload: payload.clone(),
        };

        self.tx.send(msg)
            .map_err(|e| anyhow::anyhow!("Failed to send message to ZMQ publisher task: {}", e))?;

        Ok(())
    }
}

/// Type alias for trade signal publisher
pub type ZmqSender = ZmqPublisher<TradeSignal>;

/// Type alias for config message publisher
pub type ZmqConfigSender = ZmqPublisher<ConfigMessage>;

/// Extension methods for ZmqSender to maintain existing API
impl ZmqSender {
    pub async fn send_signal(&self, trade_group_id: &str, signal: &TradeSignal) -> Result<()> {
        self.publish(trade_group_id, signal).await
    }
}

/// Extension methods for ZmqConfigSender to maintain existing API
impl ZmqConfigSender {
    pub async fn send_config(&self, config: &ConfigMessage) -> Result<()> {
        self.publish(&config.account_id, config).await
    }
}
