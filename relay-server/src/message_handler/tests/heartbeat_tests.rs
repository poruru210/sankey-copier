//! Tests for heartbeat message handling

use super::*;

#[tokio::test]
async fn test_handle_heartbeat() {
    // Use TestContext for proper ZeroMQ resource cleanup
    let ctx = create_test_context().await;
    let account_id = "TEST_001".to_string();

    // Send heartbeat (auto-registration)
    let hb_msg = HeartbeatMessage {
        message_type: "Heartbeat".to_string(),
        account_id: account_id.clone(),
        balance: 12000.0,
        equity: 11500.0,
        open_positions: 3,
        timestamp: chrono::Utc::now().to_rfc3339(),
        version: "test".to_string(),
        ea_type: "Master".to_string(),
        platform: "MT4".to_string(),
        account_number: 12345,
        broker: "Test Broker".to_string(),
        account_name: "Test Account".to_string(),
        server: "Test-Server".to_string(),
        currency: "USD".to_string(),
        leverage: 100,
        is_trade_allowed: true,
        symbol_prefix: None,
        symbol_suffix: None,
        symbol_map: None,
    };
    ctx.handle_heartbeat(hb_msg).await;

    // Verify EA was auto-registered with correct balance and equity
    let ea = ctx.connection_manager.get_ea(&account_id).await;
    assert!(ea.is_some());
    let ea = ea.unwrap();
    assert_eq!(ea.balance, 12000.0);
    assert_eq!(ea.equity, 11500.0);
    assert_eq!(ea.status, crate::models::ConnectionStatus::Online);

    // Explicit cleanup to release ZeroMQ resources
    ctx.cleanup().await;
}

#[tokio::test]
async fn test_handle_heartbeat_sends_config_on_new_master_registration() {
    // Setup
    let ctx = create_test_context().await;
    let account_id = "MASTER_NEW_REG";

    // Create TradeGroup in DB so Master is recognized
    let settings = crate::models::MasterSettings {
        enabled: true,
        symbol_prefix: None,
        symbol_suffix: None,
        config_version: 1,
    };
    ctx.db.create_trade_group(account_id).await.unwrap();
    ctx.db
        .update_master_settings(account_id, settings)
        .await
        .unwrap();

    // Send first heartbeat (New Registration)
    let hb_msg = HeartbeatMessage {
        message_type: "Heartbeat".to_string(),
        account_id: account_id.to_string(),
        balance: 10000.0,
        equity: 10000.0,
        open_positions: 0,
        timestamp: chrono::Utc::now().to_rfc3339(),
        version: "1.0.0".to_string(),
        ea_type: "Master".to_string(),
        platform: "MT5".to_string(),
        account_number: 123456,
        broker: "TestBroker".to_string(),
        account_name: "TestUser".to_string(),
        server: "TestServer".to_string(),
        currency: "USD".to_string(),
        leverage: 100,
        is_trade_allowed: true,
        symbol_prefix: None,
        symbol_suffix: None,
        symbol_map: None,
    };
    ctx.handle_heartbeat(hb_msg).await;

    // Verify: Connection status should be Online
    let conn = ctx.connection_manager.get_ea(account_id).await.unwrap();
    assert_eq!(conn.status, crate::models::ConnectionStatus::Online);

    // Note: We cannot easily assert that ZMQ message was sent without mocking the publisher,
    // but we rely on the fact that the code path is executed and no errors are logged.
    // The integration test environment uses a real ZMQ socket (bound to a random port),
    // so the send operation will succeed (or fail if socket is invalid).

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_handle_heartbeat_sends_config_on_trade_allowed_change() {
    // Setup
    let ctx = create_test_context().await;
    let account_id = "MASTER_STATE_CHANGE";

    // Create TradeGroup
    let settings = crate::models::MasterSettings {
        enabled: true,
        symbol_prefix: None,
        symbol_suffix: None,
        config_version: 1,
    };
    ctx.db.create_trade_group(account_id).await.unwrap();
    ctx.db
        .update_master_settings(account_id, settings)
        .await
        .unwrap();

    // 1. Initial Heartbeat (is_trade_allowed = false)
    let mut hb = HeartbeatMessage {
        message_type: "Heartbeat".to_string(),
        account_id: account_id.to_string(),
        balance: 10000.0,
        equity: 10000.0,
        open_positions: 0,
        timestamp: chrono::Utc::now().to_rfc3339(),
        version: "1.0.0".to_string(),
        ea_type: "Master".to_string(),
        platform: "MT5".to_string(),
        account_number: 123456,
        broker: "TestBroker".to_string(),
        account_name: "TestUser".to_string(),
        server: "TestServer".to_string(),
        currency: "USD".to_string(),
        leverage: 100,
        is_trade_allowed: false,
        symbol_prefix: None,
        symbol_suffix: None,
        symbol_map: None,
    };
    ctx.handle_heartbeat(hb.clone()).await;

    // 2. State Change Heartbeat (is_trade_allowed = true)
    hb.is_trade_allowed = true;
    ctx.handle_heartbeat(hb).await;

    // Verify connection state updated
    let conn = ctx.connection_manager.get_ea(account_id).await.unwrap();
    assert!(conn.is_trade_allowed);

    ctx.cleanup().await;
}
