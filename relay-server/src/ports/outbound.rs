use crate::models::{
    status_engine::{MemberStatusResult, SlaveRuntimeTarget},
    ConnectionStatus, EaConnection, HeartbeatMessage, SlaveSettings, TradeGroup,
    VLogsGlobalSettings,
};
use async_trait::async_trait;
use sankey_copier_zmq::{MasterConfigMessage, SlaveConfigMessage};

#[async_trait]
pub trait ConnectionManager: Send + Sync {
    async fn get_master(&self, account_id: &str) -> Option<EaConnection>;
    async fn get_slave(&self, account_id: &str) -> Option<EaConnection>;
    async fn update_heartbeat(&self, msg: HeartbeatMessage);
}

#[async_trait]
pub trait TradeGroupRepository: Send + Sync {
    async fn get_trade_group(&self, id: &str) -> anyhow::Result<Option<TradeGroup>>;
    async fn get_members(
        &self,
        master_id: &str,
    ) -> anyhow::Result<Vec<crate::models::TradeGroupMember>>;
    async fn get_settings_for_slave(&self, slave_id: &str) -> anyhow::Result<Vec<SlaveSettings>>;
    async fn update_member_runtime_status(
        &self,
        master_id: &str,
        slave_id: &str,
        status: ConnectionStatus,
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
    async fn evaluate_member_runtime_status(
        &self,
        target: SlaveRuntimeTarget<'_>,
    ) -> MemberStatusResult;
    // Add other methods if needed by StatusService
}
