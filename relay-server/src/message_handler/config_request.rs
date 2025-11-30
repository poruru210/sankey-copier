//! Configuration request handler
//!
//! Handles configuration requests from Master and Slave EAs, routing them to
//! appropriate handlers based on EA type.

use super::MessageHandler;
use crate::models::{
    status::{
        calculate_master_status, calculate_slave_status, MasterStatusInput, SlaveStatusInput,
    },
    RequestConfigMessage, SlaveConfigMessage, STATUS_CONNECTED, STATUS_DISABLED,
};
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
                // Get is_trade_allowed from connection manager
                let is_trade_allowed = self
                    .connection_manager
                    .get_ea(account_id)
                    .await
                    .map(|conn| conn.is_trade_allowed)
                    .unwrap_or(true); // Default to true if not connected yet

                // Calculate status using centralized logic
                let status = calculate_master_status(&MasterStatusInput {
                    web_ui_enabled: master_settings.enabled,
                    is_trade_allowed,
                });

                let config = MasterConfigMessage {
                    account_id: account_id.to_string(),
                    status,
                    symbol_prefix: master_settings.symbol_prefix,
                    symbol_suffix: master_settings.symbol_suffix,
                    config_version: master_settings.config_version,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                };

                // Send Master CONFIG via MessagePack
                if let Err(e) = self.publisher.send(&config).await {
                    tracing::error!("Failed to send master config to {}: {}", account_id, e);
                } else {
                    tracing::info!(
                        "Successfully sent Master CONFIG to: {} (status: {}, version: {})",
                        account_id,
                        status,
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

                    // Get Slave's is_trade_allowed from connection manager
                    let slave_is_trade_allowed = self
                        .connection_manager
                        .get_ea(account_id)
                        .await
                        .map(|conn| conn.is_trade_allowed)
                        .unwrap_or(true);

                    // Get Master's status (CONNECTED if online + trade allowed)
                    let master_conn = self
                        .connection_manager
                        .get_ea(&settings.master_account)
                        .await;
                    let master_status = if let Some(conn) = master_conn {
                        if conn.is_trade_allowed {
                            STATUS_CONNECTED
                        } else {
                            STATUS_DISABLED
                        }
                    } else {
                        STATUS_DISABLED // Master offline
                    };

                    // Calculate Slave status using centralized logic
                    // Web UI enabled = DB status > 0
                    let effective_status = calculate_slave_status(&SlaveStatusInput {
                        web_ui_enabled: settings.status > 0,
                        is_trade_allowed: slave_is_trade_allowed,
                        master_status,
                    });

                    // Fetch Master's equity for margin_ratio mode
                    let master_equity = self
                        .connection_manager
                        .get_ea(&settings.master_account)
                        .await
                        .map(|conn| conn.equity);

                    // Build SlaveConfigMessage with calculated effective status
                    let config = SlaveConfigMessage {
                        account_id: settings.slave_account.clone(),
                        master_account: settings.master_account.clone(),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                        trade_group_id: settings.master_account.clone(),
                        status: effective_status,
                        lot_calculation_mode: settings
                            .slave_settings
                            .lot_calculation_mode
                            .clone()
                            .into(),
                        lot_multiplier: settings.slave_settings.lot_multiplier,
                        reverse_trade: settings.slave_settings.reverse_trade,
                        symbol_mappings: settings.slave_settings.symbol_mappings.clone(),
                        filters: settings.slave_settings.filters.clone(),
                        config_version: settings.slave_settings.config_version,
                        symbol_prefix: settings.slave_settings.symbol_prefix.clone(),
                        symbol_suffix: settings.slave_settings.symbol_suffix.clone(),
                        source_lot_min: settings.slave_settings.source_lot_min,
                        source_lot_max: settings.slave_settings.source_lot_max,
                        master_equity,
                        // Open Sync Policy settings
                        sync_mode: settings.slave_settings.sync_mode.clone().into(),
                        limit_order_expiry_min: settings.slave_settings.limit_order_expiry_min,
                        market_sync_max_pips: settings.slave_settings.market_sync_max_pips,
                        max_slippage: settings.slave_settings.max_slippage,
                        copy_pending_orders: settings.slave_settings.copy_pending_orders,
                        // Trade Execution settings
                        max_retries: settings.slave_settings.max_retries,
                        max_signal_delay_ms: settings.slave_settings.max_signal_delay_ms,
                        use_pending_order_for_delayed: settings
                            .slave_settings
                            .use_pending_order_for_delayed,
                        // Derived from status: allow new orders when enabled
                        allow_new_orders: effective_status > 0,
                    };

                    // Send CONFIG via MessagePack
                    if let Err(e) = self.publisher.send(&config).await {
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
