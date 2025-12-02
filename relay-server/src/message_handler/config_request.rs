//! Configuration request handler
//!
//! Handles configuration requests from Master and Slave EAs, routing them to
//! appropriate handlers based on EA type.

use super::MessageHandler;
use crate::models::{
    status_engine::{
        evaluate_master_status, evaluate_slave_status, ConnectionSnapshot, MasterClusterSnapshot,
        MasterIntent, SlaveIntent,
    },
    RequestConfigMessage, SlaveConfigMessage,
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
                // Get Master connection snapshot
                let master_conn = self.connection_manager.get_ea(account_id).await;
                let master_snapshot = ConnectionSnapshot {
                    connection_status: master_conn.as_ref().map(|c| c.status),
                    is_trade_allowed: master_conn
                        .as_ref()
                        .map(|c| c.is_trade_allowed)
                        .unwrap_or(true),
                };

                // Calculate status using centralized engine
                let status = evaluate_master_status(
                    MasterIntent {
                        web_ui_enabled: master_settings.enabled,
                    },
                    master_snapshot,
                )
                .status;

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

                    // Snapshot connections for Master / Slave evaluation
                    let master_conn = self
                        .connection_manager
                        .get_ea(&settings.master_account)
                        .await;
                    let master_snapshot = ConnectionSnapshot {
                        connection_status: master_conn.as_ref().map(|c| c.status),
                        is_trade_allowed: master_conn
                            .as_ref()
                            .map(|c| c.is_trade_allowed)
                            .unwrap_or(false),
                    };
                    let master_enabled =
                        match self.db.get_trade_group(&settings.master_account).await {
                            Ok(Some(tg)) => tg.master_settings.enabled,
                            Ok(None) => {
                                tracing::warn!(
                                    "Config request: missing trade group for master {}",
                                    settings.master_account
                                );
                                false
                            }
                            Err(err) => {
                                tracing::error!(
                                    "Config request: failed to load trade group {}: {}",
                                    settings.master_account,
                                    err
                                );
                                false
                            }
                        };
                    let master_result = evaluate_master_status(
                        MasterIntent {
                            web_ui_enabled: master_enabled,
                        },
                        master_snapshot,
                    );

                    let slave_conn = self.connection_manager.get_ea(account_id).await;
                    let slave_snapshot = ConnectionSnapshot {
                        connection_status: slave_conn.as_ref().map(|c| c.status),
                        is_trade_allowed: slave_conn
                            .as_ref()
                            .map(|c| c.is_trade_allowed)
                            .unwrap_or(false),
                    };
                    let slave_result = evaluate_slave_status(
                        SlaveIntent {
                            web_ui_enabled: settings.enabled_flag,
                        },
                        slave_snapshot,
                        MasterClusterSnapshot::new(vec![master_result.status]),
                    );

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
                        status: slave_result.status,
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
                        // Derived from status engine for consistent behavior
                        allow_new_orders: slave_result.allow_new_orders,
                    };

                    // Send CONFIG via MessagePack
                    if let Err(e) = self.publisher.send(&config).await {
                        tracing::error!("Failed to send config to {}: {}", account_id, e);
                    } else {
                        tracing::info!(
                            "Successfully sent CONFIG to: {} (db_status: {}, effective_status: {})",
                            account_id,
                            settings.status,
                            slave_result.status
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
