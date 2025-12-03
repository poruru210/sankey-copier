//! Unregister message handler
//!
//! Handles EA unregistration messages, updating connection status and notifying clients.
//! When a Master EA disconnects, notifies all Slaves so they can update their status.

use super::MessageHandler;
use crate::{
    connection_manager::ConnectionManager,
    db::Database,
    models::{EaType, SlaveConfigWithMaster, UnregisterMessage},
    runtime_status_updater::{RuntimeStatusMetrics, RuntimeStatusUpdater, SlaveRuntimeTarget},
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
                self.runtime_status_metrics.clone(),
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
                let cluster_snapshot = runtime_updater
                    .master_cluster_snapshot(&member.slave_account)
                    .await;
                let slave_bundle = runtime_updater
                    .build_slave_bundle(
                        SlaveRuntimeTarget {
                            master_account,
                            trade_group_id: master_account,
                            slave_account: &member.slave_account,
                            enabled_flag: member.enabled_flag,
                            slave_settings: &member.slave_settings,
                        },
                        Some(cluster_snapshot.clone()),
                    )
                    .await;
                let config = slave_bundle.config;
                let new_status = slave_bundle.status_result.status;

                super::log_slave_runtime_trace(
                    "master_unregister",
                    master_account,
                    &member.slave_account,
                    member.runtime_status,
                    new_status,
                    slave_bundle.status_result.allow_new_orders,
                    &slave_bundle.status_result.warning_codes,
                    cluster_snapshot.master_statuses.len(),
                    cluster_snapshot.all_connected(),
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
