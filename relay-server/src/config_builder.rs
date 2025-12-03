use chrono::{DateTime, Utc};
use sankey_copier_zmq::{MasterConfigMessage, SlaveConfigMessage};

use crate::models::{
    status_engine::{
        evaluate_master_status, evaluate_slave_status, ConnectionSnapshot, MasterClusterSnapshot,
        MasterIntent, MasterStatusResult, SlaveIntent, SlaveStatusResult,
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

/// Context needed to build a SlaveConfigMessage.
pub struct SlaveConfigContext<'a> {
    pub slave_account: String,
    pub master_account: String,
    pub trade_group_id: String,
    pub intent: SlaveIntent,
    pub slave_connection_snapshot: ConnectionSnapshot,
    pub master_cluster: MasterClusterSnapshot,
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
pub struct SlaveConfigBundle {
    pub config: SlaveConfigMessage,
    pub status_result: SlaveStatusResult,
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
            timestamp: context.timestamp.to_rfc3339(),
            warning_codes: status_result.warning_codes.clone(),
        };

        MasterConfigBundle {
            config,
            status_result,
        }
    }

    /// Build a SlaveConfigMessage and return the evaluation result for reuse.
    pub fn build_slave_config(context: SlaveConfigContext) -> SlaveConfigBundle {
        let status_result = evaluate_slave_status(
            context.intent,
            context.slave_connection_snapshot,
            context.master_cluster,
        );

        let settings = context.slave_settings;
        let config = SlaveConfigMessage {
            account_id: context.slave_account,
            master_account: context.master_account,
            timestamp: context.timestamp.to_rfc3339(),
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
        status_engine::{ConnectionSnapshot, MasterClusterSnapshot, MasterIntent, SlaveIntent},
        ConnectionStatus, MasterSettings, SlaveSettings, WarningCode, STATUS_CONNECTED,
        STATUS_DISABLED, STATUS_ENABLED,
    };

    fn online_snapshot() -> ConnectionSnapshot {
        ConnectionSnapshot {
            connection_status: Some(ConnectionStatus::Online),
            is_trade_allowed: true,
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
    fn slave_builder_enforces_allow_new_orders_logic() {
        let master_bundle = MasterClusterSnapshot::new(vec![STATUS_CONNECTED]);
        let context = SlaveConfigContext {
            slave_account: "SLAVE_001".into(),
            master_account: "MASTER_001".into(),
            trade_group_id: "MASTER_001".into(),
            intent: SlaveIntent {
                web_ui_enabled: true,
            },
            slave_connection_snapshot: online_snapshot(),
            master_cluster: master_bundle,
            slave_settings: &SlaveSettings::default(),
            master_equity: Some(1000.0),
            timestamp: chrono::Utc::now(),
        };

        let bundle = ConfigBuilder::build_slave_config(context);
        assert_eq!(bundle.status_result.status, STATUS_CONNECTED);
        assert!(bundle.config.allow_new_orders);
        assert!(bundle.config.warning_codes.is_empty());

        let disabled_cluster = MasterClusterSnapshot::new(vec![STATUS_ENABLED]);
        let context = SlaveConfigContext {
            slave_account: "SLAVE_002".into(),
            master_account: "MASTER_001".into(),
            trade_group_id: "MASTER_001".into(),
            intent: SlaveIntent {
                web_ui_enabled: true,
            },
            slave_connection_snapshot: online_snapshot(),
            master_cluster: disabled_cluster,
            slave_settings: &SlaveSettings::default(),
            master_equity: Some(500.0),
            timestamp: chrono::Utc::now(),
        };

        let bundle = ConfigBuilder::build_slave_config(context);
        assert_eq!(bundle.status_result.status, STATUS_ENABLED);
        assert!(!bundle.config.allow_new_orders);
        assert!(bundle
            .config
            .warning_codes
            .contains(&WarningCode::MasterClusterDegraded));
    }
}
