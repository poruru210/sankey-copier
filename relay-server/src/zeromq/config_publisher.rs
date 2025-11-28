// relay-server/src/zeromq/config_publisher.rs
//
// ZeroMQ configuration publisher using trait-based unified sending

use anyhow::{Context, Result};
use sankey_copier_zmq::ConfigMessage; // Trait
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// Pre-serialized message ready for ZMQ transmission
struct SerializedMessage {
    topic: String,
    payload: Vec<u8>, // MessagePack bytes
}

pub struct ZmqConfigPublisher {
    tx: mpsc::UnboundedSender<SerializedMessage>,
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
            "ZeroMQ config publisher (MessagePack) bound to {}",
            bind_address
        );

        let (tx, mut rx) = mpsc::unbounded_channel::<SerializedMessage>();

        // Spawn dedicated task for ZMQ sending
        let handle = tokio::task::spawn_blocking(move || {
            while let Some(msg) = rx.blocking_recv() {
                // Build ZMQ message: topic + space + MessagePack
                let mut zmq_message = msg.topic.as_bytes().to_vec();
                zmq_message.push(b' ');
                zmq_message.extend_from_slice(&msg.payload);

                if let Err(e) = socket.send(&zmq_message, 0) {
                    tracing::error!("Failed to send ZMQ message to topic '{}': {}", msg.topic, e);
                } else {
                    tracing::debug!(
                        "Sent MessagePack message to topic '{}': {} bytes",
                        msg.topic,
                        zmq_message.len()
                    );
                }
            }

            // Explicitly drop socket before context is destroyed
            drop(socket);
            drop(context);
            tracing::info!("ZMQ config publisher shut down cleanly");
        });

        Ok(Self {
            tx,
            _handle: handle,
        })
    }

    /// Unified send method for all ConfigMessage types
    /// Uses trait-based interface for type safety and extensibility
    pub async fn send<T>(&self, message: &T) -> Result<()>
    where
        T: ConfigMessage,
    {
        // Serialize to MessagePack
        let payload =
            rmp_serde::to_vec(message).context("Failed to serialize message to MessagePack")?;

        let serialized = SerializedMessage {
            topic: message.zmq_topic().to_string(),
            payload,
        };

        self.tx
            .send(serialized)
            .map_err(|e| anyhow::anyhow!("Failed to send message: {}", e))?;

        Ok(())
    }

    /// Publish any serializable message to a specific topic
    /// Used for sync protocol messages (SyncRequest, PositionSnapshot)
    pub async fn publish_to_topic<T>(&self, topic: &str, message: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        let payload = rmp_serde::to_vec_named(message)
            .context("Failed to serialize message to MessagePack")?;

        let serialized = SerializedMessage {
            topic: topic.to_string(),
            payload,
        };

        self.tx
            .send(serialized)
            .map_err(|e| anyhow::anyhow!("Failed to send message: {}", e))?;

        Ok(())
    }

    /// Broadcast VictoriaLogs configuration to all EAs
    /// Uses fixed topic "vlogs_config" for global broadcast
    pub async fn broadcast_vlogs_config(
        &self,
        settings: &crate::models::VLogsGlobalSettings,
    ) -> Result<()> {
        let message = sankey_copier_zmq::VLogsConfigMessage {
            enabled: settings.enabled,
            endpoint: settings.endpoint.clone(),
            batch_size: settings.batch_size,
            flush_interval_secs: settings.flush_interval_secs,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        self.publish_to_topic("vlogs_config", &message).await?;

        tracing::info!(
            enabled = settings.enabled,
            endpoint = %settings.endpoint,
            "Broadcasted VictoriaLogs config to all EAs on 'vlogs_config' topic"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sankey_copier_zmq::{MasterConfigMessage, SlaveConfigMessage};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_create_publisher() {
        // Test publisher creation with valid bind address
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
        // Test sending slave config message using unified send()
        use std::sync::atomic::{AtomicU16, Ordering};
        static PORT: AtomicU16 = AtomicU16::new(26557);
        let port = PORT.fetch_add(1, Ordering::SeqCst);

        let publisher = ZmqConfigPublisher::new(&format!("tcp://127.0.0.1:{}", port)).unwrap();

        let master_account = "MASTER456".to_string();
        let config = SlaveConfigMessage {
            account_id: "TEST123".to_string(),
            master_account: master_account.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            trade_group_id: master_account.clone(),
            status: 0, // 0 = DISABLED
            lot_calculation_mode: sankey_copier_zmq::LotCalculationMode::default(),
            lot_multiplier: Some(2.0),
            reverse_trade: false,
            symbol_prefix: None,
            symbol_suffix: None,
            symbol_mappings: vec![],
            filters: sankey_copier_zmq::TradeFilters::default(),
            config_version: 1,
            source_lot_min: None,
            source_lot_max: None,
            master_equity: Some(10000.0),
            // Open Sync Policy defaults
            sync_mode: sankey_copier_zmq::SyncMode::default(),
            limit_order_expiry_min: None,
            market_sync_max_pips: None,
            max_slippage: None,
            copy_pending_orders: false,
            // Trade Execution defaults
            max_retries: 3,
            max_signal_delay_ms: 5000,
            use_pending_order_for_delayed: false,
            allow_new_orders: true,
        };

        // This should succeed (message is queued for sending)
        let result = publisher.send(&config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_master_config() {
        // Test sending master config message using unified send()
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
        let result = publisher.send(&config).await;
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

        // Send 10 slave configs concurrently using unified send()
        for i in 0..10 {
            let pub_clone = Arc::clone(&publisher);
            let handle = tokio::spawn(async move {
                let master_account = "MASTER".to_string();
                let config = SlaveConfigMessage {
                    account_id: format!("SLAVE{}", i),
                    master_account: master_account.clone(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    trade_group_id: master_account.clone(),
                    status: 0, // 0 = DISABLED
                    lot_calculation_mode: sankey_copier_zmq::LotCalculationMode::default(),
                    lot_multiplier: Some(1.0),
                    reverse_trade: false,
                    symbol_prefix: None,
                    symbol_suffix: None,
                    symbol_mappings: vec![],
                    filters: sankey_copier_zmq::TradeFilters::default(),
                    config_version: 1,
                    source_lot_min: None,
                    source_lot_max: None,
                    master_equity: None,
                    // Open Sync Policy defaults
                    sync_mode: sankey_copier_zmq::SyncMode::default(),
                    limit_order_expiry_min: None,
                    market_sync_max_pips: None,
                    max_slippage: None,
                    copy_pending_orders: false,
                    // Trade Execution defaults
                    max_retries: 3,
                    max_signal_delay_ms: 5000,
                    use_pending_order_for_delayed: false,
                    allow_new_orders: true,
                };
                pub_clone.send(&config).await
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
