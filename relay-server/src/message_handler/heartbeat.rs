//! Heartbeat message handler
//!
//! Handles heartbeat messages for health monitoring, auto-registration, and
//! Master EA is_trade_allowed change notifications to Slave EAs.

use super::MessageHandler;
use crate::models::{HeartbeatMessage, SlaveConfigMessage, SlaveConfigWithMaster};

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

        // Update heartbeat (performs auto-registration if needed)
        self.connection_manager.update_heartbeat(msg).await;

        // If this is a Master EA, check for is_trade_allowed changes
        if ea_type == "Master" {
            // Detect is_trade_allowed change
            let trade_allowed_changed = old_is_trade_allowed != Some(new_is_trade_allowed);

            if trade_allowed_changed {
                tracing::info!(
                    "Master {} is_trade_allowed changed: {:?} -> {}",
                    account_id,
                    old_is_trade_allowed,
                    new_is_trade_allowed
                );

                // Resend Config to all Slave accounts connected to this Master
                match self.db.get_members(&account_id).await {
                    Ok(members) => {
                        for member in members {
                            // Only send to enabled Slaves (status > 0)
                            if member.status > 0 {
                                // Build SlaveConfig with calculated effective status
                                let effective_status = if new_is_trade_allowed { 2 } else { 1 };

                                // Fetch Master's equity for margin_ratio mode from heartbeat
                                let master_equity = self
                                    .connection_manager
                                    .get_ea(&account_id)
                                    .await
                                    .map(|conn| conn.equity);

                                let config = SlaveConfigMessage {
                                    account_id: member.slave_account.clone(),
                                    master_account: account_id.clone(),
                                    timestamp: chrono::Utc::now().to_rfc3339(),
                                    status: effective_status,
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
                                };

                                if let Err(e) = self.config_sender.send(&config).await {
                                    tracing::error!(
                                        "Failed to send config to {} due to Master is_trade_allowed change: {}",
                                        member.slave_account,
                                        e
                                    );
                                } else {
                                    tracing::info!(
                                        "Sent config to {} (effective_status: {}) due to Master {} is_trade_allowed change: {}",
                                        member.slave_account,
                                        effective_status,
                                        account_id,
                                        new_is_trade_allowed
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

            // Update all enabled settings to CONNECTED (status=2)
            match self.db.update_master_statuses_connected(&account_id).await {
                Ok(count) if count > 0 => {
                    tracing::info!(
                        "Master {} connected: updated {} settings to CONNECTED",
                        account_id,
                        count
                    );
                    // Notify WebSocket clients
                    // We need to broadcast the updated settings for all affected slaves
                    if let Ok(members) = self.db.get_members(&account_id).await {
                        for member in members {
                            let settings_with_master = SlaveConfigWithMaster {
                                master_account: account_id.clone(),
                                slave_account: member.slave_account.clone(),
                                status: member.status,
                                slave_settings: member.slave_settings.clone(),
                            };
                            if let Ok(json) = serde_json::to_string(&settings_with_master) {
                                let _ =
                                    self.broadcast_tx.send(format!("settings_updated:{}", json));
                            }
                        }
                    }
                }
                Ok(_) => {
                    // No settings updated (no enabled settings for this master)
                }
                Err(e) => {
                    tracing::error!("Failed to update master statuses for {}: {}", account_id, e);
                }
            }
        }

        // Notify WebSocket clients of heartbeat
        let _ = self.broadcast_tx.send(format!(
            "ea_heartbeat:{}:{:.2}:{:.2}",
            account_id, balance, equity
        ));
    }
}
