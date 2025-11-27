//! Tests for unregister message handling

use super::*;
use crate::models::UnregisterMessage;

#[tokio::test]
async fn test_handle_unregister() {
    let ctx = create_test_context().await;
    let account_id = "TEST_001".to_string();

    // First auto-register via heartbeat
    let hb_msg = HeartbeatMessage {
        message_type: "Heartbeat".to_string(),
        account_id: account_id.clone(),
        balance: 10000.0,
        equity: 10000.0,
        open_positions: 0,
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

    // Then unregister
    ctx.handle_unregister(UnregisterMessage {
        message_type: "Unregister".to_string(),
        account_id: account_id.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
    .await;

    // Verify EA status is Offline
    let ea = ctx.connection_manager.get_ea(&account_id).await;
    assert!(ea.is_some());
    assert_eq!(ea.unwrap().status, crate::models::ConnectionStatus::Offline);

    ctx.cleanup().await;
}
