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
    EaConnection, HeartbeatMessage, VLogsGlobalSettings,
};
use crate::runtime_status_updater::{RuntimeStatusUpdater, SlaveRuntimeTarget};

impl MessageHandler {
    /// Handle heartbeat messages (auto-registration + health monitoring + is_trade_allowed notification)
    pub(super) async fn handle_heartbeat(&self, msg: HeartbeatMessage) {
        let account_id = msg.account_id.clone();
        let ea_type = msg.ea_type.clone();
        let new_is_trade_allowed = msg.is_trade_allowed;
        let runtime_updater = self.runtime_status_updater();

        tracing::info!(
            account = %account_id,
            ea_type = %ea_type,
            is_trade_allowed = new_is_trade_allowed,
            "[HEARTBEAT] Processing heartbeat message"
        );

        // Get old connection info before updating
        // Use ea_type from message to determine which lookup to use
        let old_conn = if ea_type == "Master" {
            self.connection_manager.get_master(&account_id).await
        } else {
            self.connection_manager.get_slave(&account_id).await
        };

        // Check if this is a new EA registration (not seen before)
        // Used for VLogs config broadcast
        let is_new_registration = old_conn.is_none();

        // Update heartbeat (performs auto-registration if needed)
        self.connection_manager.update_heartbeat(msg).await;

        // Send VictoriaLogs config to newly registered EAs
        if is_new_registration {
            self.send_vlogs_config_to_ea(&account_id).await;
        }

        // If this is a Master EA, handle status calculation and notification
        if ea_type == "Master" {
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
            let master_conn = self.connection_manager.get_master(&account_id).await;

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

            // Send MasterConfigMessage if state (status or warnings) changed
            // This uniformly handles new registrations (unknown -> new) and updates.

            // Calculate OLD Master status (or Unknown if previous connection didn't exist)
            let old_master_status = if let Some(conn) = old_conn.as_ref() {
                let old_snapshot = ConnectionSnapshot {
                    connection_status: Some(conn.status),
                    is_trade_allowed: conn.is_trade_allowed,
                };
                crate::models::status_engine::evaluate_master_status(
                    MasterIntent {
                        web_ui_enabled: trade_group.master_settings.enabled,
                    },
                    old_snapshot,
                )
            } else {
                crate::models::status_engine::MasterStatusResult::unknown()
            };

            // Compare with NEW status
            let master_changed = master_bundle.status_result.has_changed(&old_master_status);

            tracing::debug!(
                "Master {} change detection: changed={} (Old: {:?}) -> (New: {:?})",
                account_id,
                master_changed,
                old_master_status,
                master_bundle.status_result
            );

            // Send config if changed and notify Slaves
            if master_changed {
                let config = master_bundle.config;

                if let Err(e) = self.publisher.send(&config).await {
                    tracing::error!("Failed to send master config to {}: {}", account_id, e);
                } else {
                    tracing::info!(
                        "Sent Master CONFIG to {} (status: {}, reason: status_changed/new)",
                        account_id,
                        master_status
                    );
                }

                // Flag for batched WebSocket broadcast (will be triggered once after all processing)
                let mut should_broadcast = true;

                // Notify Slaves when Master status changes (N:N connection support)
                // Triggered only if Master status/warnings changed (which includes new registration and coming online)
                tracing::info!(
                    master = %account_id,
                    "Master heartbeat: status/warnings changed, re-evaluating connected Slaves"
                );

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

                            let slave_bundle = runtime_updater
                                .build_slave_bundle(SlaveRuntimeTarget {
                                    master_account: account_id.as_str(),
                                    trade_group_id: account_id.as_str(),
                                    slave_account: &slave_account,
                                    enabled_flag: member.enabled_flag,
                                    slave_settings: &member.slave_settings,
                                })
                                .await;

                            // Calculate OLD Slave Status
                            // Uses current slave settings but OLD Master Status
                            let slave_conn =
                                self.connection_manager.get_slave(&slave_account).await;
                            let slave_snapshot = crate::models::status_engine::ConnectionSnapshot {
                                connection_status: slave_conn.as_ref().map(|c| c.status),
                                is_trade_allowed: slave_conn
                                    .as_ref()
                                    .map(|c| c.is_trade_allowed)
                                    .unwrap_or(false),
                            };

                            let old_slave_result =
                                crate::models::status_engine::evaluate_member_status(
                                    crate::models::status_engine::SlaveIntent {
                                        web_ui_enabled: member.enabled_flag,
                                    },
                                    slave_snapshot,
                                    &old_master_status,
                                );

                            let new_slave_result = &slave_bundle.status_result;
                            let slave_changed = new_slave_result.has_changed(&old_slave_result);

                            tracing::debug!(
                                slave = %slave_account,
                                master = %account_id,
                                slave_changed = slave_changed,
                                old_status = old_slave_result.status,
                                new_status = new_slave_result.status,
                                "[Master Heartbeat] Slave status re-evaluated"
                            );

                            // Broadcast only if Slave specific status/warnings changed
                            if slave_changed {
                                // 1. Send ZMQ update to Slave EA
                                if let Err(e) = self.publisher.send(&slave_bundle.config).await {
                                    tracing::error!(
                                        "Failed to send config update to Slave {}: {}",
                                        slave_account,
                                        e
                                    );
                                } else {
                                    tracing::info!(
                                        "Sent Slave CONFIG to {} (status: {}, reason: master_changed)",
                                        slave_account,
                                        new_slave_result.status
                                    );
                                }

                                // 2. Flag for batched WebSocket broadcast
                                should_broadcast = true;

                                super::log_slave_runtime_trace(
                                    "master_heartbeat",
                                    &account_id,
                                    &slave_account,
                                    old_slave_result.status,
                                    new_slave_result.status,
                                    slave_bundle.status_result.allow_new_orders,
                                    &slave_bundle.status_result.warning_codes,
                                    1, // per-connection: always 1 Master
                                    new_slave_result.status == crate::models::STATUS_CONNECTED,
                                );

                                // Update database with new status
                                if let Err(e) = self
                                    .db
                                    .update_member_runtime_status(
                                        &account_id,
                                        &slave_account,
                                        new_slave_result.status,
                                    )
                                    .await
                                {
                                    tracing::error!(
                                        "Failed to update status for Slave {}: {}",
                                        slave_account,
                                        e
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to get members for Master {}: {}", account_id, e);
                    }
                }
                // Batched WebSocket broadcast: trigger once after all slaves processed
                if should_broadcast {
                    if let Some(broadcaster) = &self.snapshot_broadcaster {
                        let broadcaster = broadcaster.clone();
                        tokio::spawn(async move {
                            broadcaster.broadcast_now().await;
                        });
                    }
                }
            } // end of if master_changed
        } else {
            self.update_slave_runtime_on_heartbeat(&account_id, &runtime_updater, &old_conn)
                .await;
        }

        // Note: Individual heartbeat broadcasts are no longer sent here.
        // The SnapshotBroadcaster sends periodic connections_snapshot messages
        // to WebSocket clients when they are connected.
    }

    async fn update_slave_runtime_on_heartbeat(
        &self,
        slave_account: &str,
        runtime_updater: &RuntimeStatusUpdater,
        old_conn: &Option<EaConnection>,
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

            // Calculate OLD status result first (before CONFIG send decision)
            // This allows us to detect warning code changes (e.g., AutoTrading toggle)
            let old_status_result = if let Some(conn) = old_conn.as_ref() {
                let old_snapshot = ConnectionSnapshot {
                    connection_status: Some(conn.status),
                    is_trade_allowed: conn.is_trade_allowed,
                };

                runtime_updater
                    .evaluate_member_runtime_status_with_snapshot(
                        SlaveRuntimeTarget {
                            master_account: settings.master_account.as_str(),
                            trade_group_id: settings.master_account.as_str(),
                            slave_account: &settings.slave_account,
                            enabled_flag: settings.enabled_flag,
                            slave_settings: &settings.slave_settings,
                        },
                        old_snapshot,
                    )
                    .await
            } else {
                crate::models::status_engine::MemberStatusResult::unknown()
            };

            let previous_status = settings.status;
            let evaluated_status = slave_bundle.status_result.status;
            let is_connected = evaluated_status == crate::models::STATUS_CONNECTED;
            // Detect changes via either:
            // 1. Warning code changes (e.g., AutoTrading toggle) - has_changed()
            // 2. DB status changes (e.g., enabled_flag toggle via Web UI) - previous_status != evaluated_status
            let state_changed = slave_bundle.status_result.has_changed(&old_status_result)
                || previous_status != evaluated_status;

            super::log_slave_runtime_trace(
                "slave_heartbeat",
                &settings.master_account,
                &settings.slave_account,
                previous_status,
                evaluated_status,
                slave_bundle.status_result.allow_new_orders,
                &slave_bundle.status_result.warning_codes,
                1, // per-connection: always 1 Master
                is_connected,
            );

            // Send CONFIG if status OR warning_codes changed
            // This ensures AutoTrading toggle (is_trade_allowed change) triggers CONFIG re-send
            if state_changed {
                tracing::info!(
                    slave = %settings.slave_account,
                    master = %settings.master_account,
                    old_status = previous_status,
                    new_status = evaluated_status,
                    "Slave state changed via heartbeat (status or warnings)",
                );

                if let Err(err) = self.publisher.send(&slave_bundle.config).await {
                    tracing::error!(
                        "Failed to broadcast config to Slave {} on heartbeat: {}",
                        settings.slave_account,
                        err
                    );
                }

                // WebSocket broadcast
                if let Some(broadcaster) = &self.snapshot_broadcaster {
                    let broadcaster = broadcaster.clone();
                    tokio::spawn(async move {
                        broadcaster.broadcast_now().await;
                    });
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
