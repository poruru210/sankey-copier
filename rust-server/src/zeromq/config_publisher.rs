use anyhow::{Result, Context};
use tokio::sync::mpsc;
use crate::models::ConfigMessage;

pub struct ZmqConfigPublisher {
    tx: mpsc::UnboundedSender<ConfigMessage>,
}

impl ZmqConfigPublisher {
    pub fn new(bind_address: &str) -> Result<Self> {
        let context = zmq::Context::new();
        let socket = context.socket(zmq::PUB)
            .context("Failed to create PUB socket")?;

        socket.bind(bind_address)
            .context(format!("Failed to bind to {}", bind_address))?;

        tracing::info!("ZeroMQ ConfigMessage publisher (MessagePack) bound to {}", bind_address);

        let (tx, mut rx) = mpsc::unbounded_channel::<ConfigMessage>();

        // Spawn dedicated task for ZMQ sending with MessagePack
        tokio::task::spawn_blocking(move || {
            while let Some(config) = rx.blocking_recv() {
                // Serialize to MessagePack
                match rmp_serde::to_vec(&config) {
                    Ok(msgpack_bytes) => {
                        // トピック + スペース + MessagePack bytes
                        let mut message = config.account_id.as_bytes().to_vec();
                        message.push(b' '); // スペースで区切る
                        message.extend_from_slice(&msgpack_bytes);

                        if let Err(e) = socket.send(&message, 0) {
                            tracing::error!("Failed to send ZMQ ConfigMessage: {}", e);
                        } else {
                            tracing::debug!(
                                "Sent MessagePack config to topic '{}': {} bytes (topic: {} bytes, payload: {} bytes)",
                                config.account_id,
                                message.len(),
                                config.account_id.len() + 1,
                                msgpack_bytes.len()
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to serialize ConfigMessage to MessagePack: {}", e);
                    }
                }
            }
        });

        Ok(Self { tx })
    }

    pub async fn send_config(&self, config: &ConfigMessage) -> Result<()> {
        self.tx.send(config.clone())
            .map_err(|e| anyhow::anyhow!("Failed to send ConfigMessage to publisher task: {}", e))?;
        Ok(())
    }
}
