// relay-server/src/message_handler/register.rs
//
// Handler for Register messages from EAs.
// Registers an EA explicitly with the connection manager.

use crate::config_builder::{ConfigBuilder, MasterConfigContext};
use crate::domain::models::{RegisterMessage, VLogsGlobalSettings};
use crate::domain::services::status_calculator::SlaveRuntimeTarget;
use crate::domain::services::status_calculator::{ConnectionSnapshot, MasterIntent};

use super::MessageHandler;

impl MessageHandler {
    /// Handle an explicit Register message from an EA
    ///
    /// This is the new protocol: EAs send a Register message on OnTimer's first iteration
    /// to explicitly register themselves with the relay server.
    ///
    /// Register handles:
    /// 1. ConnectionManager registration (is_trade_allowed=false)
    /// 2. VLogs config broadcast
    /// 3. Initial Config send (with is_trade_allowed=false assumption)
    ///
    /// The first Heartbeat will then update is_trade_allowed to the actual value
    /// and trigger a proper StatusEngine evaluation with accurate status.
    pub async fn handle_register(&self, msg: RegisterMessage) {
        let account_id = &msg.account_id;
        let ea_type = &msg.ea_type;

        tracing::info!(
            account = %account_id,
            ea_type = %ea_type,
            platform = %msg.platform,
            account_number = %msg.account_number,
            broker = %msg.broker,
            "[REGISTER] EA registration received"
        );

        if let Some(symbols) = &msg.detected_symbols {
            tracing::info!(
                account = %account_id,
                symbols = ?symbols,
                "[REGISTER] with detected_symbols"
            );
        }

        // 1. Register the EA with ConnectionManager (is_trade_allowed=false)
        self.connection_manager.register_ea(&msg).await;

        // 2. Send VictoriaLogs config
        self.send_vlogs_config_on_register(account_id).await;

        // 3. Send initial Config based on ea_type
        if ea_type == "Master" {
            self.send_initial_master_config(account_id).await;
        } else if ea_type == "Slave" {
            self.send_initial_slave_config(account_id).await;
        }

        tracing::info!(
            account = %account_id,
            ea_type = %ea_type,
            "[REGISTER] EA registered, VLogs and initial Config sent"
        );
    }

    /// Send VictoriaLogs configuration to a newly registered EA
    async fn send_vlogs_config_on_register(&self, account_id: &str) {
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

    /// Send initial Master config (is_trade_allowed=false assumption)
    async fn send_initial_master_config(&self, account_id: &str) {
        // Get Master connection info
        let master_conn = self.connection_manager.get_master(account_id).await;
        tracing::info!(
            master = %account_id,
            found = master_conn.is_some(),
            "[REGISTER] get_master call result"
        );

        // Get TradeGroup for master settings
        let trade_group = match self.db.get_trade_group(account_id).await {
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

        // Build Master config with is_trade_allowed=false (initial assumption)
        let context = MasterConfigContext {
            account_id: account_id.to_string(),
            intent: MasterIntent {
                web_ui_enabled: trade_group.master_settings.enabled,
            },
            connection_snapshot: ConnectionSnapshot {
                connection_status: Some(crate::domain::models::ConnectionStatus::Online),
                is_trade_allowed: false, // Initial assumption until first Heartbeat
            },
            settings: &trade_group.master_settings,
            timestamp: chrono::Utc::now(),
        };

        let master_bundle = ConfigBuilder::build_master_config(context);

        // Send config via ZMQ
        if let Err(e) = self.publisher.send(&master_bundle.config).await {
            tracing::error!(
                account = %account_id,
                error = %e,
                "[REGISTER] Failed to send initial Master config"
            );
        } else {
            tracing::info!(
                account = %account_id,
                status = %master_bundle.status_result.status,
                "[REGISTER] Initial Master config sent (is_trade_allowed=false assumed)"
            );
        }

        // Notify Slaves that Master has registered/connected
        match self.db.get_members(account_id).await {
            Ok(members) => {
                tracing::info!(
                    master = %account_id,
                    count = members.len(),
                    "[REGISTER] Found members for Master registration check"
                );
                for member in members {
                    tracing::info!(
                        master = %account_id,
                        slave = %member.slave_account,
                        "[REGISTER] Triggering Slave update after Master registration"
                    );
                    self.send_slave_config_for_master(&member.slave_account, account_id)
                        .await;
                }
            }
            Err(e) => {
                tracing::error!("Failed to get members for Master {}: {}", account_id, e);
            }
        }
    }

    /// Send initial Slave config (is_trade_allowed=false assumption)
    async fn send_initial_slave_config(&self, account_id: &str) {
        // Get all masters this slave is connected to
        let master_ids = match self.db.get_masters_for_slave(account_id).await {
            Ok(ids) => ids,
            Err(e) => {
                tracing::error!("Failed to get masters for Slave {}: {}", account_id, e);
                return;
            }
        };

        if master_ids.is_empty() {
            tracing::debug!(
                "Slave {} is not connected to any Master (no config to send)",
                account_id
            );
            return;
        }

        for master_id in master_ids {
            self.send_slave_config_for_master(account_id, &master_id)
                .await;
        }
    }

    /// Send Slave config for a specific Master
    /// Send Slave config for a specific Master
    async fn send_slave_config_for_master(&self, slave_account: &str, master_account: &str) {
        // Get member info
        let member = match self.db.get_member(master_account, slave_account).await {
            Ok(Some(m)) => m,
            Ok(None) => return,
            Err(e) => {
                tracing::error!(
                    "Failed to get member for Slave {} Master {}: {}",
                    slave_account,
                    master_account,
                    e
                );
                return;
            }
        };

        // Debug: Check if Master is really registered in ConnectionManager
        let master_debug = self.connection_manager.get_master(master_account).await;
        tracing::info!(
            master = %master_account,
            found = master_debug.is_some(),
            "[REGISTER] Debug: Check master connection presence"
        );

        // Use RuntimeStatusUpdater to build consistent config bundle
        let runtime_updater = self.runtime_status_updater();

        let slave_bundle = runtime_updater
            .build_slave_bundle(SlaveRuntimeTarget {
                master_account,
                trade_group_id: master_account,
                slave_account,
                enabled_flag: member.enabled_flag,
                slave_settings: &member.slave_settings,
            })
            .await;

        // 1. Send Config via ZMQ
        if let Err(e) = self.publisher.send(&slave_bundle.config).await {
            tracing::error!(
                account = %slave_account,
                master = %master_account,
                error = %e,
                "[REGISTER] Failed to send initial Slave config"
            );
        } else {
            tracing::info!(
                account = %slave_account,
                master = %master_account,
                status = %slave_bundle.status_result.status,
                "[REGISTER] Initial Slave config sent"
            );
        }
    }
}
