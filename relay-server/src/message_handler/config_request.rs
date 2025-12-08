//! Configuration request handler
//!
//! Handles configuration requests from Master and Slave EAs, routing them to
//! appropriate handlers based on EA type.

use super::MessageHandler;
use crate::config_builder::{ConfigBuilder, MasterConfigContext};
use crate::models::{
    status_engine::{ConnectionSnapshot, MasterIntent},
    RequestConfigMessage,
};
use crate::runtime_status_updater::SlaveRuntimeTarget;

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
                let master_conn = self.connection_manager.get_master(account_id).await;
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

                let runtime_updater = self.runtime_status_updater();

                for settings in settings_list {
                    let master_account = settings.master_account.clone();
                    let slave_account = settings.slave_account.clone();

                    tracing::info!(
                        "Found settings for {}: master={}, db_status={}, lot_mult={:?}",
                        account_id,
                        master_account,
                        settings.status,
                        settings.slave_settings.lot_multiplier
                    );

                    let slave_bundle = runtime_updater
                        .build_slave_bundle(SlaveRuntimeTarget {
                            master_account: master_account.as_str(),
                            trade_group_id: master_account.as_str(),
                            slave_account: account_id,
                            enabled_flag: settings.enabled_flag,
                            slave_settings: &settings.slave_settings,
                        })
                        .await;
                    let config = slave_bundle.config;
                    let new_status = slave_bundle.status_result.status;

                    if let Err(err) = self
                        .db
                        .update_member_runtime_status(&master_account, &slave_account, new_status)
                        .await
                    {
                        tracing::error!(
                            "Failed to persist status for slave {} in trade group {}: {}",
                            slave_account,
                            master_account,
                            err
                        );
                    } else {
                        tracing::debug!(
                            "Updated status via RequestConfig: master={}, slave={}, status={}",
                            master_account,
                            slave_account,
                            new_status
                        );
                    }

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
