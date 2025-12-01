//! Heartbeat message handler
//!
//! Handles heartbeat messages for health monitoring, auto-registration, and
//! Master EA is_trade_allowed change notifications to Slave EAs.
//!
//! Status calculation flow:
//! - Master: DISABLED (web_ui OFF or !is_trade_allowed) or CONNECTED
//! - Slave: DISABLED (web_ui OFF or !is_trade_allowed) or ENABLED (master not connected) or CONNECTED

use super::MessageHandler;
use crate::models::{
    status::{calculate_master_status, MasterStatusInput},
    HeartbeatMessage, SlaveConfigMessage, SlaveConfigWithMaster, VLogsGlobalSettings,
    STATUS_CONNECTED, STATUS_DISABLED, STATUS_ENABLED,
};

impl MessageHandler {
    /// Handle heartbeat messages (auto-registration + health monitoring + is_trade_allowed notification)
    pub(super) async fn handle_heartbeat(&self, msg: HeartbeatMessage) {
        let account_id = msg.account_id.clone();
        let balance = msg.balance;
        let equity = msg.equity;
        let ea_type = msg.ea_type.clone();
        let new_is_trade_allowed = msg.is_trade_allowed;

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

            // Calculate Master status using centralized logic
            let master_status = calculate_master_status(&MasterStatusInput {
                web_ui_enabled: trade_group.master_settings.enabled,
                connection_status: master_conn.as_ref().map(|c| c.status),
                is_trade_allowed: new_is_trade_allowed,
            });

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
                let config = crate::models::MasterConfigMessage {
                    account_id: account_id.clone(),
                    status: master_status,
                    symbol_prefix: trade_group.master_settings.symbol_prefix.clone(),
                    symbol_suffix: trade_group.master_settings.symbol_suffix.clone(),
                    config_version: trade_group.master_settings.config_version,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                };

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

                            // Skip if we've already processed this Slave
                            if processed_slaves.contains(&slave_account) {
                                continue;
                            }
                            processed_slaves.insert(slave_account.clone());

                            // Get ALL Masters this Slave is connected to
                            let all_masters =
                                match self.db.get_masters_for_slave(&slave_account).await {
                                    Ok(masters) => masters,
                                    Err(e) => {
                                        tracing::error!(
                                            "Failed to get masters for Slave {}: {}",
                                            slave_account,
                                            e
                                        );
                                        continue;
                                    }
                                };

                            // Check if ALL Masters are CONNECTED
                            let mut all_masters_connected = true;
                            for master_account in &all_masters {
                                // Get Master's TradeGroup to check web_ui_enabled
                                let master_enabled =
                                    match self.db.get_trade_group(master_account).await {
                                        Ok(Some(tg)) => tg.master_settings.enabled,
                                        Ok(None) => {
                                            // Master not found, treat as disabled
                                            all_masters_connected = false;
                                            break;
                                        }
                                        Err(e) => {
                                            tracing::error!(
                                                "Failed to get TradeGroup for Master {}: {}",
                                                master_account,
                                                e
                                            );
                                            all_masters_connected = false;
                                            break;
                                        }
                                    };

                                // Get Master connection info
                                let master_conn =
                                    self.connection_manager.get_ea(master_account).await;
                                let master_is_trade_allowed = master_conn
                                    .as_ref()
                                    .map(|c| c.is_trade_allowed)
                                    .unwrap_or(false);

                                // Calculate Master status
                                let master_status = calculate_master_status(&MasterStatusInput {
                                    web_ui_enabled: master_enabled,
                                    connection_status: master_conn.as_ref().map(|c| c.status),
                                    is_trade_allowed: master_is_trade_allowed,
                                });

                                if master_status != STATUS_CONNECTED {
                                    all_masters_connected = false;
                                    break;
                                }
                            }

                            // Get Slave's is_trade_allowed
                            let slave_is_trade_allowed = self
                                .connection_manager
                                .get_ea(&slave_account)
                                .await
                                .map(|conn| conn.is_trade_allowed)
                                .unwrap_or(true); // Default to true if not connected

                            // Calculate Slave status
                            // Slave is enabled if member.status > 0 (was previously enabled in DB)
                            let slave_enabled = member.status > 0;
                            let new_slave_status = if !slave_enabled || !slave_is_trade_allowed {
                                STATUS_DISABLED
                            } else if all_masters_connected {
                                STATUS_CONNECTED
                            } else {
                                STATUS_ENABLED
                            };

                            // Compare with previous status
                            // Always send config on new registration, otherwise only when status changed
                            let old_slave_status = member.status;
                            if !is_new_registration && new_slave_status == old_slave_status {
                                // Status unchanged and not a new registration, skip sending config
                                tracing::debug!(
                                    "Slave {} status unchanged ({}), skipping config send",
                                    slave_account,
                                    new_slave_status
                                );
                                continue;
                            }

                            // Status changed or new registration, send SlaveConfigMessage
                            if is_new_registration {
                                tracing::info!(
                                    "Slave {} new registration, sending initial config (status: {})",
                                    slave_account,
                                    new_slave_status
                                );
                            } else {
                                tracing::info!(
                                    "Slave {} status changed: {} -> {} (Master {} heartbeat)",
                                    slave_account,
                                    old_slave_status,
                                    new_slave_status,
                                    account_id
                                );
                            }

                            // Fetch Master's equity for margin_ratio mode
                            let master_equity = self
                                .connection_manager
                                .get_ea(&account_id)
                                .await
                                .map(|conn| conn.equity);

                            let config = SlaveConfigMessage {
                                account_id: slave_account.clone(),
                                master_account: account_id.clone(),
                                timestamp: chrono::Utc::now().to_rfc3339(),
                                trade_group_id: account_id.clone(),
                                status: new_slave_status,
                                lot_calculation_mode: member
                                    .slave_settings
                                    .lot_calculation_mode
                                    .clone()
                                    .into(),
                                lot_multiplier: member.slave_settings.lot_multiplier,
                                reverse_trade: member.slave_settings.reverse_trade,
                                symbol_mappings: member.slave_settings.symbol_mappings.clone(),
                                filters: member.slave_settings.filters.clone(),
                                config_version: member.slave_settings.config_version,
                                symbol_prefix: member.slave_settings.symbol_prefix.clone(),
                                symbol_suffix: member.slave_settings.symbol_suffix.clone(),
                                source_lot_min: member.slave_settings.source_lot_min,
                                source_lot_max: member.slave_settings.source_lot_max,
                                master_equity,
                                // Open Sync Policy settings
                                sync_mode: member.slave_settings.sync_mode.clone().into(),
                                limit_order_expiry_min: member
                                    .slave_settings
                                    .limit_order_expiry_min,
                                market_sync_max_pips: member.slave_settings.market_sync_max_pips,
                                max_slippage: member.slave_settings.max_slippage,
                                copy_pending_orders: member.slave_settings.copy_pending_orders,
                                // Trade Execution settings
                                max_retries: member.slave_settings.max_retries,
                                max_signal_delay_ms: member.slave_settings.max_signal_delay_ms,
                                use_pending_order_for_delayed: member
                                    .slave_settings
                                    .use_pending_order_for_delayed,
                                // Derived from status: allow new orders when enabled
                                allow_new_orders: new_slave_status > 0,
                            };

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
                                .update_member_status(&account_id, &slave_account, new_slave_status)
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
        }

        // Notify WebSocket clients of heartbeat
        let _ = self.broadcast_tx.send(format!(
            "ea_heartbeat:{}:{:.2}:{:.2}",
            account_id, balance, equity
        ));
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
