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
