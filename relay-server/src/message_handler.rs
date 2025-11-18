use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

use crate::{
    connection_manager::ConnectionManager,
    db::Database,
    engine::CopyEngine,
    models::{
        ConfigMessage, CopySettings, HeartbeatMessage, RequestConfigMessage, SymbolConverter,
        TradeSignal, UnregisterMessage,
    },
    zeromq::{ZmqConfigPublisher, ZmqMessage, ZmqSender},
};

/// Handles incoming ZMQ messages and coordinates trade copying logic
pub struct MessageHandler {
    connection_manager: Arc<ConnectionManager>,
    copy_engine: Arc<CopyEngine>,
    zmq_sender: Arc<ZmqSender>,
    settings_cache: Arc<RwLock<Vec<CopySettings>>>,
    broadcast_tx: broadcast::Sender<String>,
    db: Arc<Database>,
    config_sender: Arc<ZmqConfigPublisher>,
}

impl MessageHandler {
    pub fn new(
        connection_manager: Arc<ConnectionManager>,
        copy_engine: Arc<CopyEngine>,
        zmq_sender: Arc<ZmqSender>,
        settings_cache: Arc<RwLock<Vec<CopySettings>>>,
        broadcast_tx: broadcast::Sender<String>,
        db: Arc<Database>,
        config_sender: Arc<ZmqConfigPublisher>,
    ) -> Self {
        Self {
            connection_manager,
            copy_engine,
            zmq_sender,
            settings_cache,
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
        }
    }

    /// Handle configuration request from Slave EA
    async fn handle_request_config(&self, msg: RequestConfigMessage) {
        let account_id = msg.account_id.clone();

        tracing::info!("Config request received from: {}", account_id);

        match self.db.get_settings_for_slave(&account_id).await {
            Ok(Some(settings)) => {
                tracing::info!(
                    "Found settings for {}: master={}, status={}, lot_mult={:?}",
                    account_id,
                    settings.master_account,
                    settings.status,
                    settings.lot_multiplier
                );

                // Convert CopySettings to ConfigMessage
                let config: ConfigMessage = settings.into();

                // Send CONFIG via MessagePack
                if let Err(e) = self.config_sender.send_config(&config).await {
                    tracing::error!("Failed to send config to {}: {}", account_id, e);
                } else {
                    tracing::info!("Successfully sent CONFIG to: {}", account_id);
                }
            }
            Ok(None) => {
                tracing::info!(
                    "No configuration found for {}. EA will wait for Web UI configuration.",
                    account_id
                );
            }
            Err(e) => {
                tracing::error!("Failed to query settings for {}: {}", account_id, e);
            }
        }
    }

    /// Handle EA unregistration
    async fn handle_unregister(&self, msg: UnregisterMessage) {
        let account_id = &msg.account_id;
        self.connection_manager.unregister_ea(account_id).await;

        // Notify WebSocket clients
        let _ = self
            .broadcast_tx
            .send(format!("ea_disconnected:{}", account_id));
    }

    /// Handle heartbeat messages (auto-registration + health monitoring only)
    async fn handle_heartbeat(&self, msg: HeartbeatMessage) {
        let account_id = msg.account_id.clone();
        let balance = msg.balance;
        let equity = msg.equity;
        let ea_type = msg.ea_type.clone();

        // Update heartbeat (performs auto-registration if needed)
        self.connection_manager.update_heartbeat(msg).await;

        // If this is a Master EA, update all enabled settings to CONNECTED (status=2)
        if ea_type == "Master" {
            match self.db.update_master_statuses_connected(&account_id).await {
                Ok(count) if count > 0 => {
                    tracing::info!(
                        "Master {} connected: updated {} settings to CONNECTED",
                        account_id,
                        count
                    );
                    // Refresh settings cache to reflect the status change
                    if let Ok(settings) = self.db.list_copy_settings().await {
                        let mut cache = self.settings_cache.write().await;
                        *cache = settings;
                    }
                    // Notify WebSocket clients
                    let _ = self
                        .broadcast_tx
                        .send(format!("master_connected:{}", account_id));
                }
                Ok(_) => {
                    // No settings updated (no enabled settings for this master)
                }
                Err(e) => {
                    tracing::error!("Failed to update master statuses for {}: {}", account_id, e);
                }
            }
        }

        // Notify WebSocket clients of heartbeat
        let _ = self.broadcast_tx.send(format!(
            "ea_heartbeat:{}:{:.2}:{:.2}",
            account_id, balance, equity
        ));
    }

    /// Handle trade signals and process copying
    async fn handle_trade_signal(&self, signal: TradeSignal) {
        tracing::info!("Processing trade signal: {:?}", signal);

        // Notify WebSocket clients
        let _ = self.broadcast_tx.send(format!(
            "trade_received:{}:{}:{}",
            signal.source_account, signal.symbol, signal.lots
        ));

        let settings = self.settings_cache.read().await;

        for setting in settings.iter() {
            // Check if this signal is from the master account for this setting
            if signal.source_account != setting.master_account {
                continue;
            }

            // Apply filters
            if !self.copy_engine.should_copy_trade(&signal, setting) {
                tracing::debug!(
                    "Trade filtered out for slave account: {}",
                    setting.slave_account
                );
                continue;
            }

            // Process the trade copy
            self.process_trade_copy(&signal, setting).await;
        }
    }

    /// Process a single trade copy for a specific setting
    async fn process_trade_copy(&self, signal: &TradeSignal, setting: &CopySettings) {
        // Transform signal
        let converter = SymbolConverter {
            prefix_remove: None,
            suffix_remove: None,
            prefix_add: None,
            suffix_add: None,
        };

        match self
            .copy_engine
            .transform_signal(signal.clone(), setting, &converter)
        {
            Ok(transformed) => {
                tracing::info!(
                    "Copying trade to {}: {} {} lots",
                    setting.slave_account,
                    transformed.symbol,
                    transformed.lots
                );

                // Send to trade group using PUB/SUB with master_account as topic
                // This allows multiple slaves to subscribe to the same master's trades
                if let Err(e) = self
                    .zmq_sender
                    .send_signal(&setting.master_account, &transformed)
                    .await
                {
                    tracing::error!("Failed to send signal to trade group: {}", e);
                } else {
                    tracing::debug!(
                        "Sent signal to trade group '{}' for slave '{}'",
                        setting.master_account,
                        setting.slave_account
                    );

                    // Notify WebSocket clients
                    let _ = self.broadcast_tx.send(format!(
                        "trade_copied:{}:{}:{}:{}",
                        setting.slave_account, transformed.symbol, transformed.lots, setting.id
                    ));
                }
            }
            Err(e) => {
                tracing::error!("Failed to transform signal: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        CopySettings, HeartbeatMessage, OrderType, TradeAction, TradeFilters, UnregisterMessage,
    };
    use chrono::Utc;

    async fn create_test_handler() -> MessageHandler {
        use std::sync::atomic::{AtomicU16, Ordering};
        static PORT_COUNTER: AtomicU16 = AtomicU16::new(7000);

        let connection_manager = Arc::new(ConnectionManager::new(30));
        let copy_engine = Arc::new(CopyEngine::new());

        // Use unique port for each test to avoid "Address in use" errors
        let port = PORT_COUNTER.fetch_add(1, Ordering::SeqCst);
        let zmq_sender = Arc::new(ZmqSender::new(&format!("tcp://127.0.0.1:{}", port)).unwrap());

        let settings_cache = Arc::new(RwLock::new(Vec::new()));
        let (broadcast_tx, _) = broadcast::channel::<String>(100);

        // Create test database (in-memory)
        let db = Arc::new(Database::new("sqlite::memory:").await.unwrap());

        // Create ZmqConfigPublisher for tests
        let config_port = PORT_COUNTER.fetch_add(1, Ordering::SeqCst);
        let config_sender =
            Arc::new(ZmqConfigPublisher::new(&format!("tcp://127.0.0.1:{}", config_port)).unwrap());

        MessageHandler::new(
            connection_manager,
            copy_engine,
            zmq_sender,
            settings_cache,
            broadcast_tx,
            db,
            config_sender,
        )
    }

    fn create_test_trade_signal() -> TradeSignal {
        TradeSignal {
            action: TradeAction::Open,
            ticket: 12345,
            symbol: "EURUSD".to_string(),
            order_type: OrderType::Buy,
            lots: 0.1,
            open_price: 1.1000,
            stop_loss: Some(1.0950),
            take_profit: Some(1.1050),
            magic_number: 0,
            comment: "Test trade".to_string(),
            timestamp: Utc::now(),
            source_account: "MASTER_001".to_string(),
        }
    }

    fn create_test_copy_settings() -> CopySettings {
        CopySettings {
            id: 1,
            status: 2, // STATUS_CONNECTED
            master_account: "MASTER_001".to_string(),
            slave_account: "SLAVE_001".to_string(),
            lot_multiplier: Some(1.0),
            reverse_trade: false,
            symbol_mappings: vec![],
            filters: TradeFilters {
                allowed_symbols: None,
                blocked_symbols: None,
                allowed_magic_numbers: None,
                blocked_magic_numbers: None,
            },
        }
    }

    #[tokio::test]
    async fn test_handle_unregister() {
        let handler = create_test_handler().await;
        let account_id = "TEST_001".to_string();

        // First auto-register via heartbeat
        let hb_msg = HeartbeatMessage {
            message_type: "Heartbeat".to_string(),
            account_id: account_id.clone(),
            balance: 10000.0,
            equity: 10000.0,
            open_positions: 0,
            timestamp: chrono::Utc::now().to_rfc3339(),
            version: "test".to_string(),
            ea_type: "Master".to_string(),
            platform: "MT4".to_string(),
            account_number: 12345,
            broker: "Test Broker".to_string(),
            account_name: "Test Account".to_string(),
            server: "Test-Server".to_string(),
            currency: "USD".to_string(),
            leverage: 100,
            is_trade_allowed: true,
        };
        handler.handle_heartbeat(hb_msg).await;

        // Then unregister
        handler
            .handle_unregister(UnregisterMessage {
                message_type: "Unregister".to_string(),
                account_id: account_id.clone(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            })
            .await;

        // Verify EA status is Offline
        let ea = handler.connection_manager.get_ea(&account_id).await;
        assert!(ea.is_some());
        assert_eq!(ea.unwrap().status, crate::models::ConnectionStatus::Offline);
    }

    #[tokio::test]
    async fn test_handle_heartbeat() {
        let handler = create_test_handler().await;
        let account_id = "TEST_001".to_string();

        // Send heartbeat (auto-registration)
        let hb_msg = HeartbeatMessage {
            message_type: "Heartbeat".to_string(),
            account_id: account_id.clone(),
            balance: 12000.0,
            equity: 11500.0,
            open_positions: 3,
            timestamp: chrono::Utc::now().to_rfc3339(),
            version: "test".to_string(),
            ea_type: "Master".to_string(),
            platform: "MT4".to_string(),
            account_number: 12345,
            broker: "Test Broker".to_string(),
            account_name: "Test Account".to_string(),
            server: "Test-Server".to_string(),
            currency: "USD".to_string(),
            leverage: 100,
            is_trade_allowed: true,
        };
        handler.handle_heartbeat(hb_msg).await;

        // Verify EA was auto-registered with correct balance and equity
        let ea = handler.connection_manager.get_ea(&account_id).await;
        assert!(ea.is_some());
        let ea = ea.unwrap();
        assert_eq!(ea.balance, 12000.0);
        assert_eq!(ea.equity, 11500.0);
        assert_eq!(ea.status, crate::models::ConnectionStatus::Online);
    }

    #[tokio::test]
    async fn test_handle_trade_signal_with_matching_setting() {
        let handler = create_test_handler().await;
        let signal = create_test_trade_signal();
        let settings = create_test_copy_settings();

        // Add settings to cache
        {
            let mut cache = handler.settings_cache.write().await;
            cache.push(settings);
        }

        // Process trade signal (should not panic)
        handler.handle_trade_signal(signal).await;
    }

    #[tokio::test]
    async fn test_handle_trade_signal_no_matching_master() {
        let handler = create_test_handler().await;
        let mut signal = create_test_trade_signal();
        signal.source_account = "OTHER_MASTER".to_string();
        let settings = create_test_copy_settings();

        // Add settings to cache
        {
            let mut cache = handler.settings_cache.write().await;
            cache.push(settings);
        }

        // Process trade signal (should be filtered out, no panic)
        handler.handle_trade_signal(signal).await;
    }

    #[tokio::test]
    async fn test_handle_trade_signal_disabled_setting() {
        let handler = create_test_handler().await;
        let signal = create_test_trade_signal();
        let mut settings = create_test_copy_settings();
        settings.status = 0; // STATUS_DISABLED

        // Add settings to cache
        {
            let mut cache = handler.settings_cache.write().await;
            cache.push(settings);
        }

        // Process trade signal (should be filtered out, no panic)
        handler.handle_trade_signal(signal).await;
    }
}
