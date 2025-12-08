//! Tests for ConnectionManager
//!
//! Verifies EA registration, heartbeat handling, timeout detection, and status management.

use super::*;

fn create_test_heartbeat_message(account_id: &str, ea_type: &str) -> HeartbeatMessage {
    HeartbeatMessage {
        message_type: "Heartbeat".to_string(),
        account_id: account_id.to_string(),
        balance: 10000.0,
        equity: 10000.0,
        open_positions: 0,
        timestamp: chrono::Utc::now().to_rfc3339(),
        version: "test".to_string(),
        ea_type: ea_type.to_string(),
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
    let msg = create_test_heartbeat_message("TEST_001", "Master");
    let account_id = msg.account_id.clone();

    // Auto-register via heartbeat
    manager.update_heartbeat(msg).await;

    // Verify registered
    let ea = manager.get_master(&account_id).await;
    assert!(ea.is_some());
    assert_eq!(ea.unwrap().status, ConnectionStatus::Online);

    // Unregister with ea_type
    manager.unregister_ea(&account_id, EaType::Master).await;

    let ea = manager.get_master(&account_id).await;
    assert!(ea.is_some());
    assert_eq!(ea.unwrap().status, ConnectionStatus::Offline);
}

#[tokio::test]
async fn test_update_heartbeat() {
    let manager = ConnectionManager::new(30);
    let account_id = "TEST_001".to_string();

    // First heartbeat: auto-registers the EA
    let hb_msg = create_test_heartbeat_message(&account_id, "Master");
    manager.update_heartbeat(hb_msg).await;

    // Second heartbeat: updates balance and equity
    let mut hb_msg2 = create_test_heartbeat_message(&account_id, "Master");
    hb_msg2.balance = 12000.0;
    hb_msg2.equity = 11500.0;
    manager.update_heartbeat(hb_msg2).await;

    let ea = manager.get_master(&account_id).await;
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
        .update_heartbeat(create_test_heartbeat_message("TEST_001", "Master"))
        .await;
    manager
        .update_heartbeat(create_test_heartbeat_message("TEST_002", "Master"))
        .await;

    let eas = manager.get_all_eas().await;
    assert_eq!(eas.len(), 2);
}

#[tokio::test]
async fn test_timeout_check() {
    let manager = ConnectionManager::new(1); // 1 second timeout
    let msg = create_test_heartbeat_message("TEST_001", "Master");
    let account_id = msg.account_id.clone();

    // Auto-register via heartbeat
    manager.update_heartbeat(msg).await;

    // Verify initially online
    let ea = manager.get_master(&account_id).await;
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
    let ea = manager.get_master(&account_id).await;
    assert_eq!(ea.unwrap().status, ConnectionStatus::Timeout);
}

#[tokio::test]
async fn test_heartbeat_prevents_timeout() {
    let manager = ConnectionManager::new(2); // 2 second timeout
    let msg = create_test_heartbeat_message("TEST_001", "Master");
    let account_id = msg.account_id.clone();

    // Auto-register via heartbeat
    manager.update_heartbeat(msg).await;

    // Send heartbeat after 1 second
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    manager
        .update_heartbeat(create_test_heartbeat_message(&account_id, "Master"))
        .await;

    // Wait another second (total 2 seconds, but heartbeat was sent at 1 second)
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Run timeout check
    let timed_out = manager.check_timeouts().await;

    // Should not have timed out because heartbeat was sent within timeout
    assert_eq!(timed_out.len(), 0);

    // Should still be online
    let ea = manager.get_master(&account_id).await;
    assert_eq!(ea.unwrap().status, ConnectionStatus::Online);
}

#[tokio::test]
async fn test_heartbeat_auto_registration() {
    let manager = ConnectionManager::new(30);

    // Send heartbeat without prior registration
    let mut hb_msg = create_test_heartbeat_message("TEST_NEW", "Slave");
    hb_msg.balance = 15000.0;
    hb_msg.equity = 15500.0;
    hb_msg.platform = "MT5".to_string();
    hb_msg.account_number = 67890;
    hb_msg.broker = "New Broker".to_string();
    hb_msg.currency = "EUR".to_string();
    hb_msg.leverage = 200;

    manager.update_heartbeat(hb_msg).await;

    // Verify EA was auto-registered as Slave
    let ea = manager.get_slave("TEST_NEW").await;
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

    // get_ea should NOT find Slave (Master優先)
    let ea_via_get_ea = manager.get_ea("TEST_NEW").await;
    // Since there's no Master, get_ea should fall back to Slave
    assert!(ea_via_get_ea.is_some());
    assert_eq!(ea_via_get_ea.unwrap().ea_type, EaType::Slave);
}

// ============================================================================
// NEW: Dual EA per account tests
// ============================================================================

#[tokio::test]
async fn test_same_account_master_and_slave() {
    let manager = ConnectionManager::new(30);
    let account_id = "DUAL_TEST";

    // Register Master EA
    manager
        .update_heartbeat(create_test_heartbeat_message(account_id, "Master"))
        .await;

    // Register Slave EA (same account_id)
    let mut slave_hb = create_test_heartbeat_message(account_id, "Slave");
    slave_hb.balance = 20000.0; // Different balance to distinguish
    manager.update_heartbeat(slave_hb).await;

    // Both should be registered
    let all_eas = manager.get_all_eas().await;
    assert_eq!(all_eas.len(), 2, "Both Master and Slave should be registered");

    // get_master returns Master
    let master = manager.get_master(account_id).await;
    assert!(master.is_some());
    assert_eq!(master.as_ref().unwrap().ea_type, EaType::Master);
    assert_eq!(master.as_ref().unwrap().balance, 10000.0);

    // get_slave returns Slave
    let slave = manager.get_slave(account_id).await;
    assert!(slave.is_some());
    assert_eq!(slave.as_ref().unwrap().ea_type, EaType::Slave);
    assert_eq!(slave.as_ref().unwrap().balance, 20000.0);

    // get_ea returns Master (後方互換: Master優先)
    let ea = manager.get_ea(account_id).await;
    assert!(ea.is_some());
    assert_eq!(ea.unwrap().ea_type, EaType::Master);

    // get_eas_by_account returns both
    let eas = manager.get_eas_by_account(account_id).await;
    assert_eq!(eas.len(), 2);
}

#[tokio::test]
async fn test_unregister_one_ea_keeps_other() {
    let manager = ConnectionManager::new(30);
    let account_id = "DUAL_UNREG";

    // Register both Master and Slave
    manager
        .update_heartbeat(create_test_heartbeat_message(account_id, "Master"))
        .await;
    manager
        .update_heartbeat(create_test_heartbeat_message(account_id, "Slave"))
        .await;

    assert_eq!(manager.get_all_eas().await.len(), 2);

    // Unregister only Master
    manager.unregister_ea(account_id, EaType::Master).await;

    // Master should be Offline
    let master = manager.get_master(account_id).await;
    assert_eq!(master.unwrap().status, ConnectionStatus::Offline);

    // Slave should still be Online
    let slave = manager.get_slave(account_id).await;
    assert_eq!(slave.unwrap().status, ConnectionStatus::Online);
}
