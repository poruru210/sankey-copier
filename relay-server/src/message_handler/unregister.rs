//! Unregister message handler
//!
//! Handles EA unregistration messages, updating connection status and notifying clients.
//! When a Master EA disconnects, notifies all Slaves so they can update their status.

use super::MessageHandler;
use crate::config_builder::{ConfigBuilder, SlaveConfigContext};
use crate::{
    connection_manager::ConnectionManager,
    db::Database,
    models::{
        status_engine::{
            evaluate_master_status, ConnectionSnapshot, MasterClusterSnapshot, MasterIntent,
            SlaveIntent,
        },
        EaType, SlaveConfigWithMaster, UnregisterMessage,
    },
    zeromq::ZmqConfigPublisher,
};
use std::sync::Arc;
use tokio::sync::broadcast;

impl MessageHandler {
    /// Handle EA unregistration
    /// When a Master disconnects, notify all Slaves to update their status from CONNECTED to ENABLED
    pub(super) async fn handle_unregister(&self, msg: UnregisterMessage) {
        let account_id = &msg.account_id;

        // Get EA type before unregistering
        let ea_type = self
            .connection_manager
            .get_ea(account_id)
            .await
            .map(|conn| conn.ea_type);

        // Unregister the EA
        self.connection_manager.unregister_ea(account_id).await;

        // Notify WebSocket clients
        let _ = self
            .broadcast_tx
            .send(format!("ea_disconnected:{}", account_id));

        // If this was a Master EA, notify all Slaves
        if ea_type == Some(EaType::Master) {
            tracing::info!("Master {} disconnected, notifying Slaves", account_id);

            // Update DB: all CONNECTED slaves should become ENABLED
            match self
                .db
                .update_master_statuses_disconnected(account_id)
                .await
            {
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
                account_id,
            )
            .await;
        }
    }
}

pub(crate) async fn notify_slaves_master_offline(
    connection_manager: &Arc<ConnectionManager>,
    db: &Arc<Database>,
    publisher: &Arc<ZmqConfigPublisher>,
    broadcast_tx: &broadcast::Sender<String>,
    master_account: &str,
) {
    // Resolve Master's intent for consistent status evaluation
    let master_web_ui_enabled = match db.get_trade_group(master_account).await {
        Ok(Some(tg)) => tg.master_settings.enabled,
        _ => true,
    };

    let master_result = evaluate_master_status(
        MasterIntent {
            web_ui_enabled: master_web_ui_enabled,
        },
        ConnectionSnapshot {
            connection_status: None,
            is_trade_allowed: false,
        },
    );
    let master_cluster = MasterClusterSnapshot::new(vec![master_result.status]);

    match db.get_members(master_account).await {
        Ok(members) => {
            for member in members {
                let slave_conn = connection_manager.get_ea(&member.slave_account).await;
                let slave_snapshot = ConnectionSnapshot {
                    connection_status: slave_conn.as_ref().map(|c| c.status),
                    is_trade_allowed: slave_conn
                        .as_ref()
                        .map(|c| c.is_trade_allowed)
                        .unwrap_or(false),
                };

                let slave_bundle = ConfigBuilder::build_slave_config(SlaveConfigContext {
                    slave_account: member.slave_account.clone(),
                    master_account: master_account.to_string(),
                    trade_group_id: master_account.to_string(),
                    intent: SlaveIntent {
                        web_ui_enabled: member.enabled_flag,
                    },
                    slave_connection_snapshot: slave_snapshot,
                    master_cluster: master_cluster.clone(),
                    slave_settings: &member.slave_settings,
                    master_equity: None,
                    timestamp: chrono::Utc::now(),
                });
                let config = slave_bundle.config;
                let new_status = slave_bundle.status_result.status;

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

                let settings_with_master = SlaveConfigWithMaster {
                    master_account: master_account.to_string(),
                    slave_account: member.slave_account.clone(),
                    status: new_status,
                    runtime_status: new_status,
                    enabled_flag: member.enabled_flag,
                    slave_settings: member.slave_settings.clone(),
                };
                if let Ok(json) = serde_json::to_string(&settings_with_master) {
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
