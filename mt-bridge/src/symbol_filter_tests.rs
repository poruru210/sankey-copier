use crate::msgpack::{ConfigMessage, HeartbeatMessage, TradeFilters};

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

/// Test that ConfigMessage with None symbol filters serializes correctly
#[test]
fn test_config_message_none_symbol_filters() {
    let config = ConfigMessage {
        account_id: "TEST_001".to_string(),
        master_account: "MASTER_001".to_string(),
        timestamp: "2025-01-01T00:00:00Z".to_string(),
        status: 2,
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
    };

    let serialized = rmp_serde::to_vec_named(&config).expect("Failed to serialize");
    let deserialized: ConfigMessage =
        rmp_serde::from_slice(&serialized).expect("Failed to deserialize");

    assert_eq!(config.symbol_prefix, deserialized.symbol_prefix);
    assert_eq!(config.symbol_suffix, deserialized.symbol_suffix);
}

/// Test that ConfigMessage with Some symbol filters serializes correctly
#[test]
fn test_config_message_some_symbol_filters() {
    let config = ConfigMessage {
        account_id: "TEST_001".to_string(),
        master_account: "MASTER_001".to_string(),
        trade_group_id: "MASTER_001".to_string(),
        timestamp: "2025-01-01T00:00:00Z".to_string(),
        status: 2,
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
    };

    let serialized = rmp_serde::to_vec_named(&config).expect("Failed to serialize");
    let deserialized: ConfigMessage =
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
