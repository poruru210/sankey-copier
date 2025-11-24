//! Tests for ConnectionManager
//!
//! Verifies EA registration, heartbeat handling, timeout detection, and status management.

use super::*;

fn create_test_heartbeat_message(account_id: &str) -> HeartbeatMessage {
    HeartbeatMessage {
        message_type: "Heartbeat".to_string(),
        account_id: account_id.to_string(),
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
    }
}

#[tokio::test]
async fn test_unregister_ea() {
    let manager = ConnectionManager::new(30);
    let msg = create_test_heartbeat_message("TEST_001");
    let account_id = msg.account_id.clone();

    // Auto-register via heartbeat
    manager.update_heartbeat(msg).await;

    // Verify registered
    let ea = manager.get_ea(&account_id).await;
    assert!(ea.is_some());
    assert_eq!(ea.unwrap().status, ConnectionStatus::Online);

    // Unregister
    manager.unregister_ea(&account_id).await;

    let ea = manager.get_ea(&account_id).await;
    assert!(ea.is_some());
    assert_eq!(ea.unwrap().status, ConnectionStatus::Offline);
}

#[tokio::test]
async fn test_update_heartbeat() {
    let manager = ConnectionManager::new(30);
    let account_id = "TEST_001".to_string();

    // First heartbeat: auto-registers the EA
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
    manager.update_heartbeat(hb_msg).await;

    // Second heartbeat: updates balance and equity
    let hb_msg2 = HeartbeatMessage {
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
    manager.update_heartbeat(hb_msg2).await;

    let ea = manager.get_ea(&account_id).await;
    assert!(ea.is_some());
    let ea = ea.unwrap();
    assert_eq!(ea.balance, 12000.0);
    assert_eq!(ea.equity, 11500.0);
    assert_eq!(ea.status, ConnectionStatus::Online);
}

#[tokio::test]
async fn test_get_all_eas() {
    let manager = ConnectionManager::new(30);

    // Auto-register two EAs via heartbeat
    manager
        .update_heartbeat(create_test_heartbeat_message("TEST_001"))
        .await;
    manager
        .update_heartbeat(create_test_heartbeat_message("TEST_002"))
        .await;

    let eas = manager.get_all_eas().await;
    assert_eq!(eas.len(), 2);
}

#[tokio::test]
async fn test_timeout_check() {
    let manager = ConnectionManager::new(1); // 1 second timeout
    let msg = create_test_heartbeat_message("TEST_001");
    let account_id = msg.account_id.clone();

    // Auto-register via heartbeat
    manager.update_heartbeat(msg).await;

    // Verify initially online
    let ea = manager.get_ea(&account_id).await;
    assert_eq!(ea.unwrap().status, ConnectionStatus::Online);

    // Wait for timeout
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Run timeout check
    let timed_out = manager.check_timeouts().await;

    // Verify one EA timed out
    assert_eq!(timed_out.len(), 1);
    assert_eq!(timed_out[0].0, account_id);
    assert_eq!(timed_out[0].1, EaType::Master);

    // Verify timed out status
    let ea = manager.get_ea(&account_id).await;
    assert_eq!(ea.unwrap().status, ConnectionStatus::Timeout);
}

#[tokio::test]
async fn test_heartbeat_prevents_timeout() {
    let manager = ConnectionManager::new(2); // 2 second timeout
    let msg = create_test_heartbeat_message("TEST_001");
    let account_id = msg.account_id.clone();

    // Auto-register via heartbeat
    manager.update_heartbeat(msg).await;

    // Send heartbeat after 1 second
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    manager
        .update_heartbeat(HeartbeatMessage {
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
        })
        .await;

    // Wait another second (total 2 seconds, but heartbeat was sent at 1 second)
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Run timeout check
    let timed_out = manager.check_timeouts().await;

    // Should not have timed out because heartbeat was sent within timeout
    assert_eq!(timed_out.len(), 0);

    // Should still be online
    let ea = manager.get_ea(&account_id).await;
    assert_eq!(ea.unwrap().status, ConnectionStatus::Online);
}

#[tokio::test]
async fn test_heartbeat_auto_registration() {
    let manager = ConnectionManager::new(30);

    // Send heartbeat without prior registration
    let hb_msg = HeartbeatMessage {
        message_type: "Heartbeat".to_string(),
        account_id: "TEST_NEW".to_string(),
        balance: 15000.0,
        equity: 15500.0,
        open_positions: 2,
        timestamp: chrono::Utc::now().to_rfc3339(),
        version: "test123".to_string(),
        ea_type: "Slave".to_string(),
        platform: "MT5".to_string(),
        account_number: 67890,
        broker: "New Broker".to_string(),
        account_name: "New Account".to_string(),
        server: "NewServer-Live".to_string(),
        currency: "EUR".to_string(),
        leverage: 200,
        is_trade_allowed: true,
        symbol_prefix: None,
        symbol_suffix: None,
        symbol_map: None,
    };

    manager.update_heartbeat(hb_msg).await;

    // Verify EA was auto-registered
    let ea = manager.get_ea("TEST_NEW").await;
    assert!(ea.is_some(), "EA should be auto-registered from heartbeat");

    let ea = ea.unwrap();
    assert_eq!(ea.account_id, "TEST_NEW");
    assert_eq!(ea.ea_type, EaType::Slave);
    assert_eq!(ea.platform, Platform::MT5);
    assert_eq!(ea.account_number, 67890);
    assert_eq!(ea.broker, "New Broker");
    assert_eq!(ea.balance, 15000.0);
    assert_eq!(ea.equity, 15500.0);
    assert_eq!(ea.currency, "EUR");
    assert_eq!(ea.leverage, 200);
    assert_eq!(ea.status, ConnectionStatus::Online);
}
