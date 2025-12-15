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
        timestamp: chrono::Utc::now().timestamp_millis(),
        ea_type: Some("Master".to_string()),
    })
    .await;

    // Verify EA status is Offline
    let ea = ctx.connection_manager.get_master(&account_id).await;
    assert!(ea.is_some());
    assert_eq!(ea.unwrap().status, crate::models::ConnectionStatus::Offline);

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_master_unregister_updates_slave_runtime_status() {
    let ctx = create_test_context().await;
    let master_account = "MASTER_UNREGISTER_TRIGGER";
    let slave_account = "SLAVE_RUNTIME_SYNC";

    ctx.db.create_trade_group(master_account).await.unwrap();
    ctx.db
        .update_master_settings(
            master_account,
            crate::models::MasterSettings {
                enabled: true,
                config_version: 1,
                ..crate::models::MasterSettings::default()
            },
        )
        .await
        .unwrap();

    ctx.db
        .add_member(
            master_account,
            slave_account,
            crate::models::SlaveSettings::default(),
            crate::models::STATUS_CONNECTED,
        )
        .await
        .unwrap();

    ctx.handle_heartbeat(build_heartbeat(master_account, "Master", true))
        .await;
    ctx.handle_heartbeat(build_heartbeat(slave_account, "Slave", true))
        .await;

    ctx.handle_unregister(UnregisterMessage {
        message_type: "Unregister".to_string(),
        account_id: master_account.to_string(),
        timestamp: chrono::Utc::now().timestamp_millis(),
        ea_type: Some("Master".to_string()),
    })
    .await;

    let member = ctx
        .db
        .get_member(master_account, slave_account)
        .await
        .unwrap()
        .expect("member should exist");
    assert_eq!(member.status, crate::models::STATUS_ENABLED);

    let master_conn = ctx
        .connection_manager
        .get_master(master_account)
        .await
        .expect("master should remain tracked");
    assert_eq!(master_conn.status, crate::models::ConnectionStatus::Offline);

    ctx.cleanup().await;
}
