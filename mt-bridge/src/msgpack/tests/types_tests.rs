// Location: mt-bridge/src/msgpack/tests/types_tests.rs
// Purpose: Tests for MessagePack type serialization/deserialization
// Why: Ensures all message types correctly roundtrip through MessagePack format

use crate::msgpack::*;

#[test]
fn test_request_config_message_serialization() {
    let msg = RequestConfigMessage {
        message_type: "RequestConfig".to_string(),
        account_id: "test_account_123".to_string(),
        timestamp: "2025-01-01T00:00:00Z".to_string(),
        ea_type: "Slave".to_string(),
    };

    // Serialize
    let serialized = rmp_serde::to_vec_named(&msg).expect("Failed to serialize");
    assert!(
        !serialized.is_empty(),
        "Serialized data should not be empty"
    );

    // Deserialize
    let deserialized: RequestConfigMessage =
        rmp_serde::from_slice(&serialized).expect("Failed to deserialize");

    // Verify fields
    assert_eq!(msg.message_type, deserialized.message_type);
    assert_eq!(msg.account_id, deserialized.account_id);
    assert_eq!(msg.timestamp, deserialized.timestamp);
    assert_eq!(msg.ea_type, deserialized.ea_type);
}

#[test]
fn test_unregister_message_serialization() {
    let msg = UnregisterMessage {
        message_type: "Unregister".to_string(),
        account_id: "test_account_123".to_string(),
        timestamp: "2025-01-01T00:00:00Z".to_string(),
    };

    let serialized = rmp_serde::to_vec_named(&msg).expect("Failed to serialize");
    let deserialized: UnregisterMessage =
        rmp_serde::from_slice(&serialized).expect("Failed to deserialize");

    assert_eq!(msg.message_type, deserialized.message_type);
    assert_eq!(msg.account_id, deserialized.account_id);
    assert_eq!(msg.timestamp, deserialized.timestamp);
}

#[test]
fn test_heartbeat_message_serialization() {
    let msg = HeartbeatMessage {
        message_type: "Heartbeat".to_string(),
        account_id: "test_account_123".to_string(),
        balance: 10500.75,
        equity: 10600.25,
        open_positions: 3,
        timestamp: "2025-01-01T00:00:00Z".to_string(),
        version: "test123".to_string(),
        ea_type: "Master".to_string(),
        platform: "MT5".to_string(),
        account_number: 12345,
        broker: "TestBroker".to_string(),
        account_name: "Test Account".to_string(),
        server: "TestServer-Live".to_string(),
        currency: "USD".to_string(),
        leverage: 100,
        is_trade_allowed: true,
        symbol_prefix: Some("pro.".to_string()),
        symbol_suffix: Some(".m".to_string()),
        symbol_map: Some("XAUUSD=GOLD".to_string()),
    };

    let serialized = rmp_serde::to_vec_named(&msg).expect("Failed to serialize");
    let deserialized: HeartbeatMessage =
        rmp_serde::from_slice(&serialized).expect("Failed to deserialize");

    assert_eq!(msg.message_type, deserialized.message_type);
    assert_eq!(msg.account_id, deserialized.account_id);
    assert_eq!(msg.balance, deserialized.balance);
    assert_eq!(msg.equity, deserialized.equity);
    assert_eq!(msg.open_positions, deserialized.open_positions);
    assert_eq!(msg.timestamp, deserialized.timestamp);
    assert_eq!(msg.ea_type, deserialized.ea_type);
    assert_eq!(msg.platform, deserialized.platform);
    assert_eq!(msg.account_number, deserialized.account_number);
    assert_eq!(msg.broker, deserialized.broker);
    assert_eq!(msg.account_name, deserialized.account_name);
    assert_eq!(msg.server, deserialized.server);
    assert_eq!(msg.currency, deserialized.currency);
    assert_eq!(msg.leverage, deserialized.leverage);
    assert_eq!(msg.is_trade_allowed, deserialized.is_trade_allowed);
    assert_eq!(msg.symbol_prefix, deserialized.symbol_prefix);
    assert_eq!(msg.symbol_suffix, deserialized.symbol_suffix);
    assert_eq!(msg.symbol_map, deserialized.symbol_map);
}

#[test]
fn test_trade_signal_message_serialization() {
    let msg = TradeSignalMessage {
        action: "Open".to_string(),
        ticket: 123456,
        symbol: Some("EURUSD".to_string()),
        order_type: Some("Buy".to_string()),
        lots: Some(0.1),
        open_price: Some(1.0850),
        stop_loss: Some(1.0800),
        take_profit: Some(1.0900),
        magic_number: Some(0),
        comment: Some("Test trade".to_string()),
        timestamp: "2025-01-01T00:00:00Z".to_string(),
        source_account: "master_account".to_string(),
    };

    let serialized = rmp_serde::to_vec_named(&msg).expect("Failed to serialize");
    let deserialized: TradeSignalMessage =
        rmp_serde::from_slice(&serialized).expect("Failed to deserialize");

    assert_eq!(msg.action, deserialized.action);
    assert_eq!(msg.ticket, deserialized.ticket);
    assert_eq!(msg.symbol, deserialized.symbol);
    assert_eq!(msg.order_type, deserialized.order_type);
    assert_eq!(msg.lots, deserialized.lots);
    assert_eq!(msg.open_price, deserialized.open_price);
    assert_eq!(msg.stop_loss, deserialized.stop_loss);
    assert_eq!(msg.take_profit, deserialized.take_profit);
    assert_eq!(msg.magic_number, deserialized.magic_number);
    assert_eq!(msg.comment, deserialized.comment);
    assert_eq!(msg.timestamp, deserialized.timestamp);
    assert_eq!(msg.source_account, deserialized.source_account);
}

#[test]
fn test_trade_signal_close_action() {
    // Close action should have minimal fields
    let msg = TradeSignalMessage {
        action: "Close".to_string(),
        ticket: 123456,
        symbol: None,
        order_type: None,
        lots: None,
        open_price: None,
        stop_loss: None,
        take_profit: None,
        magic_number: None,
        comment: None,
        timestamp: "2025-01-01T00:00:00Z".to_string(),
        source_account: "master_account".to_string(),
    };

    let serialized = rmp_serde::to_vec_named(&msg).expect("Failed to serialize");
    let deserialized: TradeSignalMessage =
        rmp_serde::from_slice(&serialized).expect("Failed to deserialize");

    assert_eq!(msg.action, deserialized.action);
    assert_eq!(msg.ticket, deserialized.ticket);
    assert!(deserialized.symbol.is_none());
    assert!(deserialized.order_type.is_none());
    assert!(deserialized.lots.is_none());
}

#[test]
fn test_config_message_serialization() {
    let config = SlaveConfigMessage {
        account_id: "slave_account_123".to_string(),
        master_account: "master_account_456".to_string(),
        timestamp: "2025-01-01T00:00:00Z".to_string(),
        status: 2, // STATUS_CONNECTED
        lot_multiplier: Some(1.5),
        reverse_trade: false,
        symbol_prefix: None,
        symbol_suffix: None,
        symbol_mappings: vec![SymbolMapping {
            source_symbol: "EURUSD".to_string(),
            target_symbol: "EURUSD.raw".to_string(),
        }],
        filters: TradeFilters {
            allowed_symbols: Some(vec!["EURUSD".to_string(), "GBPUSD".to_string()]),
            blocked_symbols: None,
            allowed_magic_numbers: Some(vec![0, 123]),
            blocked_magic_numbers: None,
        },
        config_version: 1,
    };

    let serialized = rmp_serde::to_vec_named(&config).expect("Failed to serialize");
    let deserialized: SlaveConfigMessage =
        rmp_serde::from_slice(&serialized).expect("Failed to deserialize");

    assert_eq!(config.account_id, deserialized.account_id);
    assert_eq!(config.master_account, deserialized.master_account);
    assert_eq!(config.status, deserialized.status);
    assert_eq!(config.lot_multiplier, deserialized.lot_multiplier);
    assert_eq!(config.reverse_trade, deserialized.reverse_trade);
    assert_eq!(
        config.symbol_mappings.len(),
        deserialized.symbol_mappings.len()
    );
    assert_eq!(config.config_version, deserialized.config_version);
}

#[test]
fn test_messagepack_size_optimization() {
    // Test that optional None fields are omitted in serialization
    let msg_full = TradeSignalMessage {
        action: "Open".to_string(),
        ticket: 123456,
        symbol: Some("EURUSD".to_string()),
        order_type: Some("Buy".to_string()),
        lots: Some(0.1),
        open_price: Some(1.0850),
        stop_loss: Some(1.0800),
        take_profit: Some(1.0900),
        magic_number: Some(0),
        comment: Some("Test".to_string()),
        timestamp: "2025-01-01T00:00:00Z".to_string(),
        source_account: "master".to_string(),
    };

    let msg_minimal = TradeSignalMessage {
        action: "Close".to_string(),
        ticket: 123456,
        symbol: None,
        order_type: None,
        lots: None,
        open_price: None,
        stop_loss: None,
        take_profit: None,
        magic_number: None,
        comment: None,
        timestamp: "2025-01-01T00:00:00Z".to_string(),
        source_account: "master".to_string(),
    };

    let serialized_full = rmp_serde::to_vec_named(&msg_full).unwrap();
    let serialized_minimal = rmp_serde::to_vec_named(&msg_minimal).unwrap();

    // Minimal message should be smaller
    assert!(
        serialized_minimal.len() < serialized_full.len(),
        "Minimal message ({} bytes) should be smaller than full message ({} bytes)",
        serialized_minimal.len(),
        serialized_full.len()
    );
}

#[test]
fn test_master_config_message_serialization() {
    let msg = MasterConfigMessage {
        account_id: "master_account_123".to_string(),
        symbol_prefix: Some("pro.".to_string()),
        symbol_suffix: Some(".m".to_string()),
        config_version: 1,
        timestamp: "2025-01-01T00:00:00Z".to_string(),
    };

    // Serialize
    let serialized = rmp_serde::to_vec_named(&msg).expect("Failed to serialize");
    assert!(
        !serialized.is_empty(),
        "Serialized data should not be empty"
    );

    // Deserialize
    let deserialized: MasterConfigMessage =
        rmp_serde::from_slice(&serialized).expect("Failed to deserialize");

    // Verify fields
    assert_eq!(msg.account_id, deserialized.account_id);
    assert_eq!(msg.symbol_prefix, deserialized.symbol_prefix);
    assert_eq!(msg.symbol_suffix, deserialized.symbol_suffix);
    assert_eq!(msg.config_version, deserialized.config_version);
    assert_eq!(msg.timestamp, deserialized.timestamp);
}

#[test]
fn test_master_config_message_with_none_values() {
    let msg = MasterConfigMessage {
        account_id: "master_account_456".to_string(),
        symbol_prefix: None,
        symbol_suffix: None,
        config_version: 2,
        timestamp: "2025-01-02T12:30:00Z".to_string(),
    };

    let serialized = rmp_serde::to_vec_named(&msg).expect("Failed to serialize");
    let deserialized: MasterConfigMessage =
        rmp_serde::from_slice(&serialized).expect("Failed to deserialize");

    assert_eq!(msg.account_id, deserialized.account_id);
    assert!(deserialized.symbol_prefix.is_none());
    assert!(deserialized.symbol_suffix.is_none());
    assert_eq!(msg.config_version, deserialized.config_version);
}
