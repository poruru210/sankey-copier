//! Tests for configuration request handling

use super::*;
use crate::models::{LotCalculationMode, MasterSettings, RequestConfigMessage, SlaveSettings};

#[tokio::test]
async fn test_handle_request_config_master() {
    let ctx = create_test_context().await;
    let master_account = "MASTER_TEST_001".to_string();

    // Step 1: Create TradeGroup in DB with default Master settings
    ctx.db
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
    ctx.handle_message(zmq_msg).await;

    // Step 4: Verify no panic occurred (implementation will be added in Phase 3.2b)
    // In Red phase, this test logs warning because Master EA type is rejected
    // In Green phase, this test should pass after implementing Master config logic

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_handle_request_config_master_not_found() {
    let ctx = create_test_context().await;
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
    ctx.handle_message(zmq_msg).await;

    // Should not panic even if Master not found (graceful handling)
    ctx.cleanup().await;
}

#[tokio::test]
async fn test_handle_request_config_slave() {
    let ctx = create_test_context().await;
    let master_account = "MASTER123".to_string();
    let slave_account = "SLAVE456".to_string();

    // Create TradeGroup and add member
    ctx.db.create_trade_group(&master_account).await.unwrap();

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
        sync_mode: crate::models::SyncMode::Skip,
        limit_order_expiry_min: None,
        market_sync_max_pips: None,
        max_slippage: None,
        copy_pending_orders: false,
        // Trade Execution defaults
        max_retries: 3,
        max_signal_delay_ms: 5000,
        use_pending_order_for_delayed: false,
    };
    ctx.db
        .add_member(&master_account, &slave_account, slave_settings, 0)
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
    ctx.handle_message(zmq_msg).await;

    // Should successfully send config to Slave (no panic)
    ctx.cleanup().await;
}

#[tokio::test]
async fn test_handle_request_config_slave_not_found() {
    let ctx = create_test_context().await;
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
    ctx.handle_message(zmq_msg).await;

    // Should not panic even if Slave not found (graceful handling)
    ctx.cleanup().await;
}

#[tokio::test]
async fn test_handle_request_config_unknown_ea_type() {
    let ctx = create_test_context().await;

    // Create RequestConfig message with unknown EA type
    let request_msg = RequestConfigMessage {
        message_type: "RequestConfig".to_string(),
        account_id: "TEST123".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        ea_type: "UnknownType".to_string(), // Invalid EA type
    };

    // Call handle_request_config via handle_message
    let zmq_msg = crate::zeromq::ZmqMessage::RequestConfig(request_msg);
    ctx.handle_message(zmq_msg).await;

    // Should handle gracefully (log warning, no panic)
    ctx.cleanup().await;
}

#[tokio::test]
async fn test_slave_request_config_updates_runtime_status() {
    let ctx = create_test_context().await;
    let master_account = "MASTER_STATUS_A".to_string();
    let slave_account = "SLAVE_STATUS_A".to_string();

    ctx.db.create_trade_group(&master_account).await.unwrap();

    let master_settings = MasterSettings {
        enabled: true,
        symbol_prefix: None,
        symbol_suffix: None,
        config_version: 1,
    };
    ctx.db
        .update_master_settings(&master_account, master_settings)
        .await
        .unwrap();

    ctx.db
        .add_member(&master_account, &slave_account, SlaveSettings::default(), 0)
        .await
        .unwrap();

    ctx.db
        .update_member_enabled_flag(&master_account, &slave_account, true)
        .await
        .unwrap();

    let before = ctx
        .db
        .get_member(&master_account, &slave_account)
        .await
        .unwrap()
        .expect("member missing");
    assert_eq!(before.runtime_status, 0);

    ctx.connection_manager
        .update_heartbeat(HeartbeatMessage {
            message_type: "Heartbeat".to_string(),
            account_id: slave_account.clone(),
            balance: 1000.0,
            equity: 1000.0,
            open_positions: 0,
            timestamp: chrono::Utc::now().to_rfc3339(),
            version: "test-slave".to_string(),
            ea_type: "Slave".to_string(),
            platform: "MT5".to_string(),
            account_number: 42,
            broker: "TestBroker".to_string(),
            account_name: "SlaveStatus".to_string(),
            server: "TestServer".to_string(),
            currency: "USD".to_string(),
            leverage: 100,
            is_trade_allowed: true,
            symbol_prefix: None,
            symbol_suffix: None,
            symbol_map: None,
        })
        .await;

    let request_msg = RequestConfigMessage {
        message_type: "RequestConfig".to_string(),
        account_id: slave_account.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        ea_type: "Slave".to_string(),
    };
    let zmq_msg = crate::zeromq::ZmqMessage::RequestConfig(request_msg);
    ctx.handle_message(zmq_msg).await;

    let after = ctx
        .db
        .get_member(&master_account, &slave_account)
        .await
        .unwrap()
        .expect("member missing");
    assert_eq!(after.runtime_status, 1);
    assert_eq!(after.status, 1);

    ctx.cleanup().await;
}
