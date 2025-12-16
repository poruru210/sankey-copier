use crate::adapters::outbound::persistence::Database;
use crate::domain::models::SlaveSettings;
use sankey_copier_zmq::TradeFilters;

pub(crate) async fn create_test_db() -> Database {
    Database::new("sqlite::memory:").await.unwrap()
}

pub(crate) fn create_test_slave_settings() -> SlaveSettings {
    SlaveSettings {
        lot_calculation_mode: crate::domain::models::LotCalculationMode::default(),
        config_version: 1,
        symbol_prefix: None,
        symbol_suffix: None,
        lot_multiplier: Some(1.5),
        reverse_trade: false,
        symbol_mappings: vec![],
        filters: TradeFilters {
            allowed_symbols: None,
            blocked_symbols: None,
            allowed_magic_numbers: None,
            blocked_magic_numbers: None,
        },
        source_lot_min: None,
        source_lot_max: None,
        sync_mode: crate::domain::models::SyncMode::Skip,
        limit_order_expiry_min: None,
        market_sync_max_pips: None,
        max_slippage: None,
        copy_pending_orders: false,
        max_retries: 3,
        max_signal_delay_ms: 5000,
        use_pending_order_for_delayed: false,
    }
}
