use super::*;
use crate::models::{HeartbeatMessage, OrderType, RequestConfigMessage, TradeAction, TradeSignal, UnregisterMessage};
use chrono::Utc;

/// Test that TradeSignal messages can be distinguished by the presence of 'action' field
#[test]
fn test_message_discriminator_trade_signal() {
    let signal = TradeSignal {
        action: TradeAction::Open,
        ticket: 12345,
        symbol: "EURUSD".to_string(),
        order_type: OrderType::Buy,
        lots: 0.1,
        open_price: 1.1000,
        stop_loss: Some(1.0950),
        take_profit: Some(1.1050),
        magic_number: 0,
        comment: "Test".to_string(),
        timestamp: Utc::now(),
        source_account: "MASTER_001".to_string(),
    };

    let bytes = rmp_serde::to_vec_named(&signal).unwrap();

    // Should successfully deserialize as MessageTypeDiscriminator
    let discriminator: MessageTypeDiscriminator = rmp_serde::from_slice(&bytes).unwrap();
    assert!(discriminator.message_type.is_none());
    assert!(discriminator.action.is_some());

    // Should successfully deserialize as TradeSignal
    let deserialized: TradeSignal = rmp_serde::from_slice(&bytes).unwrap();
    assert_eq!(deserialized.symbol, "EURUSD");
    assert_eq!(deserialized.lots, 0.1);
}

/// Test that Heartbeat messages can be distinguished by message_type field
#[test]
fn test_message_discriminator_heartbeat() {
    let heartbeat = HeartbeatMessage {
        message_type: "Heartbeat".to_string(),
        account_id: "TEST_001".to_string(),
        balance: 10000.0,
        equity: 9800.0,
        open_positions: 2,
        timestamp: Utc::now().to_rfc3339(),
        version: "1.0.0".to_string(),
        ea_type: "Master".to_string(),
        platform: "MT5".to_string(),
        account_number: 12345,
        broker: "Test Broker".to_string(),
        account_name: "Test Account".to_string(),
        server: "Test-Server".to_string(),
        currency: "USD".to_string(),
        leverage: 100,
    };

    let bytes = rmp_serde::to_vec_named(&heartbeat).unwrap();

    // Should successfully deserialize as MessageTypeDiscriminator
    let discriminator: MessageTypeDiscriminator = rmp_serde::from_slice(&bytes).unwrap();
    assert_eq!(discriminator.message_type, Some("Heartbeat".to_string()));
    assert!(discriminator.action.is_none());

    // Should successfully deserialize as HeartbeatMessage
    let deserialized: HeartbeatMessage = rmp_serde::from_slice(&bytes).unwrap();
    assert_eq!(deserialized.account_id, "TEST_001");
    assert_eq!(deserialized.balance, 10000.0);
}

/// Test that RequestConfig messages can be distinguished by message_type field
#[test]
fn test_message_discriminator_request_config() {
    let request = RequestConfigMessage {
        message_type: "RequestConfig".to_string(),
        account_id: "SLAVE_001".to_string(),
        timestamp: Utc::now().to_rfc3339(),
    };

    let bytes = rmp_serde::to_vec_named(&request).unwrap();

    // Should successfully deserialize as MessageTypeDiscriminator
    let discriminator: MessageTypeDiscriminator = rmp_serde::from_slice(&bytes).unwrap();
    assert_eq!(discriminator.message_type, Some("RequestConfig".to_string()));
    assert!(discriminator.action.is_none());

    // Should successfully deserialize as RequestConfigMessage
    let deserialized: RequestConfigMessage = rmp_serde::from_slice(&bytes).unwrap();
    assert_eq!(deserialized.account_id, "SLAVE_001");
}

/// Test that Unregister messages can be distinguished by message_type field
#[test]
fn test_message_discriminator_unregister() {
    let unregister = UnregisterMessage {
        message_type: "Unregister".to_string(),
        account_id: "TEST_001".to_string(),
        timestamp: Utc::now().to_rfc3339(),
    };

    let bytes = rmp_serde::to_vec_named(&unregister).unwrap();

    // Should successfully deserialize as MessageTypeDiscriminator
    let discriminator: MessageTypeDiscriminator = rmp_serde::from_slice(&bytes).unwrap();
    assert_eq!(discriminator.message_type, Some("Unregister".to_string()));
    assert!(discriminator.action.is_none());

    // Should successfully deserialize as UnregisterMessage
    let deserialized: UnregisterMessage = rmp_serde::from_slice(&bytes).unwrap();
    assert_eq!(deserialized.account_id, "TEST_001");
}

/// Test that FlexibleHeartbeat can extract account_id from partial data
#[test]
fn test_flexible_heartbeat_partial_data() {
    // Create a minimal map with just account_id
    let mut map = std::collections::HashMap::new();
    map.insert("account_id", "PARTIAL_001");

    let bytes = rmp_serde::to_vec_named(&map).unwrap();

    // Should successfully deserialize as FlexibleHeartbeat
    let flexible: FlexibleHeartbeat = rmp_serde::from_slice(&bytes).unwrap();
    assert_eq!(flexible.account_id, Some("PARTIAL_001".to_string()));
}

/// Test that FlexibleHeartbeat handles missing account_id
#[test]
fn test_flexible_heartbeat_missing_account_id() {
    // Create an empty map
    let map: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    let bytes = rmp_serde::to_vec_named(&map).unwrap();

    // Should successfully deserialize with None
    let flexible: FlexibleHeartbeat = rmp_serde::from_slice(&bytes).unwrap();
    assert_eq!(flexible.account_id, None);
}

/// Test that invalid MessagePack data fails to deserialize
#[test]
fn test_invalid_messagepack_data() {
    // Create invalid MessagePack data
    let invalid_bytes = vec![0xFF, 0xFF, 0xFF, 0xFF];

    // Should fail to deserialize
    let result: Result<MessageTypeDiscriminator, _> = rmp_serde::from_slice(&invalid_bytes);
    assert!(result.is_err());
}

/// Test that messages without message_type or action fields fail discrimination
#[test]
fn test_message_without_discriminator_fields() {
    // Create a message without message_type or action
    let mut map = std::collections::HashMap::new();
    map.insert("unknown_field", "value");

    let bytes = rmp_serde::to_vec_named(&map).unwrap();

    // Should deserialize as discriminator but both fields should be None
    let discriminator: MessageTypeDiscriminator = rmp_serde::from_slice(&bytes).unwrap();
    assert!(discriminator.message_type.is_none());
    assert!(discriminator.action.is_none());
}

/// Test ZmqPublisher topic formatting
#[test]
fn test_publish_message_topic_format() {
    let msg = PublishMessage {
        topic: "MASTER_001".to_string(),
        payload: TradeSignal {
            action: TradeAction::Open,
            ticket: 12345,
            symbol: "EURUSD".to_string(),
            order_type: OrderType::Buy,
            lots: 0.1,
            open_price: 1.1000,
            stop_loss: None,
            take_profit: None,
            magic_number: 0,
            comment: "".to_string(),
            timestamp: Utc::now(),
            source_account: "MASTER_001".to_string(),
        },
    };

    // Verify topic string is properly formatted
    assert_eq!(msg.topic, "MASTER_001");
    assert!(!msg.topic.is_empty());
}

/// Test that different trade actions serialize/deserialize correctly
#[test]
fn test_trade_action_variants() {
    let actions = vec![
        TradeAction::Open,
        TradeAction::Close,
        TradeAction::Modify,
    ];

    for action in actions {
        let signal = TradeSignal {
            action: action.clone(),
            ticket: 12345,
            symbol: "EURUSD".to_string(),
            order_type: OrderType::Buy,
            lots: 0.1,
            open_price: 1.1000,
            stop_loss: None,
            take_profit: None,
            magic_number: 0,
            comment: "".to_string(),
            timestamp: Utc::now(),
            source_account: "MASTER_001".to_string(),
        };

        let bytes = rmp_serde::to_vec_named(&signal).unwrap();
        let deserialized: TradeSignal = rmp_serde::from_slice(&bytes).unwrap();

        // Verify action is preserved
        assert_eq!(
            std::mem::discriminant(&deserialized.action),
            std::mem::discriminant(&action)
        );
    }
}

/// Test that order types serialize/deserialize correctly
#[test]
fn test_order_type_variants() {
    let order_types = vec![
        OrderType::Buy,
        OrderType::Sell,
        OrderType::BuyLimit,
        OrderType::SellLimit,
        OrderType::BuyStop,
        OrderType::SellStop,
    ];

    for order_type in order_types {
        let signal = TradeSignal {
            action: TradeAction::Open,
            ticket: 12345,
            symbol: "EURUSD".to_string(),
            order_type: order_type.clone(),
            lots: 0.1,
            open_price: 1.1000,
            stop_loss: None,
            take_profit: None,
            magic_number: 0,
            comment: "".to_string(),
            timestamp: Utc::now(),
            source_account: "MASTER_001".to_string(),
        };

        let bytes = rmp_serde::to_vec_named(&signal).unwrap();
        let deserialized: TradeSignal = rmp_serde::from_slice(&bytes).unwrap();

        // Verify order_type is preserved
        assert_eq!(
            std::mem::discriminant(&deserialized.order_type),
            std::mem::discriminant(&order_type)
        );
    }
}
