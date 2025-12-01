//! Unregister message handler
//!
//! Handles EA unregistration messages, updating connection status and notifying clients.
//! When a Master EA disconnects, notifies all Slaves so they can update their status.

use super::MessageHandler;
use crate::models::{
    status::{calculate_slave_status, SlaveStatusInput},
    EaType, SlaveConfigMessage, SlaveConfigWithMaster, UnregisterMessage, STATUS_DISABLED,
};

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

            // Send config update to all Slaves under this Master
            match self.db.get_members(account_id).await {
                Ok(members) => {
                    for member in members {
                        // Get Slave connection info
                        let slave_conn =
                            self.connection_manager.get_ea(&member.slave_account).await;
                        let slave_is_trade_allowed = slave_conn
                            .as_ref()
                            .map(|c| c.is_trade_allowed)
                            .unwrap_or(true);

                        // Calculate Slave status - Master is now DISABLED (disconnected)
                        let slave_enabled = member.status > 0;
                        let slave_status = calculate_slave_status(&SlaveStatusInput {
                            web_ui_enabled: slave_enabled,
                            connection_status: slave_conn.as_ref().map(|c| c.status),
                            is_trade_allowed: slave_is_trade_allowed,
                            master_status: STATUS_DISABLED, // Master disconnected
                        });

                        let config = SlaveConfigMessage {
                            account_id: member.slave_account.clone(),
                            master_account: account_id.clone(),
                            timestamp: chrono::Utc::now().to_rfc3339(),
                            trade_group_id: account_id.clone(),
                            status: slave_status,
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
                            master_equity: None, // Master disconnected, no equity
                            sync_mode: member.slave_settings.sync_mode.clone().into(),
                            limit_order_expiry_min: member.slave_settings.limit_order_expiry_min,
                            market_sync_max_pips: member.slave_settings.market_sync_max_pips,
                            max_slippage: member.slave_settings.max_slippage,
                            copy_pending_orders: member.slave_settings.copy_pending_orders,
                            max_retries: member.slave_settings.max_retries,
                            max_signal_delay_ms: member.slave_settings.max_signal_delay_ms,
                            use_pending_order_for_delayed: member
                                .slave_settings
                                .use_pending_order_for_delayed,
                            allow_new_orders: slave_status > 0,
                        };

                        if let Err(e) = self.publisher.send(&config).await {
                            tracing::error!(
                                "Failed to send config to {} on Master disconnect: {}",
                                member.slave_account,
                                e
                            );
                        } else {
                            tracing::info!(
                                "Notified {} (status: {}) of Master {} disconnect",
                                member.slave_account,
                                slave_status,
                                account_id
                            );
                        }

                        // Notify WebSocket clients
                        let settings_with_master = SlaveConfigWithMaster {
                            master_account: account_id.clone(),
                            slave_account: member.slave_account.clone(),
                            status: slave_status,
                            slave_settings: member.slave_settings.clone(),
                        };
                        if let Ok(json) = serde_json::to_string(&settings_with_master) {
                            let _ = self.broadcast_tx.send(format!("settings_updated:{}", json));
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to get members for Master {} disconnect: {}",
                        account_id,
                        e
                    );
                }
            }
        }
    }
}
