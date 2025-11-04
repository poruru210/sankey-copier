use anyhow::{Result, Context};
use serde::Serialize;
use tokio::sync::mpsc;
use std::sync::Arc;
use crate::models::{TradeSignal, MessageType, ConfigMessage};

pub enum ZmqMessage {
    TradeSignal(TradeSignal),
    Register(crate::models::RegisterMessage),
    Unregister(crate::models::UnregisterMessage),
    Heartbeat(crate::models::HeartbeatMessage),
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
                        // まずMessageTypeとして解析を試みる
                        match serde_json::from_slice::<MessageType>(&bytes) {
                            Ok(msg_type) => {
                                let zmq_msg = match msg_type {
                                    MessageType::Register(reg) => {
                                        tracing::debug!("Received Register message: {:?}", reg);
                                        ZmqMessage::Register(reg)
                                    }
                                    MessageType::Unregister(unreg) => {
                                        tracing::debug!("Received Unregister message: {:?}", unreg);
                                        ZmqMessage::Unregister(unreg)
                                    }
                                    MessageType::Heartbeat(hb) => {
                                        tracing::debug!("Received Heartbeat message from: {}", hb.account_id);
                                        ZmqMessage::Heartbeat(hb)
                                    }
                                    MessageType::TradeSignal(signal) => {
                                        tracing::debug!("Received TradeSignal: {:?}", signal);
                                        ZmqMessage::TradeSignal(signal)
                                    }
                                };

                                if let Err(e) = tx.send(zmq_msg) {
                                    tracing::error!("Failed to send message to channel: {}", e);
                                }
                            }
                            Err(_) => {
                                // 後方互換性: message_typeフィールドがない古いTradeSignal形式にフォールバック
                                match serde_json::from_slice::<TradeSignal>(&bytes) {
                                    Ok(signal) => {
                                        tracing::debug!("Received legacy TradeSignal: {:?}", signal);
                                        if let Err(e) = tx.send(ZmqMessage::TradeSignal(signal)) {
                                            tracing::error!("Failed to send signal to channel: {}", e);
                                        }
                                    }
                                    Err(e) => {
                                        tracing::error!("Failed to deserialize message: {}", e);
                                        tracing::debug!("Raw message: {}", String::from_utf8_lossy(&bytes));
                                    }
                                }
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
                match serde_json::to_vec(&msg.payload) {
                    Ok(json) => {
                        // トピック + スペース + メッセージ
                        let mut message = msg.topic.as_bytes().to_vec();
                        message.push(b' '); // スペースで区切る
                        message.extend_from_slice(&json);

                        if let Err(e) = socket.send(&message, 0) {
                            tracing::error!("Failed to send ZMQ message: {}", e);
                        } else {
                            tracing::debug!("Sent message to topic '{}'", msg.topic);
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
