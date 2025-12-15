use chrono::{DateTime, Utc};
use sankey_copier_zmq::{MasterConfigMessage, SlaveConfigMessage};

use crate::models::{
    status_engine::{
        evaluate_master_status, evaluate_member_status, ConnectionSnapshot, MasterIntent,
        MasterStatusResult, MemberStatusResult, SlaveIntent,
    },
    MasterSettings, SlaveSettings,
};

/// Context needed to build a MasterConfigMessage.
pub struct MasterConfigContext<'a> {
    pub account_id: String,
    pub intent: MasterIntent,
    pub connection_snapshot: ConnectionSnapshot,
    pub settings: &'a MasterSettings,
    pub timestamp: DateTime<Utc>,
}

/// Context needed to build a SlaveConfigMessage for a single Member (Master-Slave connection).
pub struct SlaveConfigContext<'a> {
    pub slave_account: String,
    pub master_account: String,
    pub trade_group_id: String,
    pub intent: SlaveIntent,
    pub slave_connection_snapshot: ConnectionSnapshot,
    /// The specific Master's status result (not the entire cluster)
    pub master_status_result: MasterStatusResult,
    pub slave_settings: &'a SlaveSettings,
    pub master_equity: Option<f64>,
    pub timestamp: DateTime<Utc>,
}

/// Bundle returned when building a Master config. Includes the calculated status for reuse.
pub struct MasterConfigBundle {
    pub config: MasterConfigMessage,
    pub status_result: MasterStatusResult,
}

/// Bundle returned when building a Slave config. Includes the calculated status/result.
#[derive(Default)]
pub struct SlaveConfigBundle {
    pub config: SlaveConfigMessage,
    pub status_result: MemberStatusResult,
}

/// Helper that centralizes Master/Slave config message creation.
pub struct ConfigBuilder;

impl ConfigBuilder {
    /// Build a MasterConfigMessage and return the evaluation result for callers that need it.
    pub fn build_master_config(context: MasterConfigContext) -> MasterConfigBundle {
        let status_result = evaluate_master_status(context.intent, context.connection_snapshot);
        let config = MasterConfigMessage {
            account_id: context.account_id,
            status: status_result.status,
            symbol_prefix: context.settings.symbol_prefix.clone(),
            symbol_suffix: context.settings.symbol_suffix.clone(),
            config_version: context.settings.config_version,
            timestamp: context.timestamp.timestamp_millis(),
            warning_codes: status_result.warning_codes.clone(),
        };

        MasterConfigBundle {
            config,
            status_result,
        }
    }

    /// Build a SlaveConfigMessage for a single Member (Master-Slave connection).
    /// Uses `evaluate_member_status` to evaluate based on the specific Master's state.
    pub fn build_slave_config(context: SlaveConfigContext) -> SlaveConfigBundle {
        let status_result = evaluate_member_status(
            context.intent,
            context.slave_connection_snapshot,
            &context.master_status_result,
        );

        let settings = context.slave_settings;
        let config = SlaveConfigMessage {
            account_id: context.slave_account,
            master_account: context.master_account,
            timestamp: context.timestamp.timestamp_millis(),
            trade_group_id: context.trade_group_id,
            status: status_result.status,
            lot_calculation_mode: settings.lot_calculation_mode.clone().into(),
            lot_multiplier: settings.lot_multiplier,
            reverse_trade: settings.reverse_trade,
            symbol_mappings: settings.symbol_mappings.clone(),
            filters: settings.filters.clone(),
            config_version: settings.config_version,
            symbol_prefix: settings.symbol_prefix.clone(),
            symbol_suffix: settings.symbol_suffix.clone(),
            source_lot_min: settings.source_lot_min,
            source_lot_max: settings.source_lot_max,
            master_equity: context.master_equity,
            // Open Sync Policy settings
            sync_mode: settings.sync_mode.clone().into(),
            limit_order_expiry_min: settings.limit_order_expiry_min,
            market_sync_max_pips: settings.market_sync_max_pips,
            max_slippage: settings.max_slippage,
            copy_pending_orders: settings.copy_pending_orders,
            // Trade Execution settings
            max_retries: settings.max_retries,
            max_signal_delay_ms: settings.max_signal_delay_ms,
            use_pending_order_for_delayed: settings.use_pending_order_for_delayed,
            allow_new_orders: status_result.allow_new_orders,
            warning_codes: status_result.warning_codes.clone(),
        };

        SlaveConfigBundle {
            config,
            status_result,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        status_engine::{ConnectionSnapshot, MasterIntent, SlaveIntent},
        ConnectionStatus, MasterSettings, SlaveSettings, WarningCode, STATUS_CONNECTED,
        STATUS_DISABLED, STATUS_ENABLED,
    };

    fn online_snapshot() -> ConnectionSnapshot {
        ConnectionSnapshot {
            connection_status: Some(ConnectionStatus::Online),
            is_trade_allowed: true,
        }
    }

    fn connected_master() -> MasterStatusResult {
        MasterStatusResult {
            status: STATUS_CONNECTED,
            warning_codes: vec![],
        }
    }

    fn offline_master() -> MasterStatusResult {
        MasterStatusResult {
            status: STATUS_DISABLED,
            warning_codes: vec![WarningCode::MasterOffline],
        }
    }

    #[test]
    fn master_builder_clones_settings_and_returns_connected() {
        let settings = MasterSettings {
            enabled: true,
            symbol_prefix: Some("pre.".into()),
            symbol_suffix: Some(".suf".into()),
            config_version: 7,
        };

        let context = MasterConfigContext {
            account_id: "MASTER_001".into(),
            intent: MasterIntent {
                web_ui_enabled: true,
            },
            connection_snapshot: online_snapshot(),
            settings: &settings,
            timestamp: chrono::Utc::now(),
        };

        let bundle = ConfigBuilder::build_master_config(context);
        assert_eq!(bundle.status_result.status, STATUS_CONNECTED);
        assert!(bundle.config.warning_codes.is_empty());
        assert_eq!(bundle.config.symbol_prefix, Some("pre.".to_string()));
        assert_eq!(bundle.config.symbol_suffix, Some(".suf".to_string()));
    }

    #[test]
    fn master_builder_includes_warning_codes_when_disabled() {
        let settings = MasterSettings {
            enabled: false,
            symbol_prefix: None,
            symbol_suffix: None,
            config_version: 4,
        };

        let context = MasterConfigContext {
            account_id: "MASTER_WARN".into(),
            intent: MasterIntent {
                web_ui_enabled: settings.enabled,
            },
            connection_snapshot: online_snapshot(),
            settings: &settings,
            timestamp: chrono::Utc::now(),
        };

        let bundle = ConfigBuilder::build_master_config(context);
        assert_eq!(bundle.status_result.status, STATUS_DISABLED);
        assert!(bundle
            .config
            .warning_codes
            .contains(&WarningCode::MasterWebUiDisabled));
    }

    #[test]
    fn slave_builder_connected_when_master_connected() {
        // Master is connected, Slave is online with Web UI ON
        let context = SlaveConfigContext {
            slave_account: "SLAVE_001".into(),
            master_account: "MASTER_001".into(),
            trade_group_id: "MASTER_001".into(),
            intent: SlaveIntent {
                web_ui_enabled: true,
            },
            slave_connection_snapshot: online_snapshot(),
            master_status_result: connected_master(),
            slave_settings: &SlaveSettings::default(),
            master_equity: Some(1000.0),
            timestamp: chrono::Utc::now(),
        };

        let bundle = ConfigBuilder::build_slave_config(context);
        assert_eq!(bundle.status_result.status, STATUS_CONNECTED);
        assert!(bundle.config.allow_new_orders);
        assert!(bundle.config.warning_codes.is_empty());
    }

    #[test]
    fn slave_builder_enabled_when_master_offline() {
        // Master is offline, Slave is online with Web UI ON
        // Member status should be ENABLED (waiting for Master)
        // But allow_new_orders should still be true (Slave can process signals if they arrive)
        let context = SlaveConfigContext {
            slave_account: "SLAVE_002".into(),
            master_account: "MASTER_001".into(),
            trade_group_id: "MASTER_001".into(),
            intent: SlaveIntent {
                web_ui_enabled: true,
            },
            slave_connection_snapshot: online_snapshot(),
            master_status_result: offline_master(),
            slave_settings: &SlaveSettings::default(),
            master_equity: Some(500.0),
            timestamp: chrono::Utc::now(),
        };

        let bundle = ConfigBuilder::build_slave_config(context);
        assert_eq!(bundle.status_result.status, STATUS_ENABLED);
        // allow_new_orders is based on Slave's own state, not Master
        assert!(bundle.config.allow_new_orders);
        // Master's warning code should be propagated
        assert!(bundle
            .config
            .warning_codes
            .contains(&WarningCode::MasterOffline));
    }

    #[test]
    fn slave_builder_disabled_when_slave_offline() {
        // Master is connected, but Slave is offline
        let offline_slave = ConnectionSnapshot {
            connection_status: Some(ConnectionStatus::Offline),
            is_trade_allowed: true,
        };

        let context = SlaveConfigContext {
            slave_account: "SLAVE_003".into(),
            master_account: "MASTER_001".into(),
            trade_group_id: "MASTER_001".into(),
            intent: SlaveIntent {
                web_ui_enabled: true,
            },
            slave_connection_snapshot: offline_slave,
            master_status_result: connected_master(),
            slave_settings: &SlaveSettings::default(),
            master_equity: Some(500.0),
            timestamp: chrono::Utc::now(),
        };

        let bundle = ConfigBuilder::build_slave_config(context);
        assert_eq!(bundle.status_result.status, STATUS_DISABLED);
        assert!(!bundle.config.allow_new_orders);
        assert!(bundle
            .config
            .warning_codes
            .contains(&WarningCode::SlaveOffline));
    }
}
