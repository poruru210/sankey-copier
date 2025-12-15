// relay-server/src/zeromq/config_publisher.rs
//
// ZeroMQ unified publisher for all outgoing messages (config + trade signals)
// 2-port architecture: This single PUB socket handles all Server → EA messages

use anyhow::{Context, Result};
use sankey_copier_zmq::{build_trade_topic, ConfigMessage}; // Trait
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::domain::models::TradeSignal;

/// Pre-serialized message ready for ZMQ transmission
struct SerializedMessage {
    topic: String,
    payload: Vec<u8>, // MessagePack bytes
}

/// Unified ZeroMQ publisher for all outgoing messages
/// - Trade signals (to Slave EAs via trade_group_id topic)
/// - Config messages (to Master/Slave EAs via account_id topic)
/// - VLogs config broadcasts (to all EAs via vlogs_config topic)
pub struct ZmqPublisher {
    tx: mpsc::UnboundedSender<SerializedMessage>,
    _handle: JoinHandle<()>,
}

/// Type alias for backward compatibility
pub type ZmqConfigPublisher = ZmqPublisher;

impl ZmqPublisher {
    pub fn new(bind_address: &str) -> Result<Self> {
        let context = zmq::Context::new();
        let socket = context
            .socket(zmq::PUB)
            .context("Failed to create PUB socket")?;

        socket
            .bind(bind_address)
            .context(format!("Failed to bind to {}", bind_address))?;

        tracing::info!(
            "ZeroMQ unified publisher (MessagePack) bound to {}",
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
            tracing::info!("ZMQ unified publisher shut down cleanly");
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
        // Serialize to MessagePack (Map format for field-name based deserialization)
        let payload = rmp_serde::to_vec_named(message)
            .context("Failed to serialize message to MessagePack")?;

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
    /// Uses fixed topic "config/global" for system-wide broadcast
    pub async fn broadcast_vlogs_config(
        &self,
        settings: &crate::domain::models::VLogsGlobalSettings,
    ) -> Result<()> {
        let message = sankey_copier_zmq::GlobalConfigMessage {
            enabled: settings.enabled,
            endpoint: settings.endpoint.clone(),
            batch_size: settings.batch_size,
            flush_interval_secs: settings.flush_interval_secs,
            log_level: settings.log_level.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        self.publish_to_topic("config/global", &message).await?;

        tracing::info!(
            enabled = settings.enabled,
            endpoint = %settings.endpoint,
            log_level = %settings.log_level,
            "Broadcasted VictoriaLogs config to all EAs on 'config/global' topic"
        );

        Ok(())
    }

    /// Send trade signal to a specific Master-Slave pair
    /// Uses trade/{master_id}/{slave_id} as the topic for precise routing
    pub async fn send_trade_signal(
        &self,
        master_id: &str,
        slave_id: &str,
        signal: &TradeSignal,
    ) -> Result<()> {
        // Use rmp_serde::to_vec_named to match the previous ZmqSender serialization format
        let payload = rmp_serde::to_vec_named(signal)
            .context("Failed to serialize TradeSignal to MessagePack")?;

        let serialized = SerializedMessage {
            topic: build_trade_topic(master_id, slave_id),
            payload,
        };

        self.tx
            .send(serialized)
            .map_err(|e| anyhow::anyhow!("Failed to send trade signal: {}", e))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sankey_copier_zmq::{MasterConfigMessage, SlaveConfigMessage};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_create_publisher() {
        // Test publisher creation with valid bind address
        use std::sync::atomic::{AtomicU16, Ordering};
        static PORT: AtomicU16 = AtomicU16::new(25557);
        let port = PORT.fetch_add(1, Ordering::SeqCst);

        let result = ZmqPublisher::new(&format!("tcp://127.0.0.1:{}", port));
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_publisher_invalid_address() {
        // Test publisher creation with invalid bind address
        let result = ZmqPublisher::new("invalid://address");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_send_slave_config() {
        // Test sending slave config message using unified send()
        use std::sync::atomic::{AtomicU16, Ordering};
        static PORT: AtomicU16 = AtomicU16::new(26557);
        let port = PORT.fetch_add(1, Ordering::SeqCst);

        let publisher = ZmqPublisher::new(&format!("tcp://127.0.0.1:{}", port)).unwrap();

        let master_account = "MASTER456".to_string();
        let account_id = "TEST123".to_string();
        let trade_group_id = master_account.clone();

        let config = SlaveConfigMessage {
            account_id: account_id.to_string(),
            master_account: master_account.to_string(),
            timestamp: Utc::now().timestamp_millis(),
            trade_group_id: trade_group_id.to_string(),
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
            warning_codes: Vec::new(),
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

        let publisher = ZmqPublisher::new(&format!("tcp://127.0.0.1:{}", port)).unwrap();

        let config = MasterConfigMessage {
            account_id: "MASTER123".to_string(),
            status: 2, // STATUS_CONNECTED
            symbol_prefix: Some("pro.".to_string()),
            symbol_suffix: Some(".m".to_string()),
            config_version: 1,
            timestamp: chrono::Utc::now().timestamp_millis(),
            warning_codes: Vec::new(),
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

        let publisher = Arc::new(ZmqPublisher::new(&format!("tcp://127.0.0.1:{}", port)).unwrap());

        let mut handles = vec![];

        // Define constants/structs needed for the test
        const STATUS_DISABLED: i32 = 0;
        struct Member {
            id: String,
            master_id: String,
        }

        // Send 10 slave configs concurrently using unified send()
        for i in 0..10 {
            let pub_clone = Arc::clone(&publisher);
            let handle = tokio::spawn(async move {
                let master_account = "MASTER".to_string();
                let member = Member {
                    id: format!("SLAVE{}", i),
                    master_id: master_account.clone(),
                };
                let trade_group_id = master_account.clone();

                let config = SlaveConfigMessage {
                    account_id: member.id.clone(),
                    master_account: member.master_id.clone(),
                    timestamp: Utc::now().timestamp_millis(),
                    trade_group_id: trade_group_id.clone(),
                    status: STATUS_DISABLED,
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
                    warning_codes: Vec::new(),
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
