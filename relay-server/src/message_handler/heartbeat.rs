//! Heartbeat message handler
//!
//! Handles heartbeat messages for health monitoring, auto-registration, and
//! Master EA is_trade_allowed change notifications to Slave EAs.
//!
//! Status calculation flow:
//! - Master: DISABLED (web_ui OFF or !is_trade_allowed) or CONNECTED
//! - Slave: DISABLED (web_ui OFF or !is_trade_allowed) or ENABLED (master not connected) or CONNECTED

use super::MessageHandler;
use crate::config_builder::{ConfigBuilder, MasterConfigContext};
use crate::models::{
    status_engine::{ConnectionSnapshot, MasterIntent},
    HeartbeatMessage, SlaveConfigWithMaster, VLogsGlobalSettings, STATUS_CONNECTED,
};
use crate::runtime_status_updater::{RuntimeStatusUpdater, SlaveRuntimeTarget};

impl MessageHandler {
    /// Handle heartbeat messages (auto-registration + health monitoring + is_trade_allowed notification)
    pub(super) async fn handle_heartbeat(&self, msg: HeartbeatMessage) {
        let account_id = msg.account_id.clone();
        let balance = msg.balance;
        let equity = msg.equity;
        let ea_type = msg.ea_type.clone();
        let new_is_trade_allowed = msg.is_trade_allowed;
        let runtime_updater = self.runtime_status_updater();

        // Get old is_trade_allowed before updating
        let old_is_trade_allowed = self
            .connection_manager
            .get_ea(&account_id)
            .await
            .map(|conn| conn.is_trade_allowed);

        // Check if this is a new EA registration (not seen before)
        let is_new_registration = old_is_trade_allowed.is_none();

        // Update heartbeat (performs auto-registration if needed)
        self.connection_manager.update_heartbeat(msg).await;

        // Send VictoriaLogs config to newly registered EAs
        if is_new_registration {
            self.send_vlogs_config_to_ea(&account_id).await;
        }

        // If this is a Master EA, handle status calculation and notification
        if ea_type == "Master" {
            // Detect is_trade_allowed change
            let trade_allowed_changed = old_is_trade_allowed != Some(new_is_trade_allowed);

            // Fetch TradeGroup to get master_settings.enabled
            let trade_group = match self.db.get_trade_group(&account_id).await {
                Ok(Some(tg)) => tg,
                Ok(None) => {
                    tracing::debug!(
                        "No TradeGroup found for Master {} (new connection without config)",
                        account_id
                    );
                    return;
                }
                Err(e) => {
                    tracing::error!("Failed to get TradeGroup for {}: {}", account_id, e);
                    return;
                }
            };

            // Get Master connection info
            let master_conn = self.connection_manager.get_ea(&account_id).await;

            let master_bundle = ConfigBuilder::build_master_config(MasterConfigContext {
                account_id: account_id.clone(),
                intent: MasterIntent {
                    web_ui_enabled: trade_group.master_settings.enabled,
                },
                connection_snapshot: ConnectionSnapshot {
                    connection_status: master_conn.as_ref().map(|c| c.status),
                    is_trade_allowed: new_is_trade_allowed,
                },
                settings: &trade_group.master_settings,
                timestamp: chrono::Utc::now(),
            });
            let master_status = master_bundle.status_result.status;

            tracing::debug!(
                "Master {} status calculated: {} (enabled={}, is_trade_allowed={})",
                account_id,
                master_status,
                trade_group.master_settings.enabled,
                new_is_trade_allowed
            );

            // Send MasterConfigMessage if this is a new registration or if auto-trading state changed
            // This ensures Master EA is in sync with Server status (e.g. after Server restart or local toggle)
            if is_new_registration || trade_allowed_changed {
                let config = master_bundle.config;

                if let Err(e) = self.publisher.send(&config).await {
                    tracing::error!("Failed to send master config to {}: {}", account_id, e);
                } else {
                    tracing::info!(
                        "Sent Master CONFIG to {} (status: {}, reason: {})",
                        account_id,
                        master_status,
                        if is_new_registration {
                            "new_registration"
                        } else {
                            "trade_allowed_changed"
                        }
                    );
                }
            }

            // Notify Slaves when Master status changes (N:N connection support)
            // Only send SlaveConfigMessage when Slave's calculated status changes
            if trade_allowed_changed || is_new_registration {
                if trade_allowed_changed {
                    tracing::info!(
                        "Master {} is_trade_allowed changed: {:?} -> {}",
                        account_id,
                        old_is_trade_allowed,
                        new_is_trade_allowed
                    );
                }

                // Get all Slaves connected to this Master
                match self.db.get_members(&account_id).await {
                    Ok(members) => {
                        // Track which Slaves we've already processed to avoid duplicates
                        let mut processed_slaves = std::collections::HashSet::new();

                        for member in members {
                            let slave_account = member.slave_account.clone();

                            if processed_slaves.contains(&slave_account) {
                                continue;
                            }
                            processed_slaves.insert(slave_account.clone());

                            let masters_snapshot = runtime_updater
                                .master_cluster_snapshot(&slave_account)
                                .await;

                            let slave_bundle = runtime_updater
                                .build_slave_bundle(
                                    SlaveRuntimeTarget {
                                        master_account: account_id.as_str(),
                                        trade_group_id: account_id.as_str(),
                                        slave_account: &slave_account,
                                        enabled_flag: member.enabled_flag,
                                        slave_settings: &member.slave_settings,
                                    },
                                    Some(masters_snapshot.clone()),
                                )
                                .await;
                            let new_slave_status = slave_bundle.status_result.status;

                            // Compare with previous status
                            // Always send config on new registration or trade_allowed_changed
                            // When Master's is_trade_allowed changes, we must notify Slave even if Slave's status doesn't change
                            // (e.g., Slave stays ENABLED when Master goes from CONNECTED to DISABLED)
                            let old_slave_status = member.status;

                            // Debug logging to diagnose notification issues
                            tracing::info!(
                                slave = %slave_account,
                                master = %account_id,
                                old_slave_status = old_slave_status,
                                new_slave_status = new_slave_status,
                                is_new_registration = is_new_registration,
                                trade_allowed_changed = trade_allowed_changed,
                                all_masters_connected = masters_snapshot.all_connected(),
                                slave_online = slave_bundle.config.allow_new_orders || new_slave_status != 0,
                                "Master heartbeat: evaluating Slave notification"
                            );

                            if !is_new_registration
                                && !trade_allowed_changed
                                && new_slave_status == old_slave_status
                            {
                                // Status unchanged, no trade_allowed change, and not a new registration - skip sending config
                                tracing::debug!(
                                    "Slave {} status unchanged ({}) and no Master trade_allowed change, skipping config send",
                                    slave_account,
                                    new_slave_status
                                );
                                continue;
                            }

                            let master_cluster_size = masters_snapshot.master_statuses.len();
                            let masters_all_connected = masters_snapshot.all_connected();
                            super::log_slave_runtime_trace(
                                "master_heartbeat",
                                &account_id,
                                &slave_account,
                                old_slave_status,
                                new_slave_status,
                                slave_bundle.status_result.allow_new_orders,
                                &slave_bundle.status_result.warning_codes,
                                master_cluster_size,
                                masters_all_connected,
                            );

                            // Status changed, trade_allowed changed, or new registration - send SlaveConfigMessage
                            if is_new_registration {
                                tracing::info!(
                                    "Slave {} new registration, sending initial config (status: {})",
                                    slave_account,
                                    new_slave_status
                                );
                            } else if old_slave_status != new_slave_status {
                                tracing::info!(
                                    "Slave {} status changed: {} -> {} (Master {} heartbeat)",
                                    slave_account,
                                    old_slave_status,
                                    new_slave_status,
                                    account_id
                                );
                            } else {
                                tracing::info!(
                                    "Slave {} status unchanged ({}) but notifying due to Master {} trade_allowed change",
                                    slave_account,
                                    new_slave_status,
                                    account_id
                                );
                            }

                            let config = slave_bundle.config;

                            if let Err(e) = self.publisher.send(&config).await {
                                tracing::error!(
                                    "Failed to send config to {}: {}",
                                    slave_account,
                                    e
                                );
                            } else {
                                tracing::info!(
                                    "Sent config to {} (status: {})",
                                    slave_account,
                                    new_slave_status
                                );
                            }

                            // Update database with new status
                            if let Err(e) = self
                                .db
                                .update_member_runtime_status(
                                    &account_id,
                                    &slave_account,
                                    new_slave_status,
                                )
                                .await
                            {
                                tracing::error!(
                                    "Failed to update status for Slave {}: {}",
                                    slave_account,
                                    e
                                );
                            }

                            // Broadcast runtime status change to WebSocket clients (on Master heartbeat)
                            if old_slave_status != new_slave_status {
                                let settings_with_master = SlaveConfigWithMaster {
                                    master_account: account_id.clone(),
                                    slave_account: slave_account.clone(),
                                    status: member.status,
                                    runtime_status: new_slave_status,
                                    enabled_flag: member.enabled_flag,
                                    slave_settings: member.slave_settings.clone(),
                                };
                                if let Ok(json) = serde_json::to_string(&settings_with_master) {
                                    let _ = self
                                        .broadcast_tx
                                        .send(format!("settings_updated:{}", json));
                                    tracing::debug!(
                                        "Broadcasted runtime status change for Slave {} via Master {} heartbeat: {} -> {}",
                                        slave_account,
                                        account_id,
                                        old_slave_status,
                                        new_slave_status
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to get members for Master {}: {}", account_id, e);
                    }
                }
            }

            // Update DB status for all slaves based on master connection state
            // Only if master is now CONNECTED
            if master_status == STATUS_CONNECTED {
                match self.db.update_master_statuses_connected(&account_id).await {
                    Ok(count) if count > 0 => {
                        tracing::info!(
                            "Master {} connected: updated {} settings to CONNECTED",
                            account_id,
                            count
                        );
                        // Notify WebSocket clients
                        if let Ok(members) = self.db.get_members(&account_id).await {
                            for member in members {
                                let settings_with_master = SlaveConfigWithMaster {
                                    master_account: account_id.clone(),
                                    slave_account: member.slave_account.clone(),
                                    status: member.status,
                                    runtime_status: member.runtime_status,
                                    enabled_flag: member.enabled_flag,
                                    slave_settings: member.slave_settings.clone(),
                                };
                                if let Ok(json) = serde_json::to_string(&settings_with_master) {
                                    let _ = self
                                        .broadcast_tx
                                        .send(format!("settings_updated:{}", json));
                                }
                            }
                        }
                    }
                    Ok(_) => {
                        // No settings updated (no enabled settings for this master)
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to update master statuses for {}: {}",
                            account_id,
                            e
                        );
                    }
                }
            }
        } else {
            self.update_slave_runtime_on_heartbeat(&account_id, &runtime_updater)
                .await;
        }

        // Notify WebSocket clients of heartbeat
        let _ = self.broadcast_tx.send(format!(
            "ea_heartbeat:{}:{:.2}:{:.2}",
            account_id, balance, equity
        ));
    }

    async fn update_slave_runtime_on_heartbeat(
        &self,
        slave_account: &str,
        runtime_updater: &RuntimeStatusUpdater,
    ) {
        let settings_list = match self.db.get_settings_for_slave(slave_account).await {
            Ok(list) => list,
            Err(err) => {
                tracing::error!(
                    "Failed to fetch settings for Slave {} during heartbeat: {}",
                    slave_account,
                    err
                );
                return;
            }
        };

        if settings_list.is_empty() {
            tracing::debug!(
                "Skipping Slave {} heartbeat runtime evaluation (no trade groups)",
                slave_account
            );
            return;
        }

        let master_snapshot = runtime_updater.master_cluster_snapshot(slave_account).await;

        for settings in settings_list {
            let slave_bundle = runtime_updater
                .build_slave_bundle(
                    SlaveRuntimeTarget {
                        master_account: settings.master_account.as_str(),
                        trade_group_id: settings.master_account.as_str(),
                        slave_account: &settings.slave_account,
                        enabled_flag: settings.enabled_flag,
                        slave_settings: &settings.slave_settings,
                    },
                    Some(master_snapshot.clone()),
                )
                .await;

            let previous_status = settings.runtime_status;
            let evaluated_status = slave_bundle.status_result.status;

            super::log_slave_runtime_trace(
                "slave_heartbeat",
                &settings.master_account,
                &settings.slave_account,
                previous_status,
                evaluated_status,
                slave_bundle.status_result.allow_new_orders,
                &slave_bundle.status_result.warning_codes,
                master_snapshot.master_statuses.len(),
                master_snapshot.all_connected(),
            );

            if evaluated_status != previous_status {
                tracing::info!(
                    slave = %settings.slave_account,
                    master = %settings.master_account,
                    old = previous_status,
                    new = evaluated_status,
                    "Slave runtime status changed via heartbeat",
                );

                if let Err(err) = self.publisher.send(&slave_bundle.config).await {
                    tracing::error!(
                        "Failed to broadcast config to Slave {} on heartbeat: {}",
                        settings.slave_account,
                        err
                    );
                }
            }

            if let Err(err) = self
                .db
                .update_member_runtime_status(
                    &settings.master_account,
                    slave_account,
                    evaluated_status,
                )
                .await
            {
                tracing::error!(
                    "Failed to persist runtime status for Slave {} (master {}): {}",
                    settings.slave_account,
                    settings.master_account,
                    err
                );
            }

            // Broadcast runtime status change to WebSocket clients
            if evaluated_status != previous_status {
                let settings_with_master = SlaveConfigWithMaster {
                    master_account: settings.master_account.clone(),
                    slave_account: settings.slave_account.clone(),
                    status: settings.status,
                    runtime_status: evaluated_status,
                    enabled_flag: settings.enabled_flag,
                    slave_settings: settings.slave_settings.clone(),
                };
                if let Ok(json) = serde_json::to_string(&settings_with_master) {
                    let _ = self.broadcast_tx.send(format!("settings_updated:{}", json));
                    tracing::debug!(
                        "Broadcasted runtime status change for Slave {} (master {}): {} -> {}",
                        settings.slave_account,
                        settings.master_account,
                        previous_status,
                        evaluated_status
                    );
                }
            }
        }
    }

    /// Send VictoriaLogs configuration to a specific EA on registration
    async fn send_vlogs_config_to_ea(&self, account_id: &str) {
        // Get VictoriaLogs settings from controller (config.toml based)
        let Some(controller) = &self.vlogs_controller else {
            tracing::debug!(
                account_id = %account_id,
                "VictoriaLogs not configured, skipping config broadcast"
            );
            return;
        };

        let config = controller.config();
        let settings = VLogsGlobalSettings {
            enabled: controller.is_enabled(),
            endpoint: config.endpoint(),
            batch_size: config.batch_size as i32,
            flush_interval_secs: config.flush_interval_secs as i32,
            log_level: "INFO".to_string(),
        };

        if let Err(e) = self.publisher.broadcast_vlogs_config(&settings).await {
            tracing::error!(
                account_id = %account_id,
                error = %e,
                "Failed to send VictoriaLogs config to newly registered EA"
            );
        } else {
            tracing::info!(
                account_id = %account_id,
                enabled = settings.enabled,
                "Sent VictoriaLogs config to newly registered EA"
            );
        }
    }
}
