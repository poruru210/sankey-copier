//! Tests for configuration request handling

use super::*;
use crate::models::{LotCalculationMode, RequestConfigMessage, SlaveSettings};

#[tokio::test]
async fn test_handle_request_config_master() {
    let handler = create_test_handler().await;
    let master_account = "MASTER_TEST_001".to_string();

    // Step 1: Create TradeGroup in DB with default Master settings
    handler
        .db
        .create_trade_group(&master_account)
        .await
        .expect("Failed to create trade group");

    // Step 2: Create RequestConfig message with ea_type="Master"
    let request_msg = RequestConfigMessage {
        message_type: "RequestConfig".to_string(),
        account_id: master_account.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        ea_type: "Master".to_string(),
    };

    // Step 3: Call handle_request_config via handle_message
    let zmq_msg = crate::zeromq::ZmqMessage::RequestConfig(request_msg);
    handler.handle_message(zmq_msg).await;

    // Step 4: Verify no panic occurred (implementation will be added in Phase 3.2b)
    // In Red phase, this test logs warning because Master EA type is rejected
    // In Green phase, this test should pass after implementing Master config logic
}

#[tokio::test]
async fn test_handle_request_config_master_not_found() {
    let handler = create_test_handler().await;
    let master_account = "NONEXISTENT_MASTER".to_string();

    // Create RequestConfig message for non-existent Master
    let request_msg = RequestConfigMessage {
        message_type: "RequestConfig".to_string(),
        account_id: master_account.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        ea_type: "Master".to_string(),
    };

    // Call handle_request_config via handle_message
    let zmq_msg = crate::zeromq::ZmqMessage::RequestConfig(request_msg);
    handler.handle_message(zmq_msg).await;

    // Should not panic even if Master not found (graceful handling)
}

#[tokio::test]
async fn test_handle_request_config_slave() {
    let handler = create_test_handler().await;
    let master_account = "MASTER123".to_string();
    let slave_account = "SLAVE456".to_string();

    // Create TradeGroup and add member
    handler
        .db
        .create_trade_group(&master_account)
        .await
        .unwrap();

    let slave_settings = SlaveSettings {
        lot_calculation_mode: LotCalculationMode::default(),
        config_version: 1,
        symbol_prefix: Some("pro.".to_string()),
        symbol_suffix: Some(".m".to_string()),
        lot_multiplier: Some(2.0),
        reverse_trade: false,
        symbol_mappings: vec![],
        filters: TradeFilters::default(),
        source_lot_min: None,
        source_lot_max: None,
    };
    handler
        .db
        .add_member(&master_account, &slave_account, slave_settings)
        .await
        .unwrap();

    // Create RequestConfig message for Slave
    let request_msg = RequestConfigMessage {
        message_type: "RequestConfig".to_string(),
        account_id: slave_account.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        ea_type: "Slave".to_string(),
    };

    // Call handle_request_config via handle_message
    let zmq_msg = crate::zeromq::ZmqMessage::RequestConfig(request_msg);
    handler.handle_message(zmq_msg).await;

    // Should successfully send config to Slave (no panic)
}

#[tokio::test]
async fn test_handle_request_config_slave_not_found() {
    let handler = create_test_handler().await;
    let slave_account = "NONEXISTENT_SLAVE".to_string();

    // Create RequestConfig message for non-existent Slave
    let request_msg = RequestConfigMessage {
        message_type: "RequestConfig".to_string(),
        account_id: slave_account.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        ea_type: "Slave".to_string(),
    };

    // Call handle_request_config via handle_message
    let zmq_msg = crate::zeromq::ZmqMessage::RequestConfig(request_msg);
    handler.handle_message(zmq_msg).await;

    // Should not panic even if Slave not found (graceful handling)
}

#[tokio::test]
async fn test_handle_request_config_unknown_ea_type() {
    let handler = create_test_handler().await;

    // Create RequestConfig message with unknown EA type
    let request_msg = RequestConfigMessage {
        message_type: "RequestConfig".to_string(),
        account_id: "TEST123".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        ea_type: "UnknownType".to_string(), // Invalid EA type
    };

    // Call handle_request_config via handle_message
    let zmq_msg = crate::zeromq::ZmqMessage::RequestConfig(request_msg);
    handler.handle_message(zmq_msg).await;

    // Should handle gracefully (log warning, no panic)
}
