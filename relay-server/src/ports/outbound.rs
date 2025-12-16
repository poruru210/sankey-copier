use crate::domain::models::{
    EaConnection, HeartbeatMessage, SlaveConfigWithMaster, TradeGroup, VLogsGlobalSettings,
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
    async fn get_masters_for_slave(&self, slave_account: &str) -> anyhow::Result<Vec<String>>;
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
    async fn broadcast_ea_disconnected(&self, account_id: &str);
    async fn broadcast_settings_updated(&self, json: &str);
}

/// Trait for handling EA disconnection events
/// This encapsulates the logic for notifying other EAs and updating status when an EA goes offline
#[async_trait]
pub trait DisconnectionService: Send + Sync {
    /// Handle Master EA going offline
    /// Notifies all connected Slaves and updates their runtime status
    async fn handle_master_offline(&self, master_account: &str);

    /// Handle Slave EA going offline
    /// Updates runtime status for all trade groups the Slave belongs to
    async fn handle_slave_offline(&self, slave_account: &str);
}
