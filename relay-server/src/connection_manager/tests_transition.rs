
use super::*;
use super::tests::create_test_heartbeat_message;

#[tokio::test]
async fn test_register_ea_state_transition() {
    let manager = ConnectionManager::new(30);
    let account_id = "TRANSITION_TEST";

    // 1. Explicit Register
    let register_msg = crate::models::RegisterMessage {
        message_type: "Register".to_string(),
        account_id: account_id.to_string(),
        ea_type: "Master".to_string(),
        platform: "MT5".to_string(),
        account_number: 12345,
        broker: "Test Broker".to_string(),
        account_name: "Test Account".to_string(),
        server: "Test-Server".to_string(),
        currency: "USD".to_string(),
        leverage: 100,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    manager.register_ea(&register_msg).await;

    // Verify status is Registered (NOT Online)
    let ea = manager.get_master(account_id).await;
    assert!(ea.is_some());
    let ea = ea.unwrap();
    assert_eq!(ea.status, ConnectionStatus::Registered);
    assert!(!ea.is_trade_allowed); // Default is false

    // 2. Heartbeat (Transition to Online)
    let hb_msg = create_test_heartbeat_message(account_id, "Master");
    manager.update_heartbeat(hb_msg).await;

    // Verify status is Online
    let ea = manager.get_master(account_id).await;
    assert!(ea.is_some());
    let ea = ea.unwrap();
    assert_eq!(ea.status, ConnectionStatus::Online);
    assert!(ea.is_trade_allowed); // Heartbeat updates this to true
}
