//! Configuration request handler
//!
//! Handles configuration requests from Master and Slave EAs, routing them to
//! appropriate handlers based on EA type.

use super::MessageHandler;
use crate::models::{RequestConfigMessage, SlaveConfigMessage};
use sankey_copier_zmq::MasterConfigMessage;

impl MessageHandler {
    /// Handle configuration request from Master or Slave EA
    pub(super) async fn handle_request_config(&self, msg: RequestConfigMessage) {
        let account_id = msg.account_id.clone();

        tracing::info!(
            "Config request received from: {} (ea_type: {})",
            account_id,
            msg.ea_type
        );

        // Route to appropriate handler based on EA type
        match msg.ea_type.as_str() {
            "Master" => self.handle_master_config_request(&account_id).await,
            "Slave" => self.handle_slave_config_request(&account_id).await,
            _ => {
                tracing::warn!(
                    "Config request rejected: account {} sent request with unknown ea_type '{}'",
                    account_id,
                    msg.ea_type
                );
            }
        }
    }

    /// Handle configuration request from Master EA
    async fn handle_master_config_request(&self, account_id: &str) {
        match self.db.get_settings_for_master(account_id).await {
            Ok(master_settings) => {
                let config = MasterConfigMessage {
                    account_id: account_id.to_string(),
                    symbol_prefix: master_settings.symbol_prefix,
                    symbol_suffix: master_settings.symbol_suffix,
                    config_version: master_settings.config_version,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                };

                // Send Master CONFIG via MessagePack
                if let Err(e) = self.config_sender.send(&config).await {
                    tracing::error!("Failed to send master config to {}: {}", account_id, e);
                } else {
                    tracing::info!(
                        "Successfully sent Master CONFIG to: {} (version: {})",
                        account_id,
                        config.config_version
                    );
                }
            }
            Err(e) => {
                tracing::error!("Failed to get master settings for {}: {}", account_id, e);
            }
        }
    }

    /// Handle configuration request from Slave EA
    async fn handle_slave_config_request(&self, account_id: &str) {
        match self.db.get_settings_for_slave(account_id).await {
            Ok(settings_list) => {
                if settings_list.is_empty() {
                    tracing::info!(
                        "No configuration found for {}. EA will wait for Web UI configuration.",
                        account_id
                    );
                    return;
                }

                for settings in settings_list {
                    tracing::info!(
                        "Found settings for {}: master={}, db_status={}, lot_mult={:?}",
                        account_id,
                        settings.master_account,
                        settings.status,
                        settings.slave_settings.lot_multiplier
                    );

                    // Calculate effective status based on Master's is_trade_allowed
                    let effective_status = if settings.status == 0 {
                        // User disabled -> DISABLED
                        0
                    } else {
                        // User enabled (status == 1)
                        // Check if Master is connected and has trading allowed
                        let master_conn = self
                            .connection_manager
                            .get_ea(&settings.master_account)
                            .await;

                        if let Some(conn) = master_conn {
                            if conn.is_trade_allowed {
                                // Master online && trading allowed -> CONNECTED
                                2
                            } else {
                                // Master online but trading NOT allowed -> ENABLED
                                1
                            }
                        } else {
                            // Master offline -> ENABLED
                            1
                        }
                    };

                    // Build SlaveConfigMessage with calculated effective status
                    let config = SlaveConfigMessage {
                        account_id: settings.slave_account.clone(),
                        master_account: settings.master_account.clone(),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                        status: effective_status,
                        lot_multiplier: settings.slave_settings.lot_multiplier,
                        reverse_trade: settings.slave_settings.reverse_trade,
                        symbol_mappings: settings.slave_settings.symbol_mappings.clone(),
                        filters: settings.slave_settings.filters.clone(),
                        config_version: settings.slave_settings.config_version,
                        symbol_prefix: settings.slave_settings.symbol_prefix.clone(),
                        symbol_suffix: settings.slave_settings.symbol_suffix.clone(),
                    };

                    // Send CONFIG via MessagePack
                    if let Err(e) = self.config_sender.send(&config).await {
                        tracing::error!("Failed to send config to {}: {}", account_id, e);
                    } else {
                        tracing::info!(
                            "Successfully sent CONFIG to: {} (db_status: {}, effective_status: {})",
                            account_id,
                            settings.status,
                            effective_status
                        );
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to query settings for {}: {}", account_id, e);
            }
        }
    }
}
