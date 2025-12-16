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

// Adapter implementation for Outbound Port
use crate::domain::models::VLogsGlobalSettings;
use async_trait::async_trait;
use sankey_copier_zmq::{MasterConfigMessage, SlaveConfigMessage};

#[async_trait]
impl crate::ports::ConfigPublisher for ZmqPublisher {
    async fn send_master_config(&self, config: &MasterConfigMessage) -> anyhow::Result<()> {
        self.send(config).await
    }

    async fn send_slave_config(&self, config: &SlaveConfigMessage) -> anyhow::Result<()> {
        self.send(config).await
    }

    async fn broadcast_vlogs_config(&self, config: &VLogsGlobalSettings) -> anyhow::Result<()> {
        self.broadcast_vlogs_config(config).await
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

    #[tokio::test]
    async fn test_send_trade_signal() {
        // Test sending trade signal to specific Master-Slave pair
        use std::sync::atomic::{AtomicU16, Ordering};
        static PORT: AtomicU16 = AtomicU16::new(29557);
        let port = PORT.fetch_add(1, Ordering::SeqCst);

        let publisher = ZmqPublisher::new(&format!("tcp://127.0.0.1:{}", port)).unwrap();

        let signal = TradeSignal {
            action: sankey_copier_zmq::TradeAction::Open,
            ticket: 12345,
            symbol: Some("EURUSD".to_string()),
            order_type: Some(sankey_copier_zmq::OrderType::Buy),
            lots: Some(0.1),
            open_price: Some(1.1000),
            stop_loss: None,
            take_profit: None,
            magic_number: Some(0),
            comment: Some("Test Trade".to_string()),
            timestamp: Utc::now(),
            source_account: "MASTER_001".to_string(),
            close_ratio: None,
        };

        let result = publisher
            .send_trade_signal("MASTER_001", "SLAVE_001", &signal)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_publish_to_topic() {
        // Test publishing arbitrary message to custom topic
        use std::sync::atomic::{AtomicU16, Ordering};
        static PORT: AtomicU16 = AtomicU16::new(30557);
        let port = PORT.fetch_add(1, Ordering::SeqCst);

        let publisher = ZmqPublisher::new(&format!("tcp://127.0.0.1:{}", port)).unwrap();

        #[derive(serde::Serialize)]
        struct CustomMessage {
            msg_type: String,
            data: String,
        }

        let custom = CustomMessage {
            msg_type: "test".to_string(),
            data: "hello".to_string(),
        };

        let result = publisher.publish_to_topic("custom/topic", &custom).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_topic_generation_trade() {
        // Test that trade topic generation works correctly
        let topic = build_trade_topic("MASTER_001", "SLAVE_001");
        assert_eq!(topic, "trade/MASTER_001/SLAVE_001");

        let topic_long = build_trade_topic("LONG_MASTER_ID_12345", "LONG_SLAVE_ID_67890");
        assert_eq!(topic_long, "trade/LONG_MASTER_ID_12345/LONG_SLAVE_ID_67890");
    }

    #[test]
    fn test_master_config_zmq_topic() {
        // Test MasterConfigMessage uses account_id as topic
        let config = MasterConfigMessage {
            account_id: "MASTER_FOR_TOPIC".to_string(),
            status: 2,
            symbol_prefix: None,
            symbol_suffix: None,
            config_version: 1,
            timestamp: 1234567890,
            warning_codes: Vec::new(),
        };

        // Note: Topic includes "config/" prefix for routing
        assert_eq!(config.zmq_topic(), "config/MASTER_FOR_TOPIC");
    }

    #[test]
    fn test_slave_config_zmq_topic() {
        // Test SlaveConfigMessage uses account_id as topic
        let config = SlaveConfigMessage {
            account_id: "SLAVE_FOR_TOPIC".to_string(),
            master_account: "MASTER_X".to_string(),
            timestamp: 1234567890,
            trade_group_id: "MASTER_X".to_string(),
            status: 2,
            lot_calculation_mode: sankey_copier_zmq::LotCalculationMode::default(),
            lot_multiplier: None,
            reverse_trade: false,
            symbol_prefix: None,
            symbol_suffix: None,
            symbol_mappings: vec![],
            filters: sankey_copier_zmq::TradeFilters::default(),
            config_version: 0,
            source_lot_min: None,
            source_lot_max: None,
            master_equity: None,
            sync_mode: sankey_copier_zmq::SyncMode::default(),
            limit_order_expiry_min: None,
            market_sync_max_pips: None,
            max_slippage: None,
            copy_pending_orders: false,
            max_retries: 3,
            max_signal_delay_ms: 5000,
            use_pending_order_for_delayed: false,
            allow_new_orders: true,
            warning_codes: Vec::new(),
        };

        // Note: Topic includes "config/" prefix for routing
        assert_eq!(config.zmq_topic(), "config/SLAVE_FOR_TOPIC");
    }

    #[test]
    fn test_messagepack_serialization_master_config() {
        // Test MessagePack serialization format and size
        let config = MasterConfigMessage {
            account_id: "MASTER_MSGPACK".to_string(),
            status: 2,
            symbol_prefix: Some("pro.".to_string()),
            symbol_suffix: Some(".m".to_string()),
            config_version: 5,
            timestamp: 1702666800000,
            warning_codes: Vec::new(),
        };

        let bytes = rmp_serde::to_vec_named(&config).unwrap();

        // Should be non-empty
        assert!(!bytes.is_empty());

        // Should deserialize back to same values
        let decoded: MasterConfigMessage = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.account_id, config.account_id);
        assert_eq!(decoded.status, config.status);
        assert_eq!(decoded.symbol_prefix, config.symbol_prefix);
        assert_eq!(decoded.symbol_suffix, config.symbol_suffix);
        assert_eq!(decoded.config_version, config.config_version);
    }

    #[test]
    fn test_messagepack_serialization_trade_signal() {
        // Test TradeSignal serialization round-trip
        let signal = TradeSignal {
            action: sankey_copier_zmq::TradeAction::Close,
            ticket: 99999,
            symbol: Some("GBPJPY".to_string()),
            order_type: Some(sankey_copier_zmq::OrderType::Sell),
            lots: Some(0.5),
            open_price: Some(185.500),
            stop_loss: Some(186.000),
            take_profit: Some(184.500),
            magic_number: Some(12345),
            comment: Some("Closing trade".to_string()),
            timestamp: Utc::now(),
            source_account: "MASTER_CLOSE".to_string(),
            close_ratio: Some(0.5),
        };

        let bytes = rmp_serde::to_vec_named(&signal).unwrap();

        // Should be non-empty
        assert!(!bytes.is_empty());

        // Should deserialize back correctly
        let decoded: TradeSignal = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.ticket, signal.ticket);
        assert_eq!(decoded.symbol, signal.symbol);
        assert_eq!(decoded.lots, signal.lots);
        assert_eq!(decoded.close_ratio, signal.close_ratio);
    }
}
