//! Configuration request handler
//!
//! Handles configuration requests from Master and Slave EAs, routing them to
//! appropriate handlers based on EA type.

use super::MessageHandler;
use crate::config_builder::{ConfigBuilder, MasterConfigContext, SlaveConfigContext};
use crate::models::{
    status_engine::{
        evaluate_master_status, ConnectionSnapshot, MasterClusterSnapshot, MasterIntent,
        SlaveIntent,
    },
    RequestConfigMessage,
};

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

                let bundle = ConfigBuilder::build_master_config(MasterConfigContext {
                    account_id: account_id.to_string(),
                    intent: MasterIntent {
                        web_ui_enabled: master_settings.enabled,
                    },
                    connection_snapshot: master_snapshot,
                    settings: &master_settings,
                    timestamp: chrono::Utc::now(),
                });
                let status = bundle.status_result.status;
                let config = bundle.config;

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
                    let slave_bundle = ConfigBuilder::build_slave_config(SlaveConfigContext {
                        slave_account: settings.slave_account.clone(),
                        master_account: settings.master_account.clone(),
                        trade_group_id: settings.master_account.clone(),
                        intent: SlaveIntent {
                            web_ui_enabled: settings.enabled_flag,
                        },
                        slave_connection_snapshot: ConnectionSnapshot {
                            connection_status: slave_conn.as_ref().map(|c| c.status),
                            is_trade_allowed: slave_conn
                                .as_ref()
                                .map(|c| c.is_trade_allowed)
                                .unwrap_or(false),
                        },
                        master_cluster: MasterClusterSnapshot::new(vec![master_result.status]),
                        slave_settings: &settings.slave_settings,
                        master_equity: self
                            .connection_manager
                            .get_ea(&settings.master_account)
                            .await
                            .map(|conn| conn.equity),
                        timestamp: chrono::Utc::now(),
                    });
                    let config = slave_bundle.config;
                    let new_status = slave_bundle.status_result.status;

                    // Send CONFIG via MessagePack
                    if let Err(e) = self.publisher.send(&config).await {
                        tracing::error!("Failed to send config to {}: {}", account_id, e);
                    } else {
                        tracing::info!(
                            "Successfully sent CONFIG to: {} (db_status: {}, effective_status: {})",
                            account_id,
                            settings.status,
                            new_status
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
