// mt-bridge/src/msgpack/tests/serialization_compat_tests.rs
//
// Unit tests to verify serialization compatibility between mt-bridge and relay-server
// types, specifically TradeSignalMessage (mt-bridge) and TradeSignal (relay-server).
//
// These tests ensure that messages serialized by mt-bridge can be correctly
// deserialized by relay-server's message handler.

use crate::types::TradeSignalMessage;
use chrono::Utc;

/// Simulated relay-server TradeSignal struct for testing deserialization compatibility
/// This mirrors the structure in relay-server/src/models/mod.rs
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct RelayServerTradeSignal {
    pub action: RelayServerTradeAction,
    pub ticket: i64,
    #[serde(default)]
    pub symbol: Option<String>,
    #[serde(default)]
    pub order_type: Option<RelayServerOrderType>,
    #[serde(default)]
    pub lots: Option<f64>,
    #[serde(default)]
    pub open_price: Option<f64>,
    #[serde(default)]
    pub stop_loss: Option<f64>,
    #[serde(default)]
    pub take_profit: Option<f64>,
    #[serde(default)]
    pub magic_number: Option<i32>, // NOTE: relay-server uses i32, mt-bridge uses i64!
    #[serde(default)]
    pub comment: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub source_account: String,
    #[serde(default)]
    pub close_ratio: Option<f64>,
}

/// Simulated relay-server TradeAction enum
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
enum RelayServerTradeAction {
    Open,
    Close,
    Modify,
}

/// Simulated relay-server OrderType enum
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
enum RelayServerOrderType {
    Buy,
    Sell,
    BuyLimit,
    SellLimit,
    BuyStop,
    SellStop,
}

#[test]
fn test_trade_signal_serialization_compatibility_open() {
    // Create a TradeSignalMessage as mt-bridge would
    let msg = TradeSignalMessage {
        action: "Open".to_string(),
        ticket: 12345,
        symbol: Some("EURUSD".to_string()),
        order_type: Some("Buy".to_string()),
        lots: Some(0.1),
        open_price: Some(1.1050),
        stop_loss: Some(1.1000),
        take_profit: Some(1.1100),
        magic_number: Some(123), // Small value that fits in i32
        comment: Some("Test Comment".to_string()),
        timestamp: Utc::now(),
        source_account: "MASTER_001".to_string(),
        close_ratio: None,
    };

    // Serialize using mt-bridge's method (to_vec_named)
    let serialized = rmp_serde::encode::to_vec_named(&msg)
        .expect("Failed to serialize TradeSignalMessage");

    // Print serialized bytes for debugging
    println!("Serialized bytes ({} total): {:02x?}", serialized.len(), &serialized[..serialized.len().min(100)]);

    // Attempt to deserialize as relay-server's TradeSignal
    let result: Result<RelayServerTradeSignal, _> = rmp_serde::from_slice(&serialized);
    
    match result {
        Ok(signal) => {
            println!("Successfully deserialized: {:?}", signal);
            assert_eq!(signal.action, RelayServerTradeAction::Open);
            assert_eq!(signal.ticket, 12345);
            assert_eq!(signal.symbol, Some("EURUSD".to_string()));
            assert_eq!(signal.lots, Some(0.1), "lots should be preserved as Some(0.1)");
            assert_eq!(signal.order_type, Some(RelayServerOrderType::Buy), "order_type should be Buy");
        }
        Err(e) => {
            panic!("Deserialization failed: {}. This indicates a type mismatch between mt-bridge and relay-server.", e);
        }
    }
}

#[test]
fn test_trade_signal_magic_number_i64_overflow() {
    // Create a TradeSignalMessage with a magic_number that exceeds i32 range
    let msg = TradeSignalMessage {
        action: "Open".to_string(),
        ticket: 12345,
        symbol: Some("EURUSD".to_string()),
        order_type: Some("Buy".to_string()),
        lots: Some(0.1),
        open_price: Some(1.1050),
        stop_loss: None,
        take_profit: None,
        magic_number: Some(i32::MAX as i64 + 1), // Value that doesn't fit in i32
        comment: None,
        timestamp: Utc::now(),
        source_account: "MASTER_001".to_string(),
        close_ratio: None,
    };

    // Serialize
    let serialized = rmp_serde::encode::to_vec_named(&msg)
        .expect("Failed to serialize TradeSignalMessage");

    // This should fail if relay-server uses i32 for magic_number
    let result: Result<RelayServerTradeSignal, _> = rmp_serde::from_slice(&serialized);
    
    if result.is_err() {
        println!("Expected failure for i64 overflow: {:?}", result.err());
    } else {
        println!("Warning: Deserialization succeeded even with i64 magic_number");
    }
}

#[test]
fn test_trade_signal_action_string_to_enum() {
    // Test that action="Open" (String) can be deserialized to TradeAction::Open (enum)
    let msg = TradeSignalMessage {
        action: "Open".to_string(),
        ticket: 12345,
        symbol: Some("EURUSD".to_string()),
        order_type: Some("Buy".to_string()),
        lots: Some(0.1),
        open_price: Some(1.1050),
        stop_loss: None,
        take_profit: None,
        magic_number: None,
        comment: None,
        timestamp: Utc::now(),
        source_account: "MASTER_001".to_string(),
        close_ratio: None,
    };

    let serialized = rmp_serde::encode::to_vec_named(&msg)
        .expect("Failed to serialize");

    println!("Serialized action field test: {:02x?}", &serialized[..serialized.len().min(50)]);

    let result: Result<RelayServerTradeSignal, _> = rmp_serde::from_slice(&serialized);
    match result {
        Ok(signal) => {
            assert_eq!(signal.action, RelayServerTradeAction::Open, "Action should be Open");
        }
        Err(e) => {
            panic!("Deserialization failed: {}. String 'Open' couldn't be deserialized to enum TradeAction::Open.", e);
        }
    }
}

#[test]
fn test_discriminator_detection() {
    // Test that the message can be detected by checking 'action' field presence
    // This simulates relay-server's MessageTypeDiscriminator logic
    
    #[derive(Debug, serde::Deserialize)]
    struct MessageTypeDiscriminator {
        #[serde(default)]
        message_type: Option<String>,
        #[serde(default)]
        action: Option<String>,
    }

    let msg = TradeSignalMessage {
        action: "Open".to_string(),
        ticket: 12345,
        symbol: Some("EURUSD".to_string()),
        order_type: Some("Buy".to_string()),
        lots: Some(0.1),
        open_price: Some(1.1050),
        stop_loss: None,
        take_profit: None,
        magic_number: None,
        comment: None,
        timestamp: Utc::now(),
        source_account: "MASTER_001".to_string(),
        close_ratio: None,
    };

    let serialized = rmp_serde::encode::to_vec_named(&msg)
        .expect("Failed to serialize");

    let discriminator: MessageTypeDiscriminator = rmp_serde::from_slice(&serialized)
        .expect("Failed to deserialize discriminator");

    println!("Discriminator result: message_type={:?}, action={:?}", 
             discriminator.message_type, discriminator.action);

    assert!(discriminator.message_type.is_none(), "message_type should be None for TradeSignal");
    assert_eq!(discriminator.action, Some("Open".to_string()), "action should be 'Open'");
}

/// Test reverse compatibility: relay-server TradeSignal -> mt-bridge TradeSignalMessage
/// This simulates the message path: Master -> relay-server -> Slave
#[test]
fn test_reverse_compatibility_relay_to_slave() {
    // Simulate relay-server's TradeSignal (enum-based)
    let relay_signal = RelayServerTradeSignal {
        action: RelayServerTradeAction::Open,
        ticket: 12345,
        symbol: Some("EURUSD".to_string()),
        order_type: Some(RelayServerOrderType::Buy),
        lots: Some(1.0),
        open_price: Some(1.1050),
        stop_loss: Some(1.1000),
        take_profit: Some(1.1100),
        magic_number: Some(123),
        comment: Some("Test".to_string()),
        timestamp: Utc::now(),
        source_account: "MASTER_001".to_string(),
        close_ratio: None,
    };

    // Serialize as relay-server would (to_vec_named)
    let serialized = rmp_serde::encode::to_vec_named(&relay_signal)
        .expect("Failed to serialize RelayServerTradeSignal");

    println!("Relay-server serialized bytes: {:02x?}", &serialized[..serialized.len().min(80)]);

    // Deserialize as mt-bridge TradeSignalMessage (what Slave EA expects)
    let result: Result<TradeSignalMessage, _> = rmp_serde::from_slice(&serialized);

    match result {
        Ok(signal) => {
            println!("Successfully deserialized as TradeSignalMessage: {:?}", signal);
            assert_eq!(signal.action, "Open", "action should be 'Open'");
            assert_eq!(signal.ticket, 12345);
            assert_eq!(signal.lots, Some(1.0), "lots should be Some(1.0)");
            assert_eq!(signal.order_type.as_deref(), Some("Buy"), "order_type should be 'Buy'");
        }
        Err(e) => {
            panic!(
                "Reverse deserialization failed: {}. relay-server TradeSignal format is incompatible with mt-bridge TradeSignalMessage.",
                e
            );
        }
    }
}

