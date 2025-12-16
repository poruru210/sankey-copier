//! Unregister message handler
//!
//! Handles EA unregistration messages, updating connection status and notifying clients.
//! When a Master EA disconnects, notifies all Slaves so they can update their status.
//! When a Slave EA disconnects, updates runtime status and notifies WebSocket clients.

use super::MessageHandler;
use crate::{
    adapters::infrastructure::connection_manager::ConnectionManager,
    adapters::outbound::messaging::ZmqConfigPublisher,
    adapters::outbound::persistence::Database,
    domain::models::{EaType, SlaveConfigWithMaster, UnregisterMessage},
    domain::services::status_calculator::SlaveRuntimeTarget,
    runtime_status_updater::{RuntimeStatusMetrics, RuntimeStatusUpdater},
};
use std::sync::Arc;
use tokio::sync::broadcast;

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
        let _ = self
            .broadcast_tx
            .send(format!("ea_disconnected:{}", account_id));

        match ea_type {
            Some(EaType::Master) => {
                // Master disconnected - notify all Slaves
                tracing::info!("Master {} disconnected, notifying Slaves", account_id);

                // Update DB: all CONNECTED slaves should become ENABLED
                match self.db.update_master_statuses_enabled(account_id).await {
                    Ok(count) if count > 0 => {
                        tracing::info!(
                            "Master {} disconnected: updated {} settings to ENABLED",
                            account_id,
                            count
                        );
                    }
                    Ok(_) => {
                        // No settings updated (no connected settings for this master)
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to update master statuses for disconnect {}: {}",
                            account_id,
                            e
                        );
                    }
                }

                notify_slaves_master_offline(
                    &self.connection_manager,
                    &self.db,
                    &self.publisher,
                    &self.broadcast_tx,
                    self.runtime_status_metrics.clone(),
                    account_id,
                )
                .await;
            }
            Some(EaType::Slave) => {
                // Slave disconnected - update runtime status and notify WebSocket
                tracing::info!("Slave {} disconnected, updating runtime status", account_id);

                notify_slave_offline(
                    &self.connection_manager,
                    &self.db,
                    &self.broadcast_tx,
                    self.runtime_status_metrics.clone(),
                    account_id,
                )
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

pub(crate) async fn notify_slaves_master_offline(
    connection_manager: &Arc<ConnectionManager>,
    db: &Arc<Database>,
    publisher: &Arc<ZmqConfigPublisher>,
    broadcast_tx: &broadcast::Sender<String>,
    runtime_status_metrics: Arc<RuntimeStatusMetrics>,
    master_account: &str,
) {
    let runtime_updater = RuntimeStatusUpdater::with_metrics(
        db.clone(),
        connection_manager.clone(),
        runtime_status_metrics,
    );
    match db.get_members(master_account).await {
        Ok(members) => {
            for member in members {
                let slave_bundle = runtime_updater
                    .build_slave_bundle(SlaveRuntimeTarget {
                        master_account,
                        trade_group_id: master_account,
                        slave_account: &member.slave_account,
                        enabled_flag: member.enabled_flag,
                        slave_settings: &member.slave_settings,
                    })
                    .await;
                let config = slave_bundle.config;
                let new_status = slave_bundle.status_result.status;

                super::log_slave_runtime_trace(
                    "master_unregister",
                    master_account,
                    &member.slave_account,
                    member.status,
                    new_status,
                    slave_bundle.status_result.allow_new_orders,
                    &slave_bundle.status_result.warning_codes,
                    1, // per-connection: always 1 Master
                    new_status == crate::domain::models::STATUS_CONNECTED,
                );

                if let Err(e) = publisher.send(&config).await {
                    tracing::error!(
                        "Failed to send config to {} on Master disconnect: {}",
                        member.slave_account,
                        e
                    );
                } else {
                    tracing::info!(
                        "Notified {} (status: {}) of Master {} disconnect",
                        member.slave_account,
                        new_status,
                        master_account
                    );
                }

                if let Err(err) = db
                    .update_member_runtime_status(master_account, &member.slave_account, new_status)
                    .await
                {
                    tracing::error!(
                        slave = %member.slave_account,
                        master = %master_account,
                        status = new_status,
                        error = %err,
                        "Failed to persist runtime status after master disconnect"
                    );
                }

                // WebSocket broadcast on Master disconnect
                let payload = SlaveConfigWithMaster {
                    master_account: master_account.to_string(),
                    slave_account: member.slave_account.clone(),
                    status: new_status,
                    enabled_flag: member.enabled_flag,
                    warning_codes: slave_bundle.status_result.warning_codes.clone(),
                    slave_settings: member.slave_settings.clone(),
                };

                if let Ok(json) = serde_json::to_string(&payload) {
                    let _ = broadcast_tx.send(format!("settings_updated:{}", json));
                }
            }
        }
        Err(e) => {
            tracing::error!(
                "Failed to get members for Master {} disconnect: {}",
                master_account,
                e
            );
        }
    }
}

/// Notify WebSocket clients when a Slave EA goes offline
/// Updates status in DB and broadcasts to WebSocket clients
pub(crate) async fn notify_slave_offline(
    connection_manager: &Arc<ConnectionManager>,
    db: &Arc<Database>,
    broadcast_tx: &broadcast::Sender<String>,
    runtime_status_metrics: Arc<RuntimeStatusMetrics>,
    slave_account: &str,
) {
    let runtime_updater = RuntimeStatusUpdater::with_metrics(
        db.clone(),
        connection_manager.clone(),
        runtime_status_metrics,
    );

    // Get all trade group memberships for this Slave
    let settings_list = match db.get_settings_for_slave(slave_account).await {
        Ok(list) => list,
        Err(err) => {
            tracing::error!(
                "Failed to fetch settings for Slave {} during offline notification: {}",
                slave_account,
                err
            );
            return;
        }
    };

    if settings_list.is_empty() {
        tracing::debug!(
            "No trade group settings found for Slave {} during offline notification",
            slave_account
        );
        return;
    }

    for settings in settings_list {
        let slave_bundle = runtime_updater
            .build_slave_bundle(SlaveRuntimeTarget {
                master_account: settings.master_account.as_str(),
                trade_group_id: settings.master_account.as_str(),
                slave_account: &settings.slave_account,
                enabled_flag: settings.enabled_flag,
                slave_settings: &settings.slave_settings,
            })
            .await;

        let previous_status = settings.status;
        let new_status = slave_bundle.status_result.status;

        super::log_slave_runtime_trace(
            "slave_offline",
            &settings.master_account,
            &settings.slave_account,
            previous_status,
            new_status,
            slave_bundle.status_result.allow_new_orders,
            &slave_bundle.status_result.warning_codes,
            1, // per-connection: always 1 Master
            new_status == crate::domain::models::STATUS_CONNECTED,
        );

        // Update database with new status
        if let Err(err) = db
            .update_member_runtime_status(&settings.master_account, slave_account, new_status)
            .await
        {
            tracing::error!(
                "Failed to persist runtime status for Slave {} (master {}): {}",
                settings.slave_account,
                settings.master_account,
                err
            );
        }

        // WebSocket broadcast on status change
        let status_changed = new_status != previous_status;
        if status_changed {
            let payload = SlaveConfigWithMaster {
                master_account: settings.master_account.clone(),
                slave_account: settings.slave_account.clone(),
                status: new_status,
                enabled_flag: settings.enabled_flag,
                warning_codes: slave_bundle.status_result.warning_codes.clone(),
                slave_settings: settings.slave_settings.clone(),
            };

            if let Ok(json) = serde_json::to_string(&payload) {
                let _ = broadcast_tx.send(format!("settings_updated:{}", json));
                tracing::info!(
                    "Slave {} offline: broadcast sent (status {} -> {}, master: {})",
                    slave_account,
                    previous_status,
                    new_status,
                    settings.master_account
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
