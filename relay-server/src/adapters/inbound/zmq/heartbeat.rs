//! Heartbeat message handler
//!
//! Handles heartbeat messages by delegating to StatusService.
//!
//! (Delegated to StatusService logic:
//!  - Master: DISABLED (web_ui OFF or !is_trade_allowed) or CONNECTED
//!  - Slave: DISABLED (web_ui OFF or !is_trade_allowed) or ENABLED (master not connected) or CONNECTED)

use super::MessageHandler;
use crate::domain::models::HeartbeatMessage;

impl MessageHandler {
    /// Handle heartbeat messages
    /// Delegates logic to StatusService
    pub(super) async fn handle_heartbeat(&self, msg: HeartbeatMessage) {
        // Delegate to StatusService
        self.status_service.handle_heartbeat(msg).await;
    }
}

#[cfg(test)]
mod tests {
    // use super::*;
    use crate::adapters::inbound::zmq::test_helpers::{build_heartbeat, create_test_context};
    use crate::domain::models::{SlaveSettings, STATUS_CONNECTED, STATUS_ENABLED};

    #[tokio::test]
    async fn test_handle_heartbeat_master_new_registration() {
        let ctx = create_test_context().await;
        let account_id = "MASTER_001";
        let msg = build_heartbeat(account_id, "Master", true);

        // Initial state: Master not in DB
        assert!(ctx
            .connection_manager
            .get_master(account_id)
            .await
            .is_none());

        // Process heartbeat
        ctx.handle_heartbeat(msg).await;

        // Verify registration
        let conn = ctx.connection_manager.get_master(account_id).await.unwrap();
        assert_eq!(conn.account_id, account_id);
        assert_eq!(conn.ea_type, crate::domain::models::EaType::Master);

        // Verify TradeGroup creation (auto-created on registration)
        assert!(ctx.db.get_trade_group(account_id).await.unwrap().is_some());

        ctx.cleanup().await;
    }

    #[tokio::test]
    async fn test_handle_heartbeat_slave_update() {
        let ctx = create_test_context().await;
        let account_id = "SLAVE_001";
        let msg = build_heartbeat(account_id, "Slave", true);

        // Initial state: Slave not in DB, create dummy entry
        let master_account = "MASTER_001";
        ctx.db.create_trade_group(master_account).await.unwrap();
        ctx.db
            .add_member(
                master_account,
                account_id,
                SlaveSettings::default(),
                STATUS_CONNECTED,
            )
            .await
            .unwrap();

        ctx.handle_heartbeat(msg).await;

        let conn = ctx.connection_manager.get_slave(account_id).await.unwrap();
        assert_eq!(conn.account_id, account_id);

        // Check runtime status update (Slave should be ONLINE)
        // Actually runtime status update happens inside handle_heartbeat via StatusService
        // We can verify DB state
        let member = ctx
            .db
            .get_member(master_account, account_id)
            .await
            .unwrap()
            .unwrap();
        // With auto-trading allowed and default settings, it should be ENABLED (status 2) or CONNECTED (status 2) = ENABLED
        assert_eq!(member.status, STATUS_ENABLED);

        ctx.cleanup().await;
    }
}
