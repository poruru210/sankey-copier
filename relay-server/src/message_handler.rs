use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

use crate::{
    connection_manager::ConnectionManager,
    db::Database,
    engine::CopyEngine,
    models::{
        ConfigMessage, CopySettings, HeartbeatMessage, RequestConfigMessage, SlaveConfigWithMaster,
        SymbolConverter, TradeSignal, UnregisterMessage,
    },
    zeromq::{ZmqConfigPublisher, ZmqMessage, ZmqSender},
};
use sankey_copier_zmq::MasterConfigMessage;

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

    /// Handle configuration request from Master or Slave EA
    async fn handle_request_config(&self, msg: RequestConfigMessage) {
        let account_id = msg.account_id.clone();

        tracing::info!(
            "Config request received from: {} (ea_type: {})",
            account_id,
            msg.ea_type
        );

        // Route to appropriate handler based on EA type
        match msg.ea_type.as_str() {
            "Master" => self.handle_master_config_request(&account_id).await,
            "Slave" => self.handle_slave_config_request(&account_id).await,
            _ => {
                tracing::warn!(
                    "Config request rejected: account {} sent request with unknown ea_type '{}'",
                    account_id,
                    msg.ea_type
                );
            }
        }
    }

    /// Handle configuration request from Master EA
    async fn handle_master_config_request(&self, account_id: &str) {
        match self.db.get_settings_for_master(account_id).await {
            Ok(master_settings) => {
                let config = MasterConfigMessage {
                    account_id: account_id.to_string(),
                    symbol_prefix: master_settings.symbol_prefix,
                    symbol_suffix: master_settings.symbol_suffix,
                    config_version: master_settings.config_version,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                };

                // Send Master CONFIG via MessagePack
                if let Err(e) = self.config_sender.send_master_config(&config).await {
                    tracing::error!("Failed to send master config to {}: {}", account_id, e);
                } else {
                    tracing::info!(
                        "Successfully sent Master CONFIG to: {} (version: {})",
                        account_id,
                        config.config_version
                    );
                }
            }
            Err(e) => {
                tracing::error!("Failed to get master settings for {}: {}", account_id, e);
            }
        }
    }

    /// Handle configuration request from Slave EA
    async fn handle_slave_config_request(&self, account_id: &str) {
        match self.db.get_settings_for_slave(account_id).await {
            Ok(settings_list) => {
                if settings_list.is_empty() {
                    tracing::info!(
                        "No configuration found for {}. EA will wait for Web UI configuration.",
                        account_id
                    );
                    return;
                }

                for settings in settings_list {
                    tracing::info!(
                        "Found settings for {}: master={}, db_status={}, lot_mult={:?}",
                        account_id,
                        settings.master_account,
                        settings.status,
                        settings.slave_settings.lot_multiplier
                    );

                    // Calculate effective status based on Master's is_trade_allowed
                    let effective_status = if settings.status == 0 {
                        // User disabled -> DISABLED
                        0
                    } else {
                        // User enabled (status == 1)
                        // Check if Master is connected and has trading allowed
                        let master_conn = self
                            .connection_manager
                            .get_ea(&settings.master_account)
                            .await;

                        if let Some(conn) = master_conn {
                            if conn.is_trade_allowed {
                                // Master online && trading allowed -> CONNECTED
                                2
                            } else {
                                // Master online but trading NOT allowed -> ENABLED
                                1
                            }
                        } else {
                            // Master offline -> ENABLED
                            1
                        }
                    };

                    // Build ConfigMessage with calculated effective status
                    let config = ConfigMessage {
                        account_id: settings.slave_account.clone(),
                        master_account: settings.master_account.clone(),
                        trade_group_id: settings.master_account.clone(),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                        status: effective_status,
                        lot_multiplier: settings.slave_settings.lot_multiplier,
                        reverse_trade: settings.slave_settings.reverse_trade,
                        symbol_mappings: settings.slave_settings.symbol_mappings.clone(),
                        filters: settings.slave_settings.filters.clone(),
                        config_version: settings.slave_settings.config_version,
                        symbol_prefix: settings.slave_settings.symbol_prefix.clone(),
                        symbol_suffix: settings.slave_settings.symbol_suffix.clone(),
                    };

                    // Send CONFIG via MessagePack
                    if let Err(e) = self.config_sender.send_config(&config).await {
                        tracing::error!("Failed to send config to {}: {}", account_id, e);
                    } else {
                        tracing::info!(
                            "Successfully sent CONFIG to: {} (db_status: {}, effective_status: {})",
                            account_id,
                            settings.status,
                            effective_status
                        );
                    }
                }
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

    /// Handle heartbeat messages (auto-registration + health monitoring + is_trade_allowed notification)
    async fn handle_heartbeat(&self, msg: HeartbeatMessage) {
        let account_id = msg.account_id.clone();
        let balance = msg.balance;
        let equity = msg.equity;
        let ea_type = msg.ea_type.clone();
        let new_is_trade_allowed = msg.is_trade_allowed;

        // Get old is_trade_allowed before updating
        let old_is_trade_allowed = self
            .connection_manager
            .get_ea(&account_id)
            .await
            .map(|conn| conn.is_trade_allowed);

        // Update heartbeat (performs auto-registration if needed)
        self.connection_manager.update_heartbeat(msg).await;

        // If this is a Master EA, check for is_trade_allowed changes
        if ea_type == "Master" {
            // Detect is_trade_allowed change
            let trade_allowed_changed = old_is_trade_allowed != Some(new_is_trade_allowed);

            if trade_allowed_changed {
                tracing::info!(
                    "Master {} is_trade_allowed changed: {:?} -> {}",
                    account_id,
                    old_is_trade_allowed,
                    new_is_trade_allowed
                );

                // Resend Config to all Slave accounts connected to this Master
                match self.db.get_members(&account_id).await {
                    Ok(members) => {
                        for member in members {
                            // Only send to enabled Slaves (status > 0)
                            if member.status > 0 {
                                // Build Config with calculated effective status
                                let effective_status = if new_is_trade_allowed { 2 } else { 1 };

                                let config = ConfigMessage {
                                    account_id: member.slave_account.clone(),
                                    master_account: account_id.clone(),
                                    trade_group_id: account_id.clone(),
                                    timestamp: chrono::Utc::now().to_rfc3339(),
                                    status: effective_status,
                                    lot_multiplier: member.slave_settings.lot_multiplier,
                                    reverse_trade: member.slave_settings.reverse_trade,
                                    symbol_mappings: member.slave_settings.symbol_mappings.clone(),
                                    filters: member.slave_settings.filters.clone(),
                                    config_version: member.slave_settings.config_version,
                                    symbol_prefix: member.slave_settings.symbol_prefix.clone(),
                                    symbol_suffix: member.slave_settings.symbol_suffix.clone(),
                                };

                                if let Err(e) = self.config_sender.send_config(&config).await {
                                    tracing::error!(
                                        "Failed to send config to {} due to Master is_trade_allowed change: {}",
                                        member.slave_account,
                                        e
                                    );
                                } else {
                                    tracing::info!(
                                        "Sent config to {} (effective_status: {}) due to Master {} is_trade_allowed change: {}",
                                        member.slave_account,
                                        effective_status,
                                        account_id,
                                        new_is_trade_allowed
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to get members for Master {}: {}", account_id, e);
                    }
                }
            }

            // Update all enabled settings to CONNECTED (status=2)
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
                    // We need to broadcast the updated settings for all affected slaves
                    if let Ok(members) = self.db.get_members(&account_id).await {
                        for member in members {
                            let settings_with_master = SlaveConfigWithMaster {
                                master_account: account_id.clone(),
                                slave_account: member.slave_account.clone(),
                                status: member.status,
                                slave_settings: member.slave_settings.clone(),
                            };
                            if let Ok(json) = serde_json::to_string(&settings_with_master) {
                                let _ =
                                    self.broadcast_tx.send(format!("settings_updated:{}", json));
                            }
                        }
                    }
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
            symbol_prefix: None,
            symbol_suffix: None,
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
            symbol_prefix: None,
            symbol_suffix: None,
            symbol_map: None,
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
            symbol_prefix: None,
            symbol_suffix: None,
            symbol_map: None,
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

    #[tokio::test]
    async fn test_handle_request_config_master() {
        use crate::models::RequestConfigMessage;

        let handler = create_test_handler().await;
        let master_account = "MASTER_TEST_001".to_string();

        // Step 1: Create TradeGroup in DB with default Master settings
        handler
            .db
            .create_trade_group(&master_account)
            .await
            .expect("Failed to create trade group");

        // Step 2: Create RequestConfig message with ea_type="Master"
        let request_msg = RequestConfigMessage {
            message_type: "RequestConfig".to_string(),
            account_id: master_account.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            ea_type: "Master".to_string(),
        };

        // Step 3: Call handle_request_config via handle_message
        let zmq_msg = crate::zeromq::ZmqMessage::RequestConfig(request_msg);
        handler.handle_message(zmq_msg).await;

        // Step 4: Verify no panic occurred (implementation will be added in Phase 3.2b)
        // In Red phase, this test logs warning because Master EA type is rejected
        // In Green phase, this test should pass after implementing Master config logic
    }

    #[tokio::test]
    async fn test_handle_request_config_master_not_found() {
        use crate::models::RequestConfigMessage;

        let handler = create_test_handler().await;
        let master_account = "NONEXISTENT_MASTER".to_string();

        // Create RequestConfig message for non-existent Master
        let request_msg = RequestConfigMessage {
            message_type: "RequestConfig".to_string(),
            account_id: master_account.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            ea_type: "Master".to_string(),
        };

        // Call handle_request_config via handle_message
        let zmq_msg = crate::zeromq::ZmqMessage::RequestConfig(request_msg);
        handler.handle_message(zmq_msg).await;

        // Should not panic even if Master not found (graceful handling)
    }

    #[tokio::test]
    async fn test_handle_request_config_slave() {
        use crate::models::{CopySettings, RequestConfigMessage, TradeFilters};

        let handler = create_test_handler().await;
        let master_account = "MASTER123".to_string();
        let slave_account = "SLAVE456".to_string();

        // Create and save a copy setting for the slave
        let settings = CopySettings {
            id: 0, // New record
            master_account: master_account.clone(),
            slave_account: slave_account.clone(),
            status: 1,
            lot_multiplier: Some(2.0),
            reverse_trade: false,
            symbol_prefix: Some("pro.".to_string()),
            symbol_suffix: Some(".m".to_string()),
            symbol_mappings: vec![],
            filters: TradeFilters::default(),
        };
        handler.db.save_copy_settings(&settings).await.unwrap();

        // Create RequestConfig message for Slave
        let request_msg = RequestConfigMessage {
            message_type: "RequestConfig".to_string(),
            account_id: slave_account.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            ea_type: "Slave".to_string(),
        };

        // Call handle_request_config via handle_message
        let zmq_msg = crate::zeromq::ZmqMessage::RequestConfig(request_msg);
        handler.handle_message(zmq_msg).await;

        // Should successfully send config to Slave (no panic)
    }

    #[tokio::test]
    async fn test_handle_request_config_slave_not_found() {
        use crate::models::RequestConfigMessage;

        let handler = create_test_handler().await;
        let slave_account = "NONEXISTENT_SLAVE".to_string();

        // Create RequestConfig message for non-existent Slave
        let request_msg = RequestConfigMessage {
            message_type: "RequestConfig".to_string(),
            account_id: slave_account.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            ea_type: "Slave".to_string(),
        };

        // Call handle_request_config via handle_message
        let zmq_msg = crate::zeromq::ZmqMessage::RequestConfig(request_msg);
        handler.handle_message(zmq_msg).await;

        // Should not panic even if Slave not found (graceful handling)
    }

    #[tokio::test]
    async fn test_handle_request_config_unknown_ea_type() {
        use crate::models::RequestConfigMessage;

        let handler = create_test_handler().await;

        // Create RequestConfig message with unknown EA type
        let request_msg = RequestConfigMessage {
            message_type: "RequestConfig".to_string(),
            account_id: "TEST123".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            ea_type: "UnknownType".to_string(), // Invalid EA type
        };

        // Call handle_request_config via handle_message
        let zmq_msg = crate::zeromq::ZmqMessage::RequestConfig(request_msg);
        handler.handle_message(zmq_msg).await;

        // Should handle gracefully (log warning, no panic)
    }
}
