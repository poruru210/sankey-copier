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
        _old_conn: Option<crate::models::EaConnection>,
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

        // Get Master Connection
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

        let _bundle = crate::config_builder::ConfigBuilder::build_master_config(context);

        // TODO: Implement change detection and publishing logic similar to heartbeat.rs
        // For TDD step 1, we just minimally need to pass the basic flow test which expects update_heartbeat (already called above).
        // Since the test doesn't check for publish calls yet, this is sufficient to turn Green for the current test.
    }

    async fn handle_slave_heartbeat(
        &self,
        _msg: HeartbeatMessage,
        _old_conn: Option<crate::models::EaConnection>,
    ) {
        // TODO: Implement slave heartbeat logic
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
            async fn get_settings_for_slave(&self, slave_id: &str) -> anyhow::Result<Vec<SlaveSettings>>;
            async fn update_member_runtime_status(&self, master_id: &str, slave_id: &str, status: ConnectionStatus) -> anyhow::Result<()>;
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
}
