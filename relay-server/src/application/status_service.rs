use crate::application::runtime_status_updater::RuntimeStatusUpdater;
use crate::domain::models::HeartbeatMessage;
use crate::ports::outbound::{
    ConfigPublisher, ConnectionManager, TradeGroupRepository, UpdateBroadcaster,
    VLogsConfigProvider,
};
use std::sync::Arc;

#[allow(dead_code)] // Fields will be used in Phase 2 implementation
pub struct StatusService {
    connection_manager: Arc<dyn ConnectionManager>,
    repository: Arc<dyn TradeGroupRepository>,
    publisher: Arc<dyn ConfigPublisher>,
    runtime_status_updater: Arc<RuntimeStatusUpdater>,
    broadcaster: Option<Arc<dyn UpdateBroadcaster>>,
    vlogs_provider: Option<Arc<dyn VLogsConfigProvider>>,
}

impl StatusService {
    pub fn new(
        connection_manager: Arc<dyn ConnectionManager>,
        repository: Arc<dyn TradeGroupRepository>,
        publisher: Arc<dyn ConfigPublisher>,
        runtime_status_updater: Arc<RuntimeStatusUpdater>,
        broadcaster: Option<Arc<dyn UpdateBroadcaster>>,
        vlogs_provider: Option<Arc<dyn VLogsConfigProvider>>,
    ) -> Self {
        Self {
            connection_manager,
            repository,
            publisher,
            runtime_status_updater,
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
            Ok(None) => {
                tracing::info!(
                    "TradeGroup missing for Master {}, attempting to create...",
                    account_id
                );
                match self.repository.create_trade_group(account_id).await {
                    Ok(tg) => {
                        tracing::info!("Successfully created TradeGroup for Master {}", account_id);
                        tg
                    }
                    Err(e) => {
                        tracing::error!("Failed to create TradeGroup for {}: {}", account_id, e);
                        return;
                    }
                }
            }
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
        let mut master_changed = bundle.status_result.has_changed(&old_master_status);

        // Optimization: Suppress redundant update if transition is purely Registered -> Online
        // and is_trade_allowed is consistent (meaning Register already sent the correct config).
        if master_changed {
            if let Some(old) = &old_conn {
                if old.status == crate::domain::models::ConnectionStatus::Registered {
                    // If trade state is consistent, we assume Register handler sent the correct config.
                    // Note: old.is_trade_allowed comes from RegisterMessage (now accurate).
                    // msg.is_trade_allowed comes from Heartbeat.
                    if old.is_trade_allowed == msg.is_trade_allowed {
                        tracing::info!(
                            master = %account_id,
                            "Suppressing redundant config update (Registered -> Online with consistent trade state)"
                        );
                        master_changed = false;
                    }
                }
            }
        }

        if master_changed {
            tracing::info!(
                master = %account_id,
                old_status = old_master_status.status,
                new_status = bundle.status_result.status,
                old_warnings = ?old_master_status.warning_codes,
                new_warnings = ?bundle.status_result.warning_codes,
                "[StatusService] Master status CHANGE detected"
            );
        } else {
            // Optional: Trace that no change was detected if we suspect it's NOT changing but sending anyway?
            // But existing code sends ONLY if master_changed.
            // So if it sends, master_changed IS true.
            tracing::debug!(
                master = %account_id,
                "[StatusService] Master status stable (No Change)"
            );
        }

        tracing::debug!(
            master = %account_id,
            changed = master_changed,
            old_status = old_master_status.status,
            new_status = bundle.status_result.status,
            "[StatusService] Master status change detection result"
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
        let updater = &self.runtime_status_updater;

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
            let slave_bundle = updater.build_slave_bundle(target).await;

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

        let updater = &self.runtime_status_updater;

        for settings in settings_list {
            // Build new bundle
            let slave_bundle = updater
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
                updater
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
    use async_trait::async_trait;
    use chrono::Utc;
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
            async fn create_trade_group(&self, id: &str) -> anyhow::Result<TradeGroup>;
            async fn get_members(&self, master_id: &str) -> anyhow::Result<Vec<TradeGroupMember>>;
            async fn get_settings_for_slave(&self, slave_id: &str) -> anyhow::Result<Vec<crate::domain::models::SlaveConfigWithMaster>>;
            async fn update_member_runtime_status(&self, master_id: &str, slave_id: &str, status: i32) -> anyhow::Result<()>;
            async fn get_masters_for_slave(&self, slave_account: &str) -> anyhow::Result<Vec<String>>;
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
            async fn broadcast_ea_disconnected(&self, account_id: &str);
            async fn broadcast_settings_updated(&self, json: &str);
        }
    }

    mock! {
        pub VLogsConfigProvider {}
        impl VLogsConfigProvider for VLogsConfigProvider {
            fn get_config(&self) -> crate::domain::models::VLogsGlobalSettings;
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
            Arc::new(
                crate::application::runtime_status_updater::RuntimeStatusUpdater::with_metrics(
                    Arc::new(MockTradeGroupRepository::new()),
                    Arc::new(MockConnectionManager::new()),
                    Arc::new(
                        crate::application::runtime_status_updater::RuntimeStatusMetrics::default(),
                    ),
                ),
            ),
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

        // RuntimeStatusUpdater calls get_members
        mock_repo.expect_get_members().returning(|_| Ok(vec![]));

        let service = StatusService::new(
            Arc::new(mock_conn_manager),
            Arc::new(mock_repo),
            Arc::new(mock_publisher),
            Arc::new(
                crate::application::runtime_status_updater::RuntimeStatusUpdater::with_metrics(
                    Arc::new(MockTradeGroupRepository::new()),
                    Arc::new(MockConnectionManager::new()),
                    Arc::new(
                        crate::application::runtime_status_updater::RuntimeStatusMetrics::default(),
                    ),
                ),
            ),
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
        // Note: RuntimeStatusUpdater might also call get_master, so we need sequences or flexible times.
        // But RuntimeStatusUpdater is called AFTER the registration logic in handle_heartbeat.
        // handle_heartbeat flow:
        // 1. connection_manager.get_master (old_conn) -> None
        // 2. connection_manager.update_heartbeat -> true
        // 3. connection_manager.get_master (new_conn) -> Some(...)
        // 4. trade_group_repository.get_trade_group -> None
        // 5. trade_group_repository.create_trade_group
        // 6. ConfigPublisher.send_master_config
        // 7. ConfigPublisher.publish_trade_group_updates
        // 8. RuntimeStatusUpdater.evaluate_master...
        //    -> get_trade_group
        //    -> get_master

        // Sequence for get_master
        let mut seq = mockall::Sequence::new();

        mock_conn_manager
            .expect_get_master()
            .with(eq(account_id))
            .times(1)
            .in_sequence(&mut seq)
            .return_const(None); // 1. old_conn

        // Heartbeat update
        mock_conn_manager
            .expect_update_heartbeat()
            .times(1)
            .return_const(true);

        // Subsequent calls to get_master (verification + runtime updater) return the new connection
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
            .times(1..) // 3. new_conn check + 8. RuntimeStatusUpdater
            .in_sequence(&mut seq)
            .return_const(Some(new_conn));

        // TradeGroup missing initially
        let trade_group = TradeGroup::new(account_id.to_string());

        // Sequence for get_trade_group
        // Note: Shared mock with RuntimeStatusUpdater, use returning for flexibility
        let mut call_count = 0;
        let tg_clone = trade_group.clone();
        mock_repo
            .expect_get_trade_group()
            .with(eq(account_id))
            .returning(move |_| {
                call_count += 1;
                if call_count == 1 {
                    Ok(None) // First call: not found
                } else {
                    Ok(Some(tg_clone.clone())) // Subsequent calls: found
                }
            });

        // EXPECT: create_trade_group to be called
        let tg_for_create = trade_group.clone();
        mock_repo
            .expect_create_trade_group()
            .with(eq(account_id))
            .times(1)
            .return_once(move |_| Ok(tg_for_create));

        // RuntimeStatusUpdater calls get_members
        mock_repo.expect_get_members().returning(|_| Ok(vec![]));

        // EXPECT: Config should be published for new registration
        mock_publisher
            .expect_send_master_config()
            .times(1)
            .returning(|_| Ok(()));

        // Create Arcs
        let conn_manager = Arc::new(mock_conn_manager);
        let repo = Arc::new(mock_repo);
        let publisher = Arc::new(mock_publisher);

        let service = StatusService::new(
            conn_manager.clone(),
            repo.clone(),
            publisher,
            Arc::new(
                crate::application::runtime_status_updater::RuntimeStatusUpdater::with_metrics(
                    repo,
                    conn_manager,
                    Arc::new(
                        crate::application::runtime_status_updater::RuntimeStatusMetrics::default(),
                    ),
                ),
            ),
            None,
            None,
        );

        service.handle_heartbeat(heartbeat).await;
    }

    /// TDD Test: Slave heartbeat with status change sends config
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

        // 1. Setup StatusService dependencies call expectations
        // Note: RuntimeStatusUpdater will ALSO use these mocks.

        // --- ConnectionManager Expectations ---

        // get_slave called only once by StatusService (to check old connections)
        // AND once by RuntimeStatusUpdater (to get snapshot)
        // We can just return None for the first call (new heartbeat processing basically)
        // But wait, handle_slave_heartbeat -> get_settings -> build_slave_bundle -> connection_manager.get_slave
        // AND handle_slave_heartbeat -> get_slave (old conn)

        // Let's set up the Connections to result in ENABLED status.
        let slave_conn = EaConnection {
            account_id: slave_account_id.to_string(),
            ea_type: crate::domain::models::EaType::Slave,
            status: ConnectionStatus::Online,
            is_trade_allowed: true,
            ..Default::default()
        };
        let master_conn = EaConnection {
            account_id: master_account_id.to_string(),
            ea_type: crate::domain::models::EaType::Master,
            status: ConnectionStatus::Online,
            is_trade_allowed: true,
            ..Default::default()
        };

        // Connection Manager will be called multiple times.
        // 1. update_heartbeat (StatusService)
        mock_conn_manager
            .expect_update_heartbeat()
            .times(1)
            .return_const(true);

        // 2. get_slave (StatusService: old_conn lookup - returns None as if first time or lost track)
        // Actually if we want to simulate change from DISABLED(0) to ENABLED(2),
        // we can have old_conn return None (implied Disconnected/Unknown) OR return a Connected conn but previous calc was Disabled.
        // Let's assume old_conn is None so old_status is Unknown.
        // But wait, the test says "status changed from disabled".
        // The DB settings say "status: 0".
        // If Runtime calc returns 2, then 0 != 2 -> Change Detected.
        // This is sufficient.

        // However, RuntimeStatusUpdater needs to fetch connections.
        // StatusService.handle_slave_heartbeat calls:
        //   repo.get_settings_for_slave
        //   runtime_status_updater.build_slave_bundle
        //     -> runtime_status_updater.evaluate_master_runtime_status -> repo.get_trade_group
        //     -> runtime_status_updater.evaluate_master_runtime_status -> conn.get_master
        //     -> runtime_status_updater.slave_connection_snapshot -> conn.get_slave
        //     -> conn.get_master (for equity)

        // So we need to provide these returns.
        mock_conn_manager
            .expect_get_slave()
            .with(eq(slave_account_id))
            .returning(move |_| Some(slave_conn.clone()));

        mock_conn_manager
            .expect_get_master()
            .with(eq(master_account_id))
            .returning(move |_| Some(master_conn.clone()));

        // --- Repository Expectations ---

        // 1. get_settings_for_slave (StatusService)
        let slave_settings = crate::domain::models::SlaveConfigWithMaster {
            master_account: master_account_id.to_string(),
            slave_account: slave_account_id.to_string(),
            slave_settings: crate::domain::models::SlaveSettings::default(),
            enabled_flag: true,
            status: 0, // DISABLED in DB - this is key for triggering "Change Detected"
            warning_codes: vec![],
        };
        mock_repo
            .expect_get_settings_for_slave()
            .with(eq(slave_account_id))
            .return_once(move |_| Ok(vec![slave_settings]));

        // 2. get_trade_group (RuntimeStatusUpdater -> evaluate_master_runtime_status)
        let mut trade_group = TradeGroup::new(master_account_id.to_string());
        trade_group.master_settings.enabled = true; // Master Enabled
        mock_repo
            .expect_get_trade_group()
            .with(eq(master_account_id))
            .returning(move |_| Ok(Some(trade_group.clone())));

        // 3. update_member_runtime_status (StatusService - persisting change)
        mock_repo
            .expect_update_member_runtime_status()
            .times(1)
            .returning(|_, _, _| Ok(()));

        // --- Publisher Expectations ---

        // EXPECT: Slave config should be published on state change
        // Because 0 (DB) != 2 (Calculated: Master Online + Slave Online + Flags Enabled)
        mock_publisher
            .expect_send_slave_config()
            .times(1)
            .returning(|_| Ok(()));

        // Setup Services with shared Arcs
        let conn_arc: Arc<dyn ConnectionManager> = Arc::new(mock_conn_manager);
        let repo_arc: Arc<dyn TradeGroupRepository> = Arc::new(mock_repo);
        let pub_arc: Arc<dyn ConfigPublisher> = Arc::new(mock_publisher);
        let metrics =
            Arc::new(crate::application::runtime_status_updater::RuntimeStatusMetrics::default());

        // Create real RuntimeStatusUpdater with Mocks
        let runtime_updater = Arc::new(
            crate::application::runtime_status_updater::RuntimeStatusUpdater::with_metrics(
                repo_arc.clone(),
                conn_arc.clone(),
                metrics,
            ),
        );

        let service = StatusService::new(conn_arc, repo_arc, pub_arc, runtime_updater, None, None);

        service.handle_heartbeat(heartbeat).await;
    }
    #[tokio::test]
    async fn test_handle_heartbeat_master_regression_status_change() {
        // Scenario: Master is ENABLED (Online).
        // Event: Heartbeat arrives with `is_trade_allowed=false` (Disabled).
        // Expectation: Config MUST be sent (Status Change: Connected -> Disabled).

        let mut mock_conn_manager = MockConnectionManager::new();
        let mut mock_repo = MockTradeGroupRepository::new();
        let mut mock_publisher = MockConfigPublisher::new();

        let account_id = "MASTER_REG_1";
        // 1. Initial State: Online & Trade Allowed
        let conn_initial = EaConnection {
            account_id: account_id.to_string(),
            ea_type: crate::domain::models::EaType::Master,
            status: ConnectionStatus::Online,
            is_trade_allowed: true, // Initially Allowed
            last_heartbeat: Utc::now(),
            ..Default::default()
        };

        // 2. Incoming Heartbeat: Trade DISABLED
        let heartbeat = HeartbeatMessage {
            account_id: account_id.to_string(),
            message_type: "Heartbeat".to_string(),
            is_trade_allowed: false, // Changed to False
            ea_type: "Master".to_string(),
            platform: "MT5".to_string(),
            balance: 10000.0,
            equity: 10000.0,
            open_positions: 0,
            timestamp: Utc::now().to_rfc3339(),
            version: "1.0.0".to_string(),
            account_number: 123,
            broker: "B".to_string(),
            account_name: "N".to_string(),
            server: "S".to_string(),
            currency: "USD".to_string(),
            leverage: 100,
            symbol_prefix: None,
            symbol_suffix: None,
            symbol_map: None,
        };

        let mut seq = mockall::Sequence::new();

        // Step 1: get_master returns INITIAL state (Used for old_status calculation)
        mock_conn_manager
            .expect_get_master()
            .with(eq(account_id))
            .times(1)
            .in_sequence(&mut seq)
            .return_const(Some(conn_initial.clone()));

        // Step 2: update_heartbeat returns false (Existing connection updated)
        mock_conn_manager
            .expect_update_heartbeat()
            .times(1)
            .in_sequence(&mut seq)
            .return_const(false);

        // Step 3: get_master returns NEW state (Used for new_status calculation)
        let mut conn_updated = conn_initial.clone();
        conn_updated.is_trade_allowed = false; // DB/Mem updated
        mock_conn_manager
            .expect_get_master()
            .with(eq(account_id))
            .times(1) // Called once by handle_master_heartbeat
            .in_sequence(&mut seq)
            .return_const(Some(conn_updated));

        // Step 4: TradeGroup logic (Enabled=true, so is_trade_allowed is the decider)
        let mut tg = TradeGroup::new(account_id.to_string());
        tg.master_settings.enabled = true;
        mock_repo
            .expect_get_trade_group()
            .with(eq(account_id))
            .return_once(move |_| Ok(Some(tg)));

        // Step 5: Publisher MUST be called
        mock_publisher
            .expect_send_master_config()
            .times(1)
            .returning(|_| Ok(()));

        // Ignore others
        mock_publisher
            .expect_broadcast_vlogs_config()
            .returning(|_| Ok(()));
        mock_repo.expect_get_members().returning(|_| Ok(vec![]));

        let service = StatusService::new(
            Arc::new(mock_conn_manager),
            Arc::new(mock_repo),
            Arc::new(mock_publisher),
            Arc::new(
                crate::application::runtime_status_updater::RuntimeStatusUpdater::with_metrics(
                    Arc::new(MockTradeGroupRepository::new()),
                    Arc::new(MockConnectionManager::new()),
                    Arc::new(
                        crate::application::runtime_status_updater::RuntimeStatusMetrics::default(),
                    ),
                ),
            ),
            None,
            None,
        );

        service.handle_heartbeat(heartbeat).await;
    }

    #[tokio::test]
    async fn test_handle_heartbeat_master_regression_offline_online() {
        // Scenario: Master is OFFLINE.
        // Event: Heartbeat arrives (Online).
        // Expectation: Config MUST be sent (Status Change: Disabled/Offline -> Connected).

        let mut mock_conn_manager = MockConnectionManager::new();
        let mut mock_repo = MockTradeGroupRepository::new();
        let mut mock_publisher = MockConfigPublisher::new();

        let account_id = "MASTER_REG_2";
        // 1. Initial State: Offline
        let conn_initial = EaConnection {
            account_id: account_id.to_string(),
            ea_type: crate::domain::models::EaType::Master,
            status: ConnectionStatus::Offline, // Offline
            is_trade_allowed: true,
            last_heartbeat: Utc::now() - chrono::Duration::seconds(60),
            ..Default::default()
        };

        let heartbeat = HeartbeatMessage {
            account_id: account_id.to_string(),
            message_type: "Heartbeat".to_string(),
            is_trade_allowed: true,
            ea_type: "Master".to_string(),
            platform: "MT5".to_string(),
            balance: 10000.0,
            equity: 10000.0,
            open_positions: 0,
            timestamp: Utc::now().to_rfc3339(),
            version: "1.0.0".to_string(),
            account_number: 123,
            broker: "B".to_string(),
            account_name: "N".to_string(),
            server: "S".to_string(),
            currency: "USD".to_string(),
            leverage: 100,
            symbol_prefix: None,
            symbol_suffix: None,
            symbol_map: None,
        };

        let mut seq = mockall::Sequence::new();

        mock_conn_manager
            .expect_get_master()
            .with(eq(account_id))
            .times(1)
            .in_sequence(&mut seq)
            .return_const(Some(conn_initial));
        mock_conn_manager
            .expect_update_heartbeat()
            .times(1)
            .in_sequence(&mut seq)
            .return_const(false); // Valid update

        let conn_updated = EaConnection {
            account_id: account_id.to_string(),
            ea_type: crate::domain::models::EaType::Master,
            status: ConnectionStatus::Online, // Now Online
            is_trade_allowed: true,
            ..Default::default()
        };
        mock_conn_manager
            .expect_get_master()
            .with(eq(account_id))
            .times(1)
            .in_sequence(&mut seq)
            .return_const(Some(conn_updated));

        let mut tg = TradeGroup::new(account_id.to_string());
        tg.master_settings.enabled = true;
        mock_repo
            .expect_get_trade_group()
            .with(eq(account_id))
            .return_once(move |_| Ok(Some(tg)));

        // Publisher MUST be called
        mock_publisher
            .expect_send_master_config()
            .times(1)
            .returning(|_| Ok(()));

        mock_publisher
            .expect_broadcast_vlogs_config()
            .returning(|_| Ok(()));
        mock_repo.expect_get_members().returning(|_| Ok(vec![]));

        let service = StatusService::new(
            Arc::new(mock_conn_manager),
            Arc::new(mock_repo),
            Arc::new(mock_publisher),
            Arc::new(
                crate::application::runtime_status_updater::RuntimeStatusUpdater::with_metrics(
                    Arc::new(MockTradeGroupRepository::new()),
                    Arc::new(MockConnectionManager::new()),
                    Arc::new(
                        crate::application::runtime_status_updater::RuntimeStatusMetrics::default(),
                    ),
                ),
            ),
            None,
            None,
        );

        service.handle_heartbeat(heartbeat).await;
    }

    #[tokio::test]
    async fn test_handle_heartbeat_master_idempotency() {
        // Scenario: Master is ONLINE. Identical Heartbeat arrives.
        // Expectation: Config should NOT be sent (Redundancy check).

        // !!! IMPORTANT !!!
        // Before Fix: this test should FAIL (expect 0 calls, gets 1).
        // After Fix: this test should PASS.

        let mut mock_conn_manager = MockConnectionManager::new();
        let mut mock_repo = MockTradeGroupRepository::new();
        let mut mock_publisher = MockConfigPublisher::new();

        let account_id = "MASTER_IDEM";
        // 1. Initial State: Online & Stable
        let conn_initial = EaConnection {
            account_id: account_id.to_string(),
            ea_type: crate::domain::models::EaType::Master,
            status: ConnectionStatus::Online,
            is_trade_allowed: true,
            last_heartbeat: Utc::now(), // Recent
            ..Default::default()
        };

        // 2. Incoming Heartbeat: Identical Status, Different Timestamp
        let heartbeat = HeartbeatMessage {
            account_id: account_id.to_string(),
            message_type: "Heartbeat".to_string(),
            is_trade_allowed: true, // Same
            ea_type: "Master".to_string(),
            platform: "MT5".to_string(),
            balance: 10000.0,
            equity: 10000.0,
            open_positions: 0,
            timestamp: (Utc::now() + chrono::Duration::seconds(1)).to_rfc3339(), // 1s later
            version: "1.0.0".to_string(),
            account_number: 123,
            broker: "B".to_string(),
            account_name: "N".to_string(),
            server: "S".to_string(),
            currency: "USD".to_string(),
            leverage: 100,
            symbol_prefix: None,
            symbol_suffix: None,
            symbol_map: None,
        };

        let mut seq = mockall::Sequence::new();

        // Step 1: get_master for OLD state
        mock_conn_manager
            .expect_get_master()
            .with(eq(account_id))
            .times(1)
            .in_sequence(&mut seq)
            .return_const(Some(conn_initial.clone()));

        // Step 2: update_heartbeat (Existing)
        mock_conn_manager
            .expect_update_heartbeat()
            .times(1)
            .in_sequence(&mut seq)
            .return_const(false);

        // Step 3: get_master for NEW state (Identical)
        mock_conn_manager
            .expect_get_master()
            .with(eq(account_id))
            .times(1)
            .in_sequence(&mut seq)
            .return_const(Some(conn_initial)); // Same state

        let mut tg = TradeGroup::new(account_id.to_string());
        tg.master_settings.enabled = true;
        mock_repo
            .expect_get_trade_group()
            .with(eq(account_id))
            .return_once(move |_| Ok(Some(tg)));

        // Step 4: Publisher
        // EXPECTATION: 0 calls (Silence).
        // CURRENT BUG: This likely triggers 1 call.
        mock_publisher
            .expect_send_master_config()
            .times(0) // MUST BE ZERO
            .returning(|_| Ok(()));

        mock_publisher
            .expect_broadcast_vlogs_config()
            .returning(|_| Ok(()));
        mock_repo.expect_get_members().returning(|_| Ok(vec![]));

        let service = StatusService::new(
            Arc::new(mock_conn_manager),
            Arc::new(mock_repo),
            Arc::new(mock_publisher),
            Arc::new(
                crate::application::runtime_status_updater::RuntimeStatusUpdater::with_metrics(
                    Arc::new(MockTradeGroupRepository::new()),
                    Arc::new(MockConnectionManager::new()),
                    Arc::new(
                        crate::application::runtime_status_updater::RuntimeStatusMetrics::default(),
                    ),
                ),
            ),
            None,
            None,
        );

        service.handle_heartbeat(heartbeat).await;
    }

    #[tokio::test]
    async fn test_handle_heartbeat_master_integration_sequence() {
        // Scenario: Integration test using REAL ConnectionManager (HashMap based).
        // 1. Register (Status -> Registered)
        // 2. Heartbeat 1 (Status -> Online). Expect Config Send (Change Detected).
        // 3. Heartbeat 2 (Status -> Online). Expect NO Config Send (No Change).

        // Use REAL ConnectionManager
        let real_conn_manager = Arc::new(
            crate::adapters::infrastructure::connection_manager::ConnectionManager::new(30),
        );

        // Mocks for others
        let mut mock_repo = MockTradeGroupRepository::new();
        let mut mock_publisher = MockConfigPublisher::new();

        let account_id = "MASTER_INTEGRATION";

        // Setup TradeGroup (Always present and enabled)
        let mut tg = TradeGroup::new(account_id.to_string());
        tg.master_settings.enabled = true;

        // Allow repository to be called multiple times
        // NOTE: We need to use `returning` with a closure that returns a NEW clone each time
        mock_repo
            .expect_get_trade_group()
            .with(eq(account_id))
            .returning(move |_| {
                let mut tg = TradeGroup::new("MASTER_INTEGRATION".to_string());
                tg.master_settings.enabled = true;
                Ok(Some(tg))
            });

        mock_repo.expect_get_members().returning(|_| Ok(vec![]));
        mock_repo
            .expect_create_trade_group()
            .returning(|id| Ok(TradeGroup::new(id.to_string())));

        // ConfigPublisher Expectation:
        // Should be called EXACTLY ONCE (for the first Heartbeat that transitions Registered->Online).
        // The second Heartbeat should NOT trigger it.
        mock_publisher
            .expect_send_master_config()
            .times(1)
            .returning(|_| Ok(()));

        mock_publisher
            .expect_broadcast_vlogs_config()
            .returning(|_| Ok(()));

        let service = StatusService::new(
            real_conn_manager.clone(), // Pass the real implementation
            Arc::new(mock_repo),
            Arc::new(mock_publisher),
            Arc::new(
                crate::application::runtime_status_updater::RuntimeStatusUpdater::with_metrics(
                    Arc::new(MockTradeGroupRepository::new()),
                    real_conn_manager.clone(), // Pass real CM to updater too
                    Arc::new(
                        crate::application::runtime_status_updater::RuntimeStatusMetrics::default(),
                    ),
                ),
            ),
            None,
            None,
        );

        // 1. Send Register
        let register_msg = crate::domain::models::RegisterMessage {
            message_type: "Register".to_string(),
            account_id: account_id.to_string(),
            ea_type: "Master".to_string(),
            platform: "MT5".to_string(),
            account_number: 123,
            broker: "B".to_string(),
            account_name: "N".to_string(),
            server: "S".to_string(),
            currency: "USD".to_string(),
            leverage: 100,
            timestamp: Utc::now().to_rfc3339(),
            symbol_context: None,
            is_trade_allowed: false,
        };
        real_conn_manager.register_ea(&register_msg).await;

        // Verify Initial State
        let conn = real_conn_manager.get_master(account_id).await.unwrap();
        assert_eq!(conn.status, ConnectionStatus::Registered);
        assert_eq!(conn.is_trade_allowed, false);

        // 2. Send Heartbeat 1 (Transition to Online, Trade Allowed)
        let hb1 = HeartbeatMessage {
            account_id: account_id.to_string(),
            message_type: "Heartbeat".to_string(),
            is_trade_allowed: true, // Allow
            ea_type: "Master".to_string(),
            platform: "MT5".to_string(),
            balance: 10000.0,
            equity: 10000.0,
            open_positions: 0,
            timestamp: Utc::now().to_rfc3339(),
            version: "1.0.0".to_string(),
            account_number: 123,
            broker: "B".to_string(),
            account_name: "N".to_string(),
            server: "S".to_string(),
            currency: "USD".to_string(),
            leverage: 100,
            symbol_prefix: None,
            symbol_suffix: None,
            symbol_map: None,
        };

        service.handle_heartbeat(hb1).await;
        // Expectation: send_master_config called (times=1)

        // 3. Send Heartbeat 2 (Identical)
        let hb2 = HeartbeatMessage {
            account_id: account_id.to_string(),
            message_type: "Heartbeat".to_string(),
            is_trade_allowed: true, // Still Allowed
            ea_type: "Master".to_string(),
            platform: "MT5".to_string(),
            balance: 10000.0,
            equity: 10000.0,
            open_positions: 0,
            timestamp: (Utc::now() + chrono::Duration::seconds(1)).to_rfc3339(), // 1s later
            version: "1.0.0".to_string(),
            account_number: 123,
            broker: "B".to_string(),
            account_name: "N".to_string(),
            server: "S".to_string(),
            currency: "USD".to_string(),
            leverage: 100,
            symbol_prefix: None,
            symbol_suffix: None,
            symbol_map: None,
        };

        service.handle_heartbeat(hb2).await;
        // Expectation: send_master_config NOT called (Total times still 1)
    }

    #[tokio::test]
    async fn test_heartbeat_after_accurate_register_no_duplicate() {
        // Scenario:
        // 1. Register WITH is_trade_allowed=true (New Feature)
        // 2. Heartbeat (is_trade_allowed=true)
        // Expectation: Config sent ZERO times by Heartbeat (Register already sent it)

        // Use REAL ConnectionManager
        let real_conn_manager = Arc::new(
            crate::adapters::infrastructure::connection_manager::ConnectionManager::new(30),
        );
        let mut mock_repo = MockTradeGroupRepository::new();
        let mut mock_publisher = MockConfigPublisher::new();
        let account_id = "ACCURATE_REGISTER";

        // Setup TradeGroup
        let mut tg = TradeGroup::new(account_id.to_string());
        tg.master_settings.enabled = true;

        mock_repo
            .expect_get_trade_group()
            .with(eq(account_id))
            .returning(move |_| {
                let mut tg = TradeGroup::new("ACCURATE_REGISTER".to_string());
                tg.master_settings.enabled = true;
                Ok(Some(tg))
            });
        mock_repo.expect_get_members().returning(|_| Ok(vec![]));
        mock_repo
            .expect_create_trade_group()
            .returning(|id| Ok(TradeGroup::new(id.to_string())));

        // IMPORTANT: We expect send_master_config to NOT be called
        mock_publisher
            .expect_send_master_config()
            .times(0)
            .returning(|_| Ok(()));

        // Broadcast VLogs config is expected on Register (via MessageHandler), but StatusService doesn't handle Register.
        // StatusService DOES handle Heartbeat.
        // Heartbeat logic calls update_related_slaves -> which calls repo.

        let mock_repo = Arc::new(mock_repo);
        let mock_publisher = Arc::new(mock_publisher);
        let real_conn_arc =
            real_conn_manager.clone() as Arc<dyn crate::ports::outbound::ConnectionManager>;

        let service = StatusService::new(
            real_conn_arc.clone(),
            mock_repo.clone(),
            mock_publisher,
            Arc::new(RuntimeStatusUpdater::with_metrics(
                mock_repo.clone(),
                real_conn_arc,
                Arc::new(
                    crate::application::runtime_status_updater::RuntimeStatusMetrics::default(),
                ),
            )),
            None,
            None,
        );

        // 1. Send Register (is_trade_allowed = true) directly to ConnectionManager
        // (Simulating MessageHandler::handle_register behavior)
        let register_msg = crate::domain::models::RegisterMessage {
            message_type: "Register".to_string(),
            account_id: account_id.to_string(),
            ea_type: "Master".to_string(),
            platform: "MT5".to_string(),
            account_number: 12345,
            broker: "Test Broker".to_string(),
            account_name: "Test Account".to_string(),
            server: "Test-Server".to_string(),
            currency: "USD".to_string(),
            leverage: 100,
            timestamp: chrono::Utc::now().to_rfc3339(),
            symbol_context: None,
            is_trade_allowed: true, // Accurate from start!
        };
        real_conn_manager.register_ea(&register_msg).await;

        // Verify Status is Registered but is_trade_allowed is TRUE
        let conn = real_conn_manager.get_master(account_id).await.unwrap();
        assert_eq!(conn.status, ConnectionStatus::Registered);
        assert_eq!(conn.is_trade_allowed, true);

        // 2. Send Heartbeat (Identical)
        let hb = HeartbeatMessage {
            account_id: account_id.to_string(),
            message_type: "Heartbeat".to_string(),
            is_trade_allowed: true, // Same as Register
            ea_type: "Master".to_string(),
            platform: "MT5".to_string(),
            balance: 10000.0,
            equity: 10000.0,
            open_positions: 0,
            timestamp: chrono::Utc::now().to_rfc3339(),
            version: "1.0.0".to_string(),
            account_number: 12345,
            broker: "Test Broker".to_string(),
            account_name: "Test Account".to_string(),
            server: "Test-Server".to_string(),
            currency: "USD".to_string(),
            leverage: 100,
            symbol_prefix: None,
            symbol_suffix: None,
            symbol_map: None,
        };

        service.handle_heartbeat(hb).await;
        // If successful (no panic), then mock expectation (times=0) passed partially (mockall validates on drop usually)
    }
}
