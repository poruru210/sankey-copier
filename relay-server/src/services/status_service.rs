use crate::models::HeartbeatMessage;
use crate::ports::outbound::{
    ConfigPublisher, ConnectionManager, StatusEvaluator, TradeGroupRepository, UpdateBroadcaster,
};
use std::sync::Arc;

#[allow(dead_code)] // Fields will be used in Phase 2 implementation
pub struct StatusService {
    connection_manager: Arc<dyn ConnectionManager>,
    repository: Arc<dyn TradeGroupRepository>,
    publisher: Arc<dyn ConfigPublisher>,
    status_evaluator: Option<Arc<dyn StatusEvaluator>>,
    broadcaster: Option<Arc<dyn UpdateBroadcaster>>,
}

impl StatusService {
    pub fn new(
        connection_manager: Arc<dyn ConnectionManager>,
        repository: Arc<dyn TradeGroupRepository>,
        publisher: Arc<dyn ConfigPublisher>,
        status_evaluator: Option<Arc<dyn StatusEvaluator>>,
        broadcaster: Option<Arc<dyn UpdateBroadcaster>>,
    ) -> Self {
        Self {
            connection_manager,
            repository,
            publisher,
            status_evaluator,
            broadcaster,
        }
    }

    pub async fn handle_heartbeat(&self, msg: HeartbeatMessage) {
        let account_id = msg.account_id.clone();
        let ea_type = msg.ea_type.clone();

        tracing::info!(
            account = %account_id,
            ea_type = %ea_type,
            is_trade_allowed = msg.is_trade_allowed,
            "[StatusService] Processing heartbeat"
        );

        // Get old connection info before updating
        let old_conn = if ea_type == "Master" {
            self.connection_manager.get_master(&account_id).await
        } else {
            self.connection_manager.get_slave(&account_id).await
        };

        // Update heartbeat
        self.connection_manager.update_heartbeat(msg.clone()).await;

        match ea_type.as_str() {
            "Master" => self.handle_master_heartbeat(msg, old_conn).await,
            "Slave" => self.handle_slave_heartbeat(msg, old_conn).await,
            _ => {
                tracing::warn!("Unknown EA type for heartbeat: {}", ea_type);
            }
        }
    }

    async fn handle_master_heartbeat(
        &self,
        msg: HeartbeatMessage,
        old_conn: Option<crate::models::EaConnection>,
    ) {
        let account_id = &msg.account_id;

        // Get TradeGroup
        let trade_group = match self.repository.get_trade_group(account_id).await {
            Ok(Some(tg)) => tg,
            Ok(None) => return,
            Err(e) => {
                tracing::error!("Failed to get TradeGroup for {}: {}", account_id, e);
                return;
            }
        };

        // Get Master Connection (after update_heartbeat)
        let master_conn = self.connection_manager.get_master(account_id).await;

        // Build Master Config & Status
        let context = crate::config_builder::MasterConfigContext {
            account_id: account_id.clone(),
            intent: crate::models::status_engine::MasterIntent {
                web_ui_enabled: trade_group.master_settings.enabled,
            },
            connection_snapshot: crate::models::status_engine::ConnectionSnapshot {
                connection_status: master_conn.as_ref().map(|c| c.status),
                is_trade_allowed: msg.is_trade_allowed,
            },
            settings: &trade_group.master_settings,
            timestamp: chrono::Utc::now(),
        };

        let bundle = crate::config_builder::ConfigBuilder::build_master_config(context);

        // Calculate OLD Master Status
        let old_master_status = if let Some(conn) = old_conn.as_ref() {
            let old_snapshot = crate::models::status_engine::ConnectionSnapshot {
                connection_status: Some(conn.status),
                is_trade_allowed: conn.is_trade_allowed,
            };
            crate::models::status_engine::evaluate_master_status(
                crate::models::status_engine::MasterIntent {
                    web_ui_enabled: trade_group.master_settings.enabled,
                },
                old_snapshot,
            )
        } else {
            // New registration - treat as "unknown" state which will trigger change
            crate::models::status_engine::MasterStatusResult::unknown()
        };

        // Check for changes
        let master_changed = bundle.status_result.has_changed(&old_master_status);

        tracing::debug!(
            master = %account_id,
            changed = master_changed,
            old_status = old_master_status.status,
            new_status = bundle.status_result.status,
            "[StatusService] Master status change detection"
        );

        if master_changed {
            // Send config
            if let Err(e) = self.publisher.send_master_config(&bundle.config).await {
                tracing::error!("Failed to send master config to {}: {}", account_id, e);
            } else {
                tracing::info!(
                    "Sent Master CONFIG to {} (status: {})",
                    account_id,
                    bundle.status_result.status
                );
            }

            // TODO: Notify connected Slaves about Master status change
        }
    }

    async fn handle_slave_heartbeat(
        &self,
        msg: HeartbeatMessage,
        old_conn: Option<crate::models::EaConnection>,
    ) {
        let slave_account = &msg.account_id;

        // Get all settings for this slave (one per Master connection)
        let settings_list = match self.repository.get_settings_for_slave(slave_account).await {
            Ok(list) => list,
            Err(err) => {
                tracing::error!(
                    "Failed to fetch settings for Slave {} during heartbeat: {}",
                    slave_account,
                    err
                );
                return;
            }
        };

        if settings_list.is_empty() {
            tracing::debug!(
                "Skipping Slave {} heartbeat runtime evaluation (no trade groups)",
                slave_account
            );
            return;
        }

        // Need StatusEvaluator to proceed
        let Some(evaluator) = &self.status_evaluator else {
            tracing::debug!(
                "StatusEvaluator not configured, skipping Slave {} heartbeat evaluation",
                slave_account
            );
            return;
        };

        for settings in settings_list {
            // Build new bundle
            let slave_bundle = evaluator
                .build_slave_bundle(crate::models::status_engine::SlaveRuntimeTarget {
                    master_account: settings.master_account.as_str(),
                    trade_group_id: settings.master_account.as_str(),
                    slave_account: &settings.slave_account,
                    enabled_flag: settings.enabled_flag,
                    slave_settings: &settings.slave_settings,
                })
                .await;

            // Calculate OLD status result
            let old_status_result = if let Some(conn) = old_conn.as_ref() {
                let old_snapshot = crate::models::status_engine::ConnectionSnapshot {
                    connection_status: Some(conn.status),
                    is_trade_allowed: conn.is_trade_allowed,
                };
                evaluator
                    .evaluate_member_runtime_status_with_snapshot(
                        crate::models::status_engine::SlaveRuntimeTarget {
                            master_account: settings.master_account.as_str(),
                            trade_group_id: settings.master_account.as_str(),
                            slave_account: &settings.slave_account,
                            enabled_flag: settings.enabled_flag,
                            slave_settings: &settings.slave_settings,
                        },
                        old_snapshot,
                    )
                    .await
            } else {
                crate::models::status_engine::MemberStatusResult::unknown()
            };

            let previous_status = settings.status;
            let evaluated_status = slave_bundle.status_result.status;

            // Detect changes
            let state_changed = slave_bundle.status_result.has_changed(&old_status_result)
                || previous_status != evaluated_status;

            tracing::debug!(
                slave = %settings.slave_account,
                master = %settings.master_account,
                changed = state_changed,
                old_status = previous_status,
                new_status = evaluated_status,
                "[StatusService] Slave status change detection"
            );

            if state_changed {
                tracing::info!(
                    slave = %settings.slave_account,
                    master = %settings.master_account,
                    old_status = previous_status,
                    new_status = evaluated_status,
                    "Slave state changed via heartbeat"
                );

                // Send config
                if let Err(err) = self.publisher.send_slave_config(&slave_bundle.config).await {
                    tracing::error!(
                        "Failed to broadcast config to Slave {} on heartbeat: {}",
                        settings.slave_account,
                        err
                    );
                }

                // WebSocket broadcast
                if let Some(broadcaster) = &self.broadcaster {
                    broadcaster.broadcast_snapshot().await;
                }
            }

            // Update DB status
            if let Err(err) = self
                .repository
                .update_member_runtime_status(
                    &settings.master_account,
                    slave_account,
                    evaluated_status,
                )
                .await
            {
                tracing::error!(
                    "Failed to persist runtime status for Slave {} (master {}): {}",
                    settings.slave_account,
                    settings.master_account,
                    err
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::status_engine::{MemberStatusResult, SlaveRuntimeTarget};
    use crate::models::{ConnectionStatus, EaConnection};
    use crate::models::{SlaveSettings, TradeGroup, TradeGroupMember, VLogsGlobalSettings};
    use async_trait::async_trait;
    use mockall::mock;
    use mockall::predicate;
    use mockall::predicate::*;
    use sankey_copier_zmq::{MasterConfigMessage, SlaveConfigMessage};

    // Mocks definition
    mock! {
        pub ConnectionManager {}
        #[async_trait]
        impl ConnectionManager for ConnectionManager {
            async fn get_master(&self, account_id: &str) -> Option<EaConnection>;
            async fn get_slave(&self, account_id: &str) -> Option<EaConnection>;
            async fn update_heartbeat(&self, msg: HeartbeatMessage);
        }
    }

    mock! {
        pub TradeGroupRepository {}
        #[async_trait]
        impl TradeGroupRepository for TradeGroupRepository {
            async fn get_trade_group(&self, id: &str) -> anyhow::Result<Option<TradeGroup>>;
            async fn get_members(&self, master_id: &str) -> anyhow::Result<Vec<TradeGroupMember>>;
            async fn get_settings_for_slave(&self, slave_id: &str) -> anyhow::Result<Vec<crate::models::SlaveConfigWithMaster>>;
            async fn update_member_runtime_status(&self, master_id: &str, slave_id: &str, status: i32) -> anyhow::Result<()>;
        }
    }

    mock! {
        pub ConfigPublisher {}
        #[async_trait]
        impl ConfigPublisher for ConfigPublisher {
            async fn send_master_config(&self, config: &MasterConfigMessage) -> anyhow::Result<()>;
            async fn send_slave_config(&self, config: &SlaveConfigMessage) -> anyhow::Result<()>;
            async fn broadcast_vlogs_config(&self, config: &VLogsGlobalSettings) -> anyhow::Result<()>;
        }
    }

    mock! {
        pub UpdateBroadcaster {}
        #[async_trait]
        impl UpdateBroadcaster for UpdateBroadcaster {
            async fn broadcast_snapshot(&self);
        }
    }

    struct MockStatusEvaluator;
    #[async_trait]
    impl StatusEvaluator for MockStatusEvaluator {
        async fn evaluate_member_runtime_status(
            &self,
            _target: SlaveRuntimeTarget<'_>,
        ) -> MemberStatusResult {
            MemberStatusResult::default()
        }

        async fn evaluate_member_runtime_status_with_snapshot(
            &self,
            _target: SlaveRuntimeTarget<'_>,
            _snapshot: crate::models::status_engine::ConnectionSnapshot,
        ) -> MemberStatusResult {
            MemberStatusResult::default()
        }

        async fn build_slave_bundle(
            &self,
            _target: SlaveRuntimeTarget<'_>,
        ) -> crate::config_builder::SlaveConfigBundle {
            // Return a default bundle for testing
            crate::config_builder::SlaveConfigBundle {
                config: sankey_copier_zmq::SlaveConfigMessage {
                    account_id: String::new(),
                    master_account: String::new(),
                    timestamp: 0,
                    trade_group_id: String::new(),
                    status: 0,
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
                    warning_codes: vec![],
                },
                status_result: MemberStatusResult::default(),
            }
        }
    }

    #[tokio::test]
    async fn test_handle_heartbeat_master_basic_flow() {
        let mut mock_conn_manager = MockConnectionManager::new();
        let mut mock_repo = MockTradeGroupRepository::new();
        let mut mock_publisher = MockConfigPublisher::new();

        let account_id = "MASTER_123";
        let heartbeat = HeartbeatMessage {
            account_id: account_id.to_string(),
            ea_type: "Master".to_string(),
            is_trade_allowed: true,
            message_type: "Heartbeat".to_string(),
            balance: 10000.0,
            equity: 10000.0,
            open_positions: 0,
            timestamp: "2023-01-01T00:00:00Z".to_string(),
            version: "1.0.0".to_string(),
            platform: "MT5".to_string(),
            account_number: 123456,
            broker: "TestBroker".to_string(),
            account_name: "TestAccount".to_string(),
            server: "TestServer".to_string(),
            currency: "USD".to_string(),
            leverage: 100,
            symbol_prefix: None,
            symbol_suffix: None,
            symbol_map: None,
        };

        // Expect heartbeat update
        mock_conn_manager
            .expect_update_heartbeat()
            .with(function(|msg: &HeartbeatMessage| {
                msg.account_id == "MASTER_123" && msg.ea_type == "Master" && msg.is_trade_allowed
            }))
            .times(1)
            .return_const(());

        // Initial connection lookup
        let existing_conn = EaConnection {
            account_id: account_id.to_string(),
            ea_type: crate::models::EaType::Master, // Fixed: use default or correct type
            status: ConnectionStatus::Registered,   // Simulate online/registered
            is_trade_allowed: true,
            ..Default::default()
        };

        mock_conn_manager
            .expect_get_master()
            .with(eq(account_id))
            .return_const(Some(existing_conn));

        // Use TradeGroup::new which exists
        let trade_group = TradeGroup::new(account_id.to_string());

        mock_repo
            .expect_get_trade_group()
            .with(eq(account_id))
            .return_once(|_| Ok(Some(trade_group)));

        let service = StatusService::new(
            Arc::new(mock_conn_manager),
            Arc::new(mock_repo),
            Arc::new(mock_publisher),
            None,
            None,
        );

        service.handle_heartbeat(heartbeat).await;
    }

    #[tokio::test]
    async fn test_handle_heartbeat_master_new_registration_sends_config() {
        let mut mock_conn_manager = MockConnectionManager::new();
        let mut mock_repo = MockTradeGroupRepository::new();
        let mut mock_publisher = MockConfigPublisher::new();

        let account_id = "MASTER_NEW";
        let heartbeat = HeartbeatMessage {
            account_id: account_id.to_string(),
            ea_type: "Master".to_string(),
            is_trade_allowed: true,
            message_type: "Heartbeat".to_string(),
            balance: 10000.0,
            equity: 10000.0,
            open_positions: 0,
            timestamp: "2023-01-01T00:00:00Z".to_string(),
            version: "1.0.0".to_string(),
            platform: "MT5".to_string(),
            account_number: 123456,
            broker: "TestBroker".to_string(),
            account_name: "TestAccount".to_string(),
            server: "TestServer".to_string(),
            currency: "USD".to_string(),
            leverage: 100,
            symbol_prefix: None,
            symbol_suffix: None,
            symbol_map: None,
        };

        // First call for old_conn lookup returns None (new registration)
        mock_conn_manager
            .expect_get_master()
            .with(eq(account_id))
            .times(1)
            .return_const(None);

        // Heartbeat update
        mock_conn_manager
            .expect_update_heartbeat()
            .times(1)
            .return_const(());

        // Second call after registration returns the connection
        let new_conn = EaConnection {
            account_id: account_id.to_string(),
            ea_type: crate::models::EaType::Master,
            status: ConnectionStatus::Registered,
            is_trade_allowed: true,
            ..Default::default()
        };
        mock_conn_manager
            .expect_get_master()
            .with(eq(account_id))
            .times(1)
            .return_const(Some(new_conn));

        // TradeGroup exists
        let trade_group = TradeGroup::new(account_id.to_string());
        mock_repo
            .expect_get_trade_group()
            .with(eq(account_id))
            .return_once(|_| Ok(Some(trade_group)));

        // EXPECT: Config should be published for new registration
        mock_publisher
            .expect_send_master_config()
            .times(1)
            .returning(|_| Ok(()));

        let service = StatusService::new(
            Arc::new(mock_conn_manager),
            Arc::new(mock_repo),
            Arc::new(mock_publisher),
            None,
            None,
        );

        service.handle_heartbeat(heartbeat).await;
    }
}
