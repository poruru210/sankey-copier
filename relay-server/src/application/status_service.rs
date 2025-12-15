use crate::domain::models::HeartbeatMessage;
use crate::ports::outbound::{
    ConfigPublisher, ConnectionManager, StatusEvaluator, TradeGroupRepository, UpdateBroadcaster,
    VLogsConfigProvider,
};
use std::sync::Arc;

#[allow(dead_code)] // Fields will be used in Phase 2 implementation
pub struct StatusService {
    connection_manager: Arc<dyn ConnectionManager>,
    repository: Arc<dyn TradeGroupRepository>,
    publisher: Arc<dyn ConfigPublisher>,
    status_evaluator: Option<Arc<dyn StatusEvaluator>>,
    broadcaster: Option<Arc<dyn UpdateBroadcaster>>,
    vlogs_provider: Option<Arc<dyn VLogsConfigProvider>>,
}

impl StatusService {
    pub fn new(
        connection_manager: Arc<dyn ConnectionManager>,
        repository: Arc<dyn TradeGroupRepository>,
        publisher: Arc<dyn ConfigPublisher>,
        status_evaluator: Option<Arc<dyn StatusEvaluator>>,
        broadcaster: Option<Arc<dyn UpdateBroadcaster>>,
        vlogs_provider: Option<Arc<dyn VLogsConfigProvider>>,
    ) -> Self {
        Self {
            connection_manager,
            repository,
            publisher,
            status_evaluator,
            broadcaster,
            vlogs_provider,
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
        let is_new_registration = self.connection_manager.update_heartbeat(msg.clone()).await;

        if is_new_registration {
            tracing::info!(
                account = %account_id,
                "New EA registration detected, sending VictoriaLogs config"
            );
            if let Some(provider) = &self.vlogs_provider {
                let config = provider.get_config();
                if let Err(e) = self.publisher.broadcast_vlogs_config(&config).await {
                    tracing::error!(
                        account = %account_id,
                        error = %e,
                        "Failed to send VictoriaLogs config to newly registered EA"
                    );
                }
            }
        }

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
        old_conn: Option<crate::domain::models::EaConnection>,
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
            intent: crate::domain::services::status_calculator::MasterIntent {
                web_ui_enabled: trade_group.master_settings.enabled,
            },
            connection_snapshot: crate::domain::services::status_calculator::ConnectionSnapshot {
                connection_status: master_conn.as_ref().map(|c| c.status),
                is_trade_allowed: msg.is_trade_allowed,
            },
            settings: &trade_group.master_settings,
            timestamp: chrono::Utc::now(),
        };

        let bundle = crate::config_builder::ConfigBuilder::build_master_config(context);

        // Calculate OLD Master Status
        let old_master_status = if let Some(conn) = old_conn.as_ref() {
            let old_snapshot = crate::domain::services::status_calculator::ConnectionSnapshot {
                connection_status: Some(conn.status),
                is_trade_allowed: conn.is_trade_allowed,
            };
            crate::domain::services::status_calculator::evaluate_master_status(
                crate::domain::services::status_calculator::MasterIntent {
                    web_ui_enabled: trade_group.master_settings.enabled,
                },
                old_snapshot,
            )
        } else {
            // New registration - treat as "unknown" state which will trigger change
            crate::domain::services::status_calculator::MasterStatusResult::unknown()
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
        }

        // Always update related Slaves when Master heartbeat is received
        // (even if Master status didn't change, Slaves need to be evaluated)
        self.update_related_slaves(account_id, &old_master_status, &bundle.status_result)
            .await;
    }

    /// Update all Slaves connected to this Master
    async fn update_related_slaves(
        &self,
        master_account: &str,
        old_master_status: &crate::domain::services::status_calculator::MasterStatusResult,
        _new_master_status: &crate::domain::services::status_calculator::MasterStatusResult,
    ) {
        let Some(evaluator) = &self.status_evaluator else {
            tracing::debug!(
                "StatusEvaluator not configured, skipping Slave updates for Master {}",
                master_account
            );
            return;
        };

        // Get all Slaves connected to this Master
        let members = match self.repository.get_members(master_account).await {
            Ok(members) => members,
            Err(e) => {
                tracing::error!("Failed to get members for Master {}: {}", master_account, e);
                return;
            }
        };

        // Track processed slaves to avoid duplicates
        let mut processed_slaves = std::collections::HashSet::new();
        let mut any_state_changed = false;

        for member in members {
            let slave_account = member.slave_account.clone();

            if processed_slaves.contains(&slave_account) {
                continue;
            }
            processed_slaves.insert(slave_account.clone());

            let target = crate::domain::services::status_calculator::SlaveRuntimeTarget {
                master_account,
                trade_group_id: master_account,
                slave_account: &slave_account,
                enabled_flag: member.enabled_flag,
                slave_settings: &member.slave_settings,
            };

            // Build new Slave bundle (uses current Master status)
            // Note: target cannot be cloned because it contains references, so we use it directly
            // For evaluate_member_status, we construct intents directly avoiding target reuse issues
            let slave_bundle = evaluator.build_slave_bundle(target).await;

            // Calculate OLD Slave status (uses OLD Master status)
            let slave_conn = self.connection_manager.get_slave(&slave_account).await;
            let slave_snapshot = crate::domain::services::status_calculator::ConnectionSnapshot {
                connection_status: slave_conn.as_ref().map(|c| c.status),
                is_trade_allowed: slave_conn
                    .as_ref()
                    .map(|c| c.is_trade_allowed)
                    .unwrap_or(false),
            };

            let old_slave_result =
                crate::domain::services::status_calculator::evaluate_member_status(
                    crate::domain::services::status_calculator::SlaveIntent {
                        web_ui_enabled: member.enabled_flag,
                    },
                    slave_snapshot,
                    old_master_status,
                );

            let slave_changed = slave_bundle.status_result.has_changed(&old_slave_result);

            if slave_changed {
                any_state_changed = true;
                tracing::info!(
                    slave = %slave_account,
                    master = %master_account,
                    old_status = old_slave_result.status,
                    new_status = slave_bundle.status_result.status,
                    "Slave state changed via Master heartbeat"
                );

                // Send config to Slave
                if let Err(err) = self.publisher.send_slave_config(&slave_bundle.config).await {
                    tracing::error!(
                        "Failed to broadcast config to Slave {} on Master heartbeat: {}",
                        slave_account,
                        err
                    );
                }
            }

            // Update DB status (always, to reflect current evaluation)
            if let Err(err) = self
                .repository
                .update_member_runtime_status(
                    master_account,
                    &slave_account,
                    slave_bundle.status_result.status,
                )
                .await
            {
                tracing::error!(
                    "Failed to persist runtime status for Slave {} (master {}): {}",
                    slave_account,
                    master_account,
                    err
                );
            }
        }

        // WebSocket broadcast for any status changes
        if any_state_changed {
            if let Some(broadcaster) = &self.broadcaster {
                broadcaster.broadcast_snapshot().await;
            }
        }
    }

    async fn handle_slave_heartbeat(
        &self,
        msg: HeartbeatMessage,
        old_conn: Option<crate::domain::models::EaConnection>,
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
                .build_slave_bundle(
                    crate::domain::services::status_calculator::SlaveRuntimeTarget {
                        master_account: settings.master_account.as_str(),
                        trade_group_id: settings.master_account.as_str(),
                        slave_account: &settings.slave_account,
                        enabled_flag: settings.enabled_flag,
                        slave_settings: &settings.slave_settings,
                    },
                )
                .await;

            // Calculate OLD status result
            let old_status_result = if let Some(conn) = old_conn.as_ref() {
                let old_snapshot = crate::domain::services::status_calculator::ConnectionSnapshot {
                    connection_status: Some(conn.status),
                    is_trade_allowed: conn.is_trade_allowed,
                };
                evaluator
                    .evaluate_member_runtime_status_with_snapshot(
                        crate::domain::services::status_calculator::SlaveRuntimeTarget {
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
                crate::domain::services::status_calculator::MemberStatusResult::unknown()
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
    use crate::domain::models::{ConnectionStatus, EaConnection};
    use crate::domain::models::{TradeGroup, TradeGroupMember, VLogsGlobalSettings};
    use crate::domain::services::status_calculator::{MemberStatusResult, SlaveRuntimeTarget};
    use async_trait::async_trait;
    use mockall::mock;

    use mockall::predicate::*;
    use sankey_copier_zmq::{MasterConfigMessage, SlaveConfigMessage};

    // Mocks definition
    mock! {
        pub ConnectionManager {}
        #[async_trait]
        impl ConnectionManager for ConnectionManager {
            async fn get_master(&self, account_id: &str) -> Option<EaConnection>;
            async fn get_slave(&self, account_id: &str) -> Option<EaConnection>;
            async fn update_heartbeat(&self, msg: HeartbeatMessage) -> bool;
        }
    }

    mock! {
        pub TradeGroupRepository {}
        #[async_trait]
        impl TradeGroupRepository for TradeGroupRepository {
            async fn get_trade_group(&self, id: &str) -> anyhow::Result<Option<TradeGroup>>;
            async fn get_members(&self, master_id: &str) -> anyhow::Result<Vec<TradeGroupMember>>;
            async fn get_settings_for_slave(&self, slave_id: &str) -> anyhow::Result<Vec<crate::domain::models::SlaveConfigWithMaster>>;
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

    mock! {
        pub VLogsConfigProvider {}
        impl VLogsConfigProvider for VLogsConfigProvider {
            fn get_config(&self) -> crate::domain::models::VLogsGlobalSettings;
        }
    }

    #[allow(dead_code)]
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
            _snapshot: crate::domain::services::status_calculator::ConnectionSnapshot,
        ) -> MemberStatusResult {
            MemberStatusResult::default()
        }

        async fn build_slave_bundle(
            &self,
            _target: SlaveRuntimeTarget<'_>,
        ) -> crate::config_builder::SlaveConfigBundle {
            let mut bundle = crate::config_builder::SlaveConfigBundle::default();
            bundle.status_result.status = 2; // Enabled
            bundle
        }
    }

    #[tokio::test]
    async fn test_handle_heartbeat_new_registration_sends_vlogs_config() {
        let mut mock_conn_manager = MockConnectionManager::new();

        let mut mock_publisher = MockConfigPublisher::new();
        let mut mock_vlogs_provider = MockVLogsConfigProvider::new();

        let account_id = "NEW_SLAVE_999";
        let heartbeat = HeartbeatMessage {
            account_id: account_id.to_string(),
            ea_type: "Slave".to_string(),
            is_trade_allowed: true,
            message_type: "Heartbeat".to_string(),
            balance: 500.0,
            equity: 500.0,
            open_positions: 0,
            timestamp: "2023-01-01T00:00:00Z".to_string(),
            version: "1.0.0".to_string(),
            platform: "MT5".to_string(),
            account_number: 999111,
            broker: "DemoBroker".to_string(),
            account_name: "Newbie".to_string(),
            server: "DemoServer".to_string(),
            currency: "USD".to_string(),
            leverage: 100,
            symbol_prefix: None,
            symbol_suffix: None,
            symbol_map: None,
        };

        // EXPECT: connection manager to return TRUE (is new)
        mock_conn_manager
            .expect_update_heartbeat()
            .with(mockall::predicate::function(
                |msg: &crate::domain::models::HeartbeatMessage| {
                    msg.account_id == "NEW_SLAVE_999" && msg.ea_type == "Slave"
                },
            ))
            .times(1)
            .returning(|_| true);

        // EXPECT: get_slave called (to check old status)
        mock_conn_manager
            .expect_get_slave()
            .with(mockall::predicate::eq(account_id))
            .returning(|_| None);

        // EXPECT: vlogs provider called
        let vlogs_settings = crate::domain::models::VLogsGlobalSettings {
            enabled: true,
            endpoint: "http://vlogs:8428".to_string(),
            batch_size: 1000,
            flush_interval_secs: 5,
            log_level: "INFO".to_string(),
        };
        let vlogs_settings_clone = vlogs_settings.clone();
        mock_vlogs_provider
            .expect_get_config()
            .times(1)
            .return_const(vlogs_settings);

        // EXPECT: publisher called with vlogs config
        mock_publisher
            .expect_broadcast_vlogs_config()
            .with(mockall::predicate::eq(vlogs_settings_clone))
            .times(1)
            .returning(|_| Ok(()));

        // Since we are not setting up the full flow (repo, evaluator), we might panic if handle_slave_heartbeat continues.
        // But in unit tests we control the flow.
        // StatusService splits logic.
        // We can just verify the first part.
        // But new() requires all dependencies.
        // Let's pass mock_repo which panics on use?
        // Or better, let's configure mock_repo to return error or empty so it finishes gracefully.

        // EXPECT: handle_slave_heartbeat -> repo.get_settings_for_slave
        // We simulate error or empty list to stop processing
        // But Mock by default panics.
        // Let's instantiate a repo that returns empty result.

        let mut safe_mock_repo = MockTradeGroupRepository::new();
        safe_mock_repo
            .expect_get_settings_for_slave()
            .returning(|_| Ok(vec![]));

        let service = StatusService::new(
            Arc::new(mock_conn_manager),
            Arc::new(safe_mock_repo),
            Arc::new(mock_publisher),
            None,
            None,
            Some(Arc::new(mock_vlogs_provider)),
        );

        service.handle_heartbeat(heartbeat).await;
    }

    #[tokio::test]
    async fn test_handle_heartbeat_master_basic_flow() {
        let mut mock_conn_manager = MockConnectionManager::new();
        let mut mock_repo = MockTradeGroupRepository::new();
        let mock_publisher = MockConfigPublisher::new();

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
            .return_const(false);

        // Initial connection lookup
        let existing_conn = EaConnection {
            account_id: account_id.to_string(),
            ea_type: crate::domain::models::EaType::Master, // Fixed: use default or correct type
            status: ConnectionStatus::Registered,           // Simulate online/registered
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
            .return_const(true);

        // Second call after registration returns the connection
        let new_conn = EaConnection {
            account_id: account_id.to_string(),
            ea_type: crate::domain::models::EaType::Master,
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
            None,
        );

        service.handle_heartbeat(heartbeat).await;
    }

    /// TDD Test: Slave heartbeat with status change sends config
    #[tokio::test]
    async fn test_handle_heartbeat_slave_state_change_sends_config() {
        let mut mock_conn_manager = MockConnectionManager::new();
        let mut mock_repo = MockTradeGroupRepository::new();
        let mut mock_publisher = MockConfigPublisher::new();

        let slave_account_id = "SLAVE_123";
        let master_account_id = "MASTER_001";
        let heartbeat = HeartbeatMessage {
            account_id: slave_account_id.to_string(),
            ea_type: "Slave".to_string(),
            is_trade_allowed: true,
            message_type: "Heartbeat".to_string(),
            timestamp: "2023-01-01T00:00:00Z".to_string(),
            version: "1.0.0".to_string(),
            platform: "MT5".to_string(),
            account_number: 123456,
            broker: "TestBroker".to_string(),
            account_name: "TestAccount".to_string(),
            server: "TestServer".to_string(),
            currency: "USD".to_string(),
            leverage: 100,
            balance: 10000.0,
            equity: 10000.0,
            open_positions: 0,
            symbol_prefix: None,
            symbol_suffix: None,
            symbol_map: None,
        };

        // First call for old_conn lookup returns None (new registration)
        mock_conn_manager
            .expect_get_slave()
            .with(eq(slave_account_id))
            .times(1)
            .return_const(None);

        // Heartbeat update
        mock_conn_manager
            .expect_update_heartbeat()
            .times(1)
            .return_const(true);

        // Slave settings exist for this slave
        let slave_settings = crate::domain::models::SlaveConfigWithMaster {
            master_account: master_account_id.to_string(),
            slave_account: slave_account_id.to_string(),
            slave_settings: crate::domain::models::SlaveSettings::default(),
            enabled_flag: true,
            status: 0, // DISABLED - will change to ENABLED/CONNECTED
            warning_codes: vec![],
        };
        mock_repo
            .expect_get_settings_for_slave()
            .with(eq(slave_account_id))
            .return_once(|_| Ok(vec![slave_settings]));

        // DB status update should be called
        mock_repo
            .expect_update_member_runtime_status()
            .times(1)
            .returning(|_, _, _| Ok(()));

        // EXPECT: Slave config should be published on state change
        mock_publisher
            .expect_send_slave_config()
            .times(1)
            .returning(|_| Ok(()));

        // Create a mock evaluator that returns changed status
        let mock_evaluator = Arc::new(MockStatusEvaluatorWithChange);

        let service = StatusService::new(
            Arc::new(mock_conn_manager),
            Arc::new(mock_repo),
            Arc::new(mock_publisher),
            Some(mock_evaluator),
            None,
            None,
        );

        service.handle_heartbeat(heartbeat).await;
    }

    /// Mock StatusEvaluator that simulates a status change
    struct MockStatusEvaluatorWithChange;
    #[async_trait]
    impl StatusEvaluator for MockStatusEvaluatorWithChange {
        async fn evaluate_member_runtime_status(
            &self,
            _target: SlaveRuntimeTarget<'_>,
        ) -> MemberStatusResult {
            // Return CONNECTED status (status changed from DISABLED)
            MemberStatusResult {
                status: crate::domain::models::STATUS_CONNECTED,
                allow_new_orders: true,
                warning_codes: vec![],
            }
        }

        async fn evaluate_member_runtime_status_with_snapshot(
            &self,
            _target: SlaveRuntimeTarget<'_>,
            _snapshot: crate::domain::services::status_calculator::ConnectionSnapshot,
        ) -> MemberStatusResult {
            // Return unknown/disabled for old state (to simulate change)
            MemberStatusResult::unknown()
        }

        async fn build_slave_bundle(
            &self,
            target: SlaveRuntimeTarget<'_>,
        ) -> crate::config_builder::SlaveConfigBundle {
            // Return a bundle with CONNECTED status
            crate::config_builder::SlaveConfigBundle {
                config: sankey_copier_zmq::SlaveConfigMessage {
                    account_id: target.slave_account.to_string(),
                    master_account: target.master_account.to_string(),
                    timestamp: 0,
                    trade_group_id: target.trade_group_id.to_string(),
                    status: crate::domain::models::STATUS_CONNECTED,
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
                status_result: MemberStatusResult {
                    status: crate::domain::models::STATUS_CONNECTED,
                    allow_new_orders: true,
                    warning_codes: vec![],
                },
            }
        }
    }
}
