//! Unregister message handler
//!
//! Handles EA unregistration messages, updating connection status and notifying clients.
//! When a Master EA disconnects, notifies all Slaves so they can update their status.
//! When a Slave EA disconnects, updates runtime status and notifies WebSocket clients.

use super::MessageHandler;
use crate::domain::models::{EaType, UnregisterMessage};

impl MessageHandler {
    /// Handle EA unregistration
    /// When a Master disconnects, notify all Slaves to update their status from CONNECTED to ENABLED
    /// When a Slave disconnects, update runtime status and notify WebSocket clients
    pub(super) async fn handle_unregister(&self, msg: UnregisterMessage) {
        let account_id = &msg.account_id;

        // Get EA type before unregistering
        let ea_type = self
            .connection_manager
            .get_ea(account_id)
            .await
            .map(|conn| conn.ea_type);

        // Unregister the EA (if ea_type was found)
        if let Some(et) = ea_type {
            self.connection_manager.unregister_ea(account_id, et).await;
        }

        // Notify WebSocket clients
        // DisconnectionService handles specific logic, but we broadcast the generic event here
        // Or strictly rely on DisconnectionService?
        // Existing logic sent "ea_disconnected" here.
        // If DisconnectionService handles "ea_disconnected" broadcast in handle_master_offline / handle_slave_offline,
        // we might double broadcast if we do it here too.
        // BUT DisconnectionService uses updateBroadcaster which has broadcast_ea_disconnected method.
        // Let's use DisconnectionService for everything if possible.
        // However, DisconnectionService traits methods are handle_master_offline and handle_slave_offline.
        // They are specific.
        // If we want a generic "ea_disconnected" regardless of type success, we can keep it or use the broadcaster directly.
        // Let's keep the generic broadcast here using the broadcaster injected into MessageHandler?
        // MessageHandler has broadcast_tx.

        let _ = self
            .broadcast_tx
            .send(format!("ea_disconnected:{}", account_id));

        match ea_type {
            Some(EaType::Master) => {
                // Master disconnected - notify all Slaves
                tracing::info!("Master {} disconnected, notifying Slaves", account_id);
                self.disconnection_service
                    .handle_master_offline(account_id)
                    .await;
            }
            Some(EaType::Slave) => {
                // Slave disconnected - update runtime status and notify WebSocket
                tracing::info!("Slave {} disconnected, updating runtime status", account_id);
                self.disconnection_service
                    .handle_slave_offline(account_id)
                    .await;
            }
            None => {
                tracing::debug!(
                    "Unknown EA {} disconnected (not found in connection manager)",
                    account_id
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::inbound::zmq::test_helpers::{build_heartbeat, create_test_context};
    use crate::domain::models::{STATUS_CONNECTED, STATUS_ENABLED};

    #[tokio::test]
    async fn test_handle_unregister() {
        let ctx = create_test_context().await;
        let account_id = "TEST_001".to_string();

        // First auto-register via heartbeat
        let hb_msg = crate::domain::models::HeartbeatMessage {
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
        ctx.handle_heartbeat(hb_msg).await;

        // Then unregister
        ctx.handle_unregister(UnregisterMessage {
            message_type: "Unregister".to_string(),
            account_id: account_id.clone(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            ea_type: Some("Master".to_string()),
        })
        .await;

        // Verify EA status is Offline
        let ea = ctx.connection_manager.get_master(&account_id).await;
        assert!(ea.is_some());
        assert_eq!(
            ea.unwrap().status,
            crate::domain::models::ConnectionStatus::Offline
        );

        ctx.cleanup().await;
    }

    // Note: test_master_unregister_updates_slave_runtime_status depends on the logic which is now in DisconnectionService.
    // Since create_test_context() likely uses a mock or real MessageHandler, we need to ensure MessageHandler is initialized with DisconnectionService.
    // If create_test_context creates a Real MessageHandler, we need to update test_helpers to provide DisconnectionService.
    // This test verifies the end-to-end behavior via MessageHandler.

    #[tokio::test]
    async fn test_master_unregister_updates_slave_runtime_status() {
        let ctx = create_test_context().await;
        let master_account = "MASTER_UNREGISTER_TRIGGER";
        let slave_account = "SLAVE_RUNTIME_SYNC";

        ctx.db.create_trade_group(master_account).await.unwrap();
        ctx.db
            .update_master_settings(
                master_account,
                crate::domain::models::MasterSettings {
                    enabled: true,
                    config_version: 1,
                    ..crate::domain::models::MasterSettings::default()
                },
            )
            .await
            .unwrap();

        ctx.db
            .add_member(
                master_account,
                slave_account,
                crate::domain::models::SlaveSettings::default(),
                STATUS_CONNECTED,
            )
            .await
            .unwrap();

        ctx.handle_heartbeat(build_heartbeat(master_account, "Master", true))
            .await;
        ctx.handle_heartbeat(build_heartbeat(slave_account, "Slave", true))
            .await;

        ctx.handle_unregister(UnregisterMessage {
            message_type: "Unregister".to_string(),
            account_id: master_account.to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            ea_type: Some("Master".to_string()),
        })
        .await;

        let member = ctx
            .db
            .get_member(master_account, slave_account)
            .await
            .unwrap()
            .expect("member should exist");
        assert_eq!(member.status, STATUS_ENABLED);

        let master_conn = ctx
            .connection_manager
            .get_master(master_account)
            .await
            .expect("master should remain tracked");
        assert_eq!(
            master_conn.status,
            crate::domain::models::ConnectionStatus::Offline
        );

        ctx.cleanup().await;
    }
}
