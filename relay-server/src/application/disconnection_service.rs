use std::sync::Arc;

use async_trait::async_trait;
use tracing::{error, info};

use crate::{
    adapters::outbound::persistence::Database,
    domain::models::SlaveConfigWithMaster,
    domain::services::status_calculator::SlaveRuntimeTarget,
    ports::{ConfigPublisher, ConnectionManager, DisconnectionService, UpdateBroadcaster},
};

use super::runtime_status_updater::{
    log_slave_runtime_trace, RuntimeStatusMetrics, RuntimeStatusUpdater,
};

pub struct RealDisconnectionService {
    connection_manager: Arc<dyn ConnectionManager>,
    db: Arc<Database>,
    publisher: Arc<dyn ConfigPublisher>,
    broadcaster: Arc<dyn UpdateBroadcaster>,
    metrics: Arc<RuntimeStatusMetrics>,
}

impl RealDisconnectionService {
    pub fn new(
        connection_manager: Arc<dyn ConnectionManager>,
        db: Arc<Database>,
        publisher: Arc<dyn ConfigPublisher>,
        broadcaster: Arc<dyn UpdateBroadcaster>,
        metrics: Arc<RuntimeStatusMetrics>,
    ) -> Self {
        Self {
            connection_manager,
            db,
            publisher,
            broadcaster,
            metrics,
        }
    }

    fn runtime_updater(&self) -> RuntimeStatusUpdater {
        RuntimeStatusUpdater::with_metrics(
            self.db.clone(),
            self.connection_manager.clone(),
            self.metrics.clone(),
        )
    }
}

#[async_trait]
impl DisconnectionService for RealDisconnectionService {
    async fn handle_master_offline(&self, master_account: &str) {
        // Update DB: all CONNECTED slaves should become ENABLED
        match self.db.update_master_statuses_enabled(master_account).await {
            Ok(count) if count > 0 => {
                info!(
                    "Master {} disconnected: updated {} settings to ENABLED",
                    master_account, count
                );
            }
            Ok(_) => {
                // No settings updated (no connected settings for this master)
            }
            Err(e) => {
                error!(
                    "Failed to update master statuses for disconnect {}: {}",
                    master_account, e
                );
            }
        }

        let runtime_updater = self.runtime_updater();

        match self.db.get_members(master_account).await {
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

                    log_slave_runtime_trace(
                        "master_offline",
                        master_account,
                        &member.slave_account,
                        member.status,
                        new_status,
                        slave_bundle.status_result.allow_new_orders,
                        &slave_bundle.status_result.warning_codes,
                        1, // per-connection: always 1 Master
                        new_status == crate::domain::models::STATUS_CONNECTED,
                    );

                    if let Err(e) = self.publisher.send_slave_config(&config).await {
                        error!(
                            "Failed to send config to {} on Master disconnect: {}",
                            member.slave_account, e
                        );
                    } else {
                        info!(
                            "Notified {} (status: {}) of Master {} disconnect",
                            member.slave_account, new_status, master_account
                        );
                    }

                    if let Err(err) = self
                        .db
                        .update_member_runtime_status(
                            master_account,
                            &member.slave_account,
                            new_status,
                        )
                        .await
                    {
                        error!(
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
                        self.broadcaster.broadcast_settings_updated(&json).await;
                    }
                }
            }
            Err(e) => {
                error!(
                    "Failed to get members for Master {} disconnect: {}",
                    master_account, e
                );
            }
        }
    }

    async fn handle_slave_offline(&self, slave_account: &str) {
        let runtime_updater = self.runtime_updater();

        // Get all trade group memberships for this Slave
        let settings_list = match self.db.get_settings_for_slave(slave_account).await {
            Ok(list) => list,
            Err(err) => {
                error!(
                    "Failed to fetch settings for Slave {} during offline notification: {}",
                    slave_account, err
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

            log_slave_runtime_trace(
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
            if let Err(err) = self
                .db
                .update_member_runtime_status(&settings.master_account, slave_account, new_status)
                .await
            {
                error!(
                    "Failed to persist runtime status for Slave {} (master {}): {}",
                    settings.slave_account, settings.master_account, err
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
                    self.broadcaster.broadcast_settings_updated(&json).await;
                    info!(
                        "Slave {} offline: broadcast sent (status {} -> {}, master: {})",
                        slave_account, previous_status, new_status, settings.master_account
                    );
                }
            }
        }
    }
}
