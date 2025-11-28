use crate::msgpack::{
    HeartbeatMessage, LotCalculationMode, SlaveConfigMessage, SyncMode, TradeFilters,
};

/// Test that new symbol filter fields are correctly serialized and deserialized
#[test]
fn test_symbol_filter_fields_roundtrip() {
    // Test HeartbeatMessage with symbol filter fields
    let heartbeat = HeartbeatMessage {
        message_type: "Heartbeat".to_string(),
        account_id: "TEST_001".to_string(),
        balance: 10000.0,
        equity: 10000.0,
        open_positions: 0,
        timestamp: "2025-01-01T00:00:00Z".to_string(),
        version: "1.0.0".to_string(),
        ea_type: "Slave".to_string(),
        platform: "MT5".to_string(),
        account_number: 12345,
        broker: "TestBroker".to_string(),
        account_name: "TestAccount".to_string(),
        server: "TestServer".to_string(),
        currency: "USD".to_string(),
        leverage: 100,
        is_trade_allowed: true,
        symbol_prefix: Some("pro.".to_string()),
        symbol_suffix: Some(".m".to_string()),
        symbol_map: Some("XAUUSD=GOLD,EURUSD=EUR".to_string()),
    };

    // Serialize
    let serialized = rmp_serde::to_vec_named(&heartbeat).expect("Failed to serialize");

    // Deserialize
    let deserialized: HeartbeatMessage =
        rmp_serde::from_slice(&serialized).expect("Failed to deserialize");

    // Verify symbol filter fields
    assert_eq!(
        heartbeat.symbol_prefix, deserialized.symbol_prefix,
        "symbol_prefix should match"
    );
    assert_eq!(
        heartbeat.symbol_suffix, deserialized.symbol_suffix,
        "symbol_suffix should match"
    );
    assert_eq!(
        heartbeat.symbol_map, deserialized.symbol_map,
        "symbol_map should match"
    );
}

/// Test that SlaveConfigMessage with None symbol filters serializes correctly
#[test]
fn test_config_message_none_symbol_filters() {
    let config = SlaveConfigMessage {
        account_id: "TEST_001".to_string(),
        master_account: "MASTER_001".to_string(),
        timestamp: "2025-01-01T00:00:00Z".to_string(),
        trade_group_id: "MASTER_001".to_string(),
        status: 2,
        lot_calculation_mode: LotCalculationMode::default(),
        lot_multiplier: Some(1.5),
        reverse_trade: false,
        symbol_mappings: vec![],
        filters: TradeFilters {
            allowed_symbols: None,
            blocked_symbols: None,
            allowed_magic_numbers: None,
            blocked_magic_numbers: None,
        },
        config_version: 1,
        symbol_prefix: None,
        symbol_suffix: None,
        source_lot_min: None,
        source_lot_max: None,
        master_equity: None,
        // Open Sync Policy defaults
        sync_mode: SyncMode::default(),
        limit_order_expiry_min: None,
        market_sync_max_pips: None,
        max_slippage: None,
        copy_pending_orders: false,
        // Trade Execution defaults
        max_retries: 3,
        max_signal_delay_ms: 5000,
        use_pending_order_for_delayed: false,
        allow_new_orders: true,
    };

    let serialized = rmp_serde::to_vec_named(&config).expect("Failed to serialize");
    let deserialized: SlaveConfigMessage =
        rmp_serde::from_slice(&serialized).expect("Failed to deserialize");

    assert_eq!(config.symbol_prefix, deserialized.symbol_prefix);
    assert_eq!(config.symbol_suffix, deserialized.symbol_suffix);
}

/// Test that SlaveConfigMessage with Some symbol filters serializes correctly
#[test]
fn test_config_message_some_symbol_filters() {
    let config = SlaveConfigMessage {
        account_id: "TEST_001".to_string(),
        master_account: "MASTER_001".to_string(),
        timestamp: "2025-01-01T00:00:00Z".to_string(),
        trade_group_id: "MASTER_001".to_string(),
        status: 2,
        lot_calculation_mode: LotCalculationMode::MarginRatio,
        lot_multiplier: Some(1.5),
        reverse_trade: false,
        symbol_mappings: vec![],
        filters: TradeFilters {
            allowed_symbols: None,
            blocked_symbols: None,
            allowed_magic_numbers: None,
            blocked_magic_numbers: None,
        },
        config_version: 1,
        symbol_prefix: Some("FX.".to_string()),
        symbol_suffix: Some("-ECN".to_string()),
        source_lot_min: Some(0.01),
        source_lot_max: Some(10.0),
        master_equity: Some(50000.0),
        // Open Sync Policy defaults
        sync_mode: SyncMode::MarketOrder,
        limit_order_expiry_min: Some(60),
        market_sync_max_pips: Some(50.0),
        max_slippage: Some(30),
        copy_pending_orders: true,
        // Trade Execution settings
        max_retries: 5,
        max_signal_delay_ms: 10000,
        use_pending_order_for_delayed: true,
        allow_new_orders: false,
    };

    let serialized = rmp_serde::to_vec_named(&config).expect("Failed to serialize");
    let deserialized: SlaveConfigMessage =
        rmp_serde::from_slice(&serialized).expect("Failed to deserialize");

    assert_eq!(
        config.symbol_prefix, deserialized.symbol_prefix,
        "symbol_prefix should match"
    );
    assert_eq!(
        config.symbol_suffix, deserialized.symbol_suffix,
        "symbol_suffix should match"
    );
}
