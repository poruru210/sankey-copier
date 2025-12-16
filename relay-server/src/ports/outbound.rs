use crate::domain::models::{
    EaConnection, HeartbeatMessage, SlaveConfigWithMaster, TradeGroup, VLogsGlobalSettings,
};
use crate::domain::services::status_calculator::{
    ConnectionSnapshot, MemberStatusResult, SlaveRuntimeTarget,
};
use async_trait::async_trait;
use sankey_copier_zmq::{MasterConfigMessage, SlaveConfigMessage};

#[async_trait]
pub trait ConnectionManager: Send + Sync {
    async fn get_master(&self, account_id: &str) -> Option<EaConnection>;
    async fn get_slave(&self, account_id: &str) -> Option<EaConnection>;
    async fn update_heartbeat(&self, msg: HeartbeatMessage) -> bool;
}

#[async_trait]
pub trait VLogsConfigProvider: Send + Sync {
    fn get_config(&self) -> VLogsGlobalSettings;
}

#[async_trait]
pub trait TradeGroupRepository: Send + Sync {
    async fn get_trade_group(&self, id: &str) -> anyhow::Result<Option<TradeGroup>>;
    async fn create_trade_group(&self, id: &str) -> anyhow::Result<TradeGroup>;
    async fn get_members(
        &self,
        master_id: &str,
    ) -> anyhow::Result<Vec<crate::domain::models::TradeGroupMember>>;
    async fn get_settings_for_slave(
        &self,
        slave_id: &str,
    ) -> anyhow::Result<Vec<SlaveConfigWithMaster>>;
    async fn update_member_runtime_status(
        &self,
        master_id: &str,
        slave_id: &str,
        status: i32,
    ) -> anyhow::Result<()>;
}

#[async_trait]
pub trait ConfigPublisher: Send + Sync {
    async fn send_master_config(&self, config: &MasterConfigMessage) -> anyhow::Result<()>;
    async fn send_slave_config(&self, config: &SlaveConfigMessage) -> anyhow::Result<()>;
    async fn broadcast_vlogs_config(&self, config: &VLogsGlobalSettings) -> anyhow::Result<()>;
}

// Notification trait for broadcasting updates (WebSocket)
#[async_trait]
pub trait UpdateBroadcaster: Send + Sync {
    async fn broadcast_snapshot(&self);
}

#[async_trait]
pub trait StatusEvaluator: Send + Sync {
    /// Evaluate member runtime status based on current connection state
    async fn evaluate_member_runtime_status(
        &self,
        target: SlaveRuntimeTarget<'_>,
    ) -> MemberStatusResult;

    /// Evaluate member runtime status with an explicit snapshot (for "old" state detection)
    async fn evaluate_member_runtime_status_with_snapshot(
        &self,
        target: SlaveRuntimeTarget<'_>,
        snapshot: ConnectionSnapshot,
    ) -> MemberStatusResult;

    /// Build a complete SlaveConfigBundle for a specific Master-Slave connection
    async fn build_slave_bundle(
        &self,
        target: SlaveRuntimeTarget<'_>,
    ) -> crate::config_builder::SlaveConfigBundle;
}
