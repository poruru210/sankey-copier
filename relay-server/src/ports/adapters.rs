// Adapter implementations for outbound ports
//
// This module implements the outbound port traits on concrete types,
// bridging the abstract service layer with the actual infrastructure.

use super::{
    ConfigPublisher, ConnectionManager, StatusEvaluator, TradeGroupRepository, UpdateBroadcaster,
};
use crate::adapters::inbound::http::SnapshotBroadcaster;
use crate::adapters::outbound::messaging::ZmqConfigPublisher;
use crate::adapters::outbound::persistence::Database;
use crate::config_builder::SlaveConfigBundle;
use crate::connection_manager::ConnectionManager as ConcreteConnectionManager;
use crate::domain::models::{
    EaConnection, HeartbeatMessage, SlaveConfigWithMaster, TradeGroup, TradeGroupMember,
    VLogsGlobalSettings,
};
use crate::domain::services::status_calculator::{
    ConnectionSnapshot, MemberStatusResult, SlaveRuntimeTarget,
};
use crate::runtime_status_updater::RuntimeStatusUpdater;
use async_trait::async_trait;
use sankey_copier_zmq::{MasterConfigMessage, SlaveConfigMessage};

// ============================================================================
// ConnectionManager Adapter
// ============================================================================

#[async_trait]
impl ConnectionManager for ConcreteConnectionManager {
    async fn get_master(&self, account_id: &str) -> Option<EaConnection> {
        ConcreteConnectionManager::get_master(self, account_id).await
    }

    async fn get_slave(&self, account_id: &str) -> Option<EaConnection> {
        ConcreteConnectionManager::get_slave(self, account_id).await
    }

    async fn update_heartbeat(&self, msg: HeartbeatMessage) -> bool {
        ConcreteConnectionManager::update_heartbeat(self, msg).await
    }
}

// ============================================================================
// VLogsConfigProvider Adapter
// ============================================================================

use crate::victoria_logs::VLogsController;

#[async_trait]
impl super::VLogsConfigProvider for VLogsController {
    fn get_config(&self) -> VLogsGlobalSettings {
        let config = self.config();
        VLogsGlobalSettings {
            enabled: self.is_enabled(),
            endpoint: config.endpoint(),
            batch_size: config.batch_size as i32,
            flush_interval_secs: config.flush_interval_secs as i32,
            log_level: "INFO".to_string(), // Default log level
        }
    }
}

// ============================================================================
// TradeGroupRepository Adapter
// ============================================================================

#[async_trait]
impl TradeGroupRepository for Database {
    async fn get_trade_group(&self, id: &str) -> anyhow::Result<Option<TradeGroup>> {
        Database::get_trade_group(self, id).await
    }

    async fn get_members(&self, master_id: &str) -> anyhow::Result<Vec<TradeGroupMember>> {
        Database::get_members(self, master_id).await
    }

    async fn get_settings_for_slave(
        &self,
        slave_id: &str,
    ) -> anyhow::Result<Vec<SlaveConfigWithMaster>> {
        Database::get_settings_for_slave(self, slave_id).await
    }

    async fn update_member_runtime_status(
        &self,
        master_id: &str,
        slave_id: &str,
        status: i32,
    ) -> anyhow::Result<()> {
        Database::update_member_runtime_status(self, master_id, slave_id, status).await
    }
}

// ============================================================================
// ConfigPublisher Adapter
// ============================================================================

#[async_trait]
impl ConfigPublisher for ZmqConfigPublisher {
    async fn send_master_config(&self, config: &MasterConfigMessage) -> anyhow::Result<()> {
        ZmqConfigPublisher::send(self, config).await
    }

    async fn send_slave_config(&self, config: &SlaveConfigMessage) -> anyhow::Result<()> {
        ZmqConfigPublisher::send(self, config).await
    }

    async fn broadcast_vlogs_config(&self, config: &VLogsGlobalSettings) -> anyhow::Result<()> {
        ZmqConfigPublisher::broadcast_vlogs_config(self, config).await
    }
}

// ============================================================================
// UpdateBroadcaster Adapter
// ============================================================================

#[async_trait]
impl UpdateBroadcaster for SnapshotBroadcaster {
    async fn broadcast_snapshot(&self) {
        SnapshotBroadcaster::broadcast_now(self).await
    }
}

// ============================================================================
// StatusEvaluator Adapter
// ============================================================================

/// Adapter that wraps RuntimeStatusUpdater to implement StatusEvaluator
pub struct RuntimeStatusEvaluatorAdapter {
    updater: RuntimeStatusUpdater,
}

impl RuntimeStatusEvaluatorAdapter {
    pub fn new(updater: RuntimeStatusUpdater) -> Self {
        Self { updater }
    }
}

#[async_trait]
impl StatusEvaluator for RuntimeStatusEvaluatorAdapter {
    async fn evaluate_member_runtime_status(
        &self,
        target: SlaveRuntimeTarget<'_>,
    ) -> MemberStatusResult {
        self.updater.evaluate_member_runtime_status(target).await
    }

    async fn evaluate_member_runtime_status_with_snapshot(
        &self,
        target: SlaveRuntimeTarget<'_>,
        snapshot: ConnectionSnapshot,
    ) -> MemberStatusResult {
        self.updater
            .evaluate_member_runtime_status_with_snapshot(target, snapshot)
            .await
    }

    async fn build_slave_bundle(&self, target: SlaveRuntimeTarget<'_>) -> SlaveConfigBundle {
        self.updater.build_slave_bundle(target).await
    }
}
