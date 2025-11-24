use crate::models::ConfigMessage;
use anyhow::{Context, Result};
use sankey_copier_zmq::MasterConfigMessage;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// Enum to support both Slave and Master config messages
enum ConfigPayload {
    Slave(ConfigMessage),
    Master(MasterConfigMessage),
}

pub struct ZmqConfigPublisher {
    tx: mpsc::UnboundedSender<ConfigPayload>,
    _handle: JoinHandle<()>,
}

impl ZmqConfigPublisher {
    pub fn new(bind_address: &str) -> Result<Self> {
        let context = zmq::Context::new();
        let socket = context
            .socket(zmq::PUB)
            .context("Failed to create PUB socket")?;

        socket
            .bind(bind_address)
            .context(format!("Failed to bind to {}", bind_address))?;

        tracing::info!(
            "ZeroMQ ConfigMessage publisher (MessagePack) bound to {}",
            bind_address
        );

        let (tx, mut rx) = mpsc::unbounded_channel::<ConfigPayload>();

        // Spawn dedicated task for ZMQ sending with MessagePack
        let handle = tokio::task::spawn_blocking(move || {
            while let Some(payload) = rx.blocking_recv() {
                match payload {
                    ConfigPayload::Slave(config) => {
                        // Serialize Slave ConfigMessage to MessagePack
                        match rmp_serde::to_vec(&config) {
                            Ok(msgpack_bytes) => {
                                // トピック + スペース + MessagePack bytes
                                let mut message = config.account_id.as_bytes().to_vec();
                                message.push(b' '); // スペースで区切る
                                message.extend_from_slice(&msgpack_bytes);

                                if let Err(e) = socket.send(&message, 0) {
                                    tracing::error!(
                                        "Failed to send ZMQ ConfigMessage (Slave): {}",
                                        e
                                    );
                                } else {
                                    tracing::debug!(
                                        "Sent MessagePack config (Slave) to topic '{}': {} bytes",
                                        config.account_id,
                                        message.len()
                                    );
                                }
                            }
                            Err(e) => {
                                tracing::error!(
                                    "Failed to serialize ConfigMessage (Slave) to MessagePack: {}",
                                    e
                                );
                            }
                        }
                    }
                    ConfigPayload::Master(config) => {
                        // Serialize Master MasterConfigMessage to MessagePack
                        match rmp_serde::to_vec(&config) {
                            Ok(msgpack_bytes) => {
                                // トピック + スペース + MessagePack bytes
                                let mut message = config.account_id.as_bytes().to_vec();
                                message.push(b' '); // スペースで区切る
                                message.extend_from_slice(&msgpack_bytes);

                                if let Err(e) = socket.send(&message, 0) {
                                    tracing::error!(
                                        "Failed to send ZMQ MasterConfigMessage: {}",
                                        e
                                    );
                                } else {
                                    tracing::debug!(
                                        "Sent MessagePack config (Master) to topic '{}': {} bytes",
                                        config.account_id,
                                        message.len()
                                    );
                                }
                            }
                            Err(e) => {
                                tracing::error!(
                                    "Failed to serialize MasterConfigMessage to MessagePack: {}",
                                    e
                                );
                            }
                        }
                    }
                }
            }

            // Explicitly drop socket before context is destroyed (per ZeroMQ guide)
            drop(socket);
            drop(context);
            tracing::info!("ZMQ config publisher shut down cleanly");
        });

        Ok(Self {
            tx,
            _handle: handle,
        })
    }

    pub async fn send_config(&self, config: &ConfigMessage) -> Result<()> {
        self.tx
            .send(ConfigPayload::Slave(config.clone()))
            .map_err(|e| {
                anyhow::anyhow!("Failed to send ConfigMessage to publisher task: {}", e)
            })?;
        Ok(())
    }

    pub async fn send_master_config(&self, config: &MasterConfigMessage) -> Result<()> {
        self.tx
            .send(ConfigPayload::Master(config.clone()))
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to send MasterConfigMessage to publisher task: {}",
                    e
                )
            })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_create_publisher() {
        // Test publisher creation with valid bind address
        // Use random port to avoid conflicts
        use std::sync::atomic::{AtomicU16, Ordering};
        static PORT: AtomicU16 = AtomicU16::new(25557);
        let port = PORT.fetch_add(1, Ordering::SeqCst);

        let result = ZmqConfigPublisher::new(&format!("tcp://127.0.0.1:{}", port));
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_publisher_invalid_address() {
        // Test publisher creation with invalid bind address
        let result = ZmqConfigPublisher::new("invalid://address");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_send_slave_config() {
        // Test sending slave config message
        use std::sync::atomic::{AtomicU16, Ordering};
        static PORT: AtomicU16 = AtomicU16::new(26557);
        let port = PORT.fetch_add(1, Ordering::SeqCst);

        let publisher = ZmqConfigPublisher::new(&format!("tcp://127.0.0.1:{}", port)).unwrap();

        let config = ConfigMessage {
            account_id: "TEST123".to_string(),
            master_account: "MASTER456".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            status: 1,
            lot_multiplier: Some(2.0),
            reverse_trade: false,
            symbol_prefix: None,
            symbol_suffix: None,
            symbol_mappings: vec![],
            filters: crate::models::TradeFilters::default(),
            config_version: 1,
        };

        // This should succeed (message is queued for sending)
        let result = publisher.send_config(&config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_master_config() {
        // Test sending master config message
        use std::sync::atomic::{AtomicU16, Ordering};
        static PORT: AtomicU16 = AtomicU16::new(27557);
        let port = PORT.fetch_add(1, Ordering::SeqCst);

        let publisher = ZmqConfigPublisher::new(&format!("tcp://127.0.0.1:{}", port)).unwrap();

        let config = MasterConfigMessage {
            account_id: "MASTER123".to_string(),
            symbol_prefix: Some("pro.".to_string()),
            symbol_suffix: Some(".m".to_string()),
            config_version: 1,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        // This should succeed (message is queued for sending)
        let result = publisher.send_master_config(&config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_concurrent_sends() {
        // Test sending multiple messages concurrently
        use std::sync::atomic::{AtomicU16, Ordering};
        static PORT: AtomicU16 = AtomicU16::new(28557);
        let port = PORT.fetch_add(1, Ordering::SeqCst);

        let publisher =
            Arc::new(ZmqConfigPublisher::new(&format!("tcp://127.0.0.1:{}", port)).unwrap());

        let mut handles = vec![];

        // Send 10 slave configs concurrently
        for i in 0..10 {
            let pub_clone = Arc::clone(&publisher);
            let handle = tokio::spawn(async move {
                let config = ConfigMessage {
                    account_id: format!("SLAVE{}", i),
                    master_account: "MASTER".to_string(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    status: 1,
                    lot_multiplier: Some(1.0),
                    reverse_trade: false,
                    symbol_prefix: None,
                    symbol_suffix: None,
                    symbol_mappings: vec![],
                    filters: crate::models::TradeFilters::default(),
                    config_version: 1,
                };
                pub_clone.send_config(&config).await
            });
            handles.push(handle);
        }

        // All sends should succeed
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
        }
    }
}
