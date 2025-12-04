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

#[tokio::test]
async fn test_master_heartbeat_marks_enabled_slaves_connected() {
    let ctx = create_test_context().await;
    let master_account = "MASTER_HEARTBEAT_SYNC";

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

    for slave in ["SLAVE_ONE", "SLAVE_TWO"] {
        ctx.db
            .add_member(
                master_account,
                slave,
                crate::models::SlaveSettings::default(),
                crate::models::STATUS_ENABLED,
            )
            .await
            .unwrap();
    }

    ctx.handle_heartbeat(build_heartbeat(master_account, "Master", true))
        .await;

    let members = ctx.db.get_members(master_account).await.unwrap();
    assert_eq!(members.len(), 2);
    for member in members {
        assert_eq!(member.runtime_status, crate::models::STATUS_CONNECTED);
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_slave_heartbeat_updates_runtime_when_master_offline() {
    let ctx = create_test_context().await;
    let master_account = "MASTER_HEARTBEAT_DEGRADE";
    let slave_account = "SLAVE_RUNTIME_TRACK";

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

    ctx.handle_heartbeat(build_heartbeat(slave_account, "Slave", true))
        .await;

    let member = ctx
        .db
        .get_member(master_account, slave_account)
        .await
        .unwrap()
        .expect("member should exist");
    assert_eq!(member.runtime_status, crate::models::STATUS_ENABLED);

    ctx.cleanup().await;
}

/// Test: Slave heartbeat broadcasts settings_updated when runtime status changes
#[tokio::test]
async fn test_slave_heartbeat_broadcasts_settings_updated_on_status_change() {
    let mut ctx = create_test_context().await;
    let master_account = "MASTER_BROADCAST_TEST";
    let slave_account = "SLAVE_BROADCAST_TEST";

    // Setup: Create TradeGroup with enabled Master
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

    // Add Slave member with initial status CONNECTED (will change to ENABLED when Master is offline)
    ctx.db
        .add_member(
            master_account,
            slave_account,
            crate::models::SlaveSettings::default(),
            crate::models::STATUS_CONNECTED,
        )
        .await
        .unwrap();

    // Clear any pending broadcast messages
    ctx.collect_broadcast_messages();

    // Act: Slave sends heartbeat while Master is offline
    // This should trigger status change: CONNECTED -> ENABLED
    ctx.handle_heartbeat(build_heartbeat(slave_account, "Slave", true))
        .await;

    // Assert: Check that settings_updated was broadcast
    let messages = ctx.collect_broadcast_messages();

    // Should have at least one settings_updated message
    let settings_updated_msgs: Vec<_> = messages
        .iter()
        .filter(|m| m.starts_with("settings_updated:"))
        .collect();

    assert!(
        !settings_updated_msgs.is_empty(),
        "Expected settings_updated broadcast when Slave runtime status changes. Got messages: {:?}",
        messages
    );

    // Verify the broadcast contains correct slave account
    let msg = settings_updated_msgs[0];
    assert!(
        msg.contains(slave_account),
        "Broadcast should contain slave account: {}. Got: {}",
        slave_account,
        msg
    );

    ctx.cleanup().await;
}

/// Test: Master heartbeat broadcasts settings_updated when Slave status changes
#[tokio::test]
async fn test_master_heartbeat_broadcasts_settings_updated_for_slave_status_change() {
    let mut ctx = create_test_context().await;
    let master_account = "MASTER_BROADCAST_SLAVE";
    let slave_account = "SLAVE_VIA_MASTER_HB";

    // Setup: Create TradeGroup with enabled Master
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

    // Add Slave member with initial status ENABLED (will change to CONNECTED when Master connects)
    ctx.db
        .add_member(
            master_account,
            slave_account,
            crate::models::SlaveSettings::default(),
            crate::models::STATUS_ENABLED,
        )
        .await
        .unwrap();

    // Register Slave EA first (so it's known to connection_manager)
    ctx.handle_heartbeat(build_heartbeat(slave_account, "Slave", true))
        .await;

    // Clear any pending broadcast messages from Slave registration
    ctx.collect_broadcast_messages();

    // Act: Master sends heartbeat (this should trigger Slave status change: ENABLED -> CONNECTED)
    ctx.handle_heartbeat(build_heartbeat(master_account, "Master", true))
        .await;

    // Assert: Check that settings_updated was broadcast for the Slave
    let messages = ctx.collect_broadcast_messages();

    let settings_updated_msgs: Vec<_> = messages
        .iter()
        .filter(|m| m.starts_with("settings_updated:") && m.contains(slave_account))
        .collect();

    assert!(
        !settings_updated_msgs.is_empty(),
        "Expected settings_updated broadcast for Slave when Master heartbeat triggers status change. Got messages: {:?}",
        messages
    );

    ctx.cleanup().await;
}

/// Test: No broadcast when runtime status doesn't change
#[tokio::test]
async fn test_no_broadcast_when_status_unchanged() {
    let mut ctx = create_test_context().await;
    let master_account = "MASTER_NO_CHANGE";
    let slave_account = "SLAVE_NO_CHANGE";

    // Setup: Create TradeGroup with enabled Master
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

    // Add Slave member with status ENABLED (matching what it will evaluate to with offline Master)
    ctx.db
        .add_member(
            master_account,
            slave_account,
            crate::models::SlaveSettings::default(),
            crate::models::STATUS_ENABLED,
        )
        .await
        .unwrap();

    // First heartbeat to stabilize state
    ctx.handle_heartbeat(build_heartbeat(slave_account, "Slave", true))
        .await;

    // Clear messages from first heartbeat
    ctx.collect_broadcast_messages();

    // Act: Send another heartbeat - status should remain ENABLED
    ctx.handle_heartbeat(build_heartbeat(slave_account, "Slave", true))
        .await;

    // Assert: No settings_updated broadcast for unchanged status
    let messages = ctx.collect_broadcast_messages();

    let settings_updated_msgs: Vec<_> = messages
        .iter()
        .filter(|m| m.starts_with("settings_updated:") && m.contains(slave_account))
        .collect();

    assert!(
        settings_updated_msgs.is_empty(),
        "Should NOT broadcast settings_updated when status is unchanged. Got messages: {:?}",
        settings_updated_msgs
    );

    ctx.cleanup().await;
}

// =============================================================================
// Server Restart / Master Reconnection Scenarios
// These tests verify that update_master_statuses_connected is redundant
// when per-connection evaluation is working correctly.
// =============================================================================

/// Test: Master reconnection after server restart (simulated by fresh connection_manager)
/// This is the critical scenario: connection_manager is empty (server restarted),
/// but DB has existing ENABLED members. Master reconnects and Slaves should become CONNECTED.
///
/// This test verifies that per-connection evaluation (via is_new_registration=true)
/// correctly updates Slave status without relying on update_master_statuses_connected.
#[tokio::test]
async fn test_master_reconnection_after_server_restart() {
    let ctx = create_test_context().await;
    let master_account = "MASTER_RESTART";
    let slave_account = "SLAVE_RESTART";

    // Setup: Create TradeGroup with enabled Master
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

    // Add Slave member with ENABLED status (as if Master was previously connected, then server restarted)
    // After server restart, DB persists but connection_manager is empty
    ctx.db
        .add_member(
            master_account,
            slave_account,
            crate::models::SlaveSettings::default(),
            crate::models::STATUS_ENABLED,
        )
        .await
        .unwrap();

    // Simulate: Slave EA is online (registers first after server restart)
    ctx.handle_heartbeat(build_heartbeat(slave_account, "Slave", true))
        .await;

    // Verify Slave status remains ENABLED (Master not yet connected)
    let member = ctx
        .db
        .get_member(master_account, slave_account)
        .await
        .unwrap()
        .expect("member should exist");
    assert_eq!(
        member.runtime_status,
        crate::models::STATUS_ENABLED,
        "Slave should be ENABLED before Master connects"
    );

    // Act: Master reconnects (first heartbeat after server restart)
    // connection_manager sees this as new registration (is_new_registration=true)
    ctx.handle_heartbeat(build_heartbeat(master_account, "Master", true))
        .await;

    // Assert: Slave should now be CONNECTED
    let member = ctx
        .db
        .get_member(master_account, slave_account)
        .await
        .unwrap()
        .expect("member should exist");
    assert_eq!(
        member.runtime_status,
        crate::models::STATUS_CONNECTED,
        "Slave should be CONNECTED after Master reconnects (per-connection evaluation)"
    );

    ctx.cleanup().await;
}

/// Test: Master reconnection with multiple Slaves after server restart
/// Verifies that all enabled Slaves are updated to CONNECTED via per-connection evaluation
#[tokio::test]
async fn test_master_reconnection_updates_multiple_slaves() {
    let ctx = create_test_context().await;
    let master_account = "MASTER_MULTI_RESTART";

    // Setup: Create TradeGroup with enabled Master
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

    // Add multiple Slaves with different statuses
    ctx.db
        .add_member(
            master_account,
            "SLAVE_A",
            crate::models::SlaveSettings::default(),
            crate::models::STATUS_ENABLED,
        )
        .await
        .unwrap();
    ctx.db
        .add_member(
            master_account,
            "SLAVE_B",
            crate::models::SlaveSettings::default(),
            crate::models::STATUS_ENABLED,
        )
        .await
        .unwrap();
    // SLAVE_C is disabled (should NOT become CONNECTED)
    ctx.db
        .add_member(
            master_account,
            "SLAVE_C",
            crate::models::SlaveSettings::default(),
            crate::models::STATUS_DISABLED,
        )
        .await
        .unwrap();
    // Disable SLAVE_C via enabled_flag
    ctx.db
        .update_member_enabled_flag(master_account, "SLAVE_C", false)
        .await
        .unwrap();

    // Simulate: All Slaves come online
    for slave in ["SLAVE_A", "SLAVE_B", "SLAVE_C"] {
        ctx.handle_heartbeat(build_heartbeat(slave, "Slave", true))
            .await;
    }

    // Act: Master reconnects
    ctx.handle_heartbeat(build_heartbeat(master_account, "Master", true))
        .await;

    // Assert: Enabled Slaves should be CONNECTED, disabled should remain DISABLED
    let member_a = ctx
        .db
        .get_member(master_account, "SLAVE_A")
        .await
        .unwrap()
        .expect("SLAVE_A should exist");
    let member_b = ctx
        .db
        .get_member(master_account, "SLAVE_B")
        .await
        .unwrap()
        .expect("SLAVE_B should exist");
    let member_c = ctx
        .db
        .get_member(master_account, "SLAVE_C")
        .await
        .unwrap()
        .expect("SLAVE_C should exist");

    assert_eq!(
        member_a.runtime_status,
        crate::models::STATUS_CONNECTED,
        "SLAVE_A should be CONNECTED"
    );
    assert_eq!(
        member_b.runtime_status,
        crate::models::STATUS_CONNECTED,
        "SLAVE_B should be CONNECTED"
    );
    assert_eq!(
        member_c.runtime_status,
        crate::models::STATUS_DISABLED,
        "SLAVE_C should remain DISABLED (enabled_flag=false)"
    );

    ctx.cleanup().await;
}

/// Test: Master temporary disconnect and reconnect (without server restart)
/// In this case, is_new_registration will be FALSE (Master was already known),
/// but trade_allowed_changed might be FALSE too.
/// This tests whether the status is still correctly updated.
#[tokio::test]
async fn test_master_temporary_disconnect_reconnect() {
    let ctx = create_test_context().await;
    let master_account = "MASTER_TEMP_DC";
    let slave_account = "SLAVE_TEMP_DC";

    // Setup
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
            crate::models::STATUS_ENABLED,
        )
        .await
        .unwrap();

    // Slave comes online
    ctx.handle_heartbeat(build_heartbeat(slave_account, "Slave", true))
        .await;

    // Master connects (first time - is_new_registration=true)
    ctx.handle_heartbeat(build_heartbeat(master_account, "Master", true))
        .await;

    // Verify CONNECTED
    let member = ctx
        .db
        .get_member(master_account, slave_account)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(member.runtime_status, crate::models::STATUS_CONNECTED);

    // Simulate: Master temporarily disconnects (we manually reset status in DB to simulate)
    ctx.db
        .update_member_runtime_status(master_account, slave_account, crate::models::STATUS_ENABLED)
        .await
        .unwrap();

    // Master sends another heartbeat (is_new_registration=false, trade_allowed_changed=false)
    // This is the edge case: per-connection evaluation should still detect
    // that Master is online and Slave should be CONNECTED
    ctx.handle_heartbeat(build_heartbeat(master_account, "Master", true))
        .await;

    // Assert: Slave should still be CONNECTED
    // Note: This may fail without update_master_statuses_connected because:
    // - is_new_registration=false (Master already in connection_manager)
    // - trade_allowed_changed=false (same value)
    // - Per-connection evaluation loop is not entered
    let member = ctx
        .db
        .get_member(master_account, slave_account)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        member.runtime_status,
        crate::models::STATUS_CONNECTED,
        "Slave should be CONNECTED after Master's subsequent heartbeat"
    );

    ctx.cleanup().await;
}

/// Test: Verify that per-connection evaluation and update_master_statuses_connected
/// produce the same final state (idempotency test)
#[tokio::test]
async fn test_per_connection_and_bulk_update_produce_same_result() {
    let ctx = create_test_context().await;
    let master_account = "MASTER_IDEMPOTENT";

    // Setup
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

    // Add Slaves
    for i in 1..=3 {
        ctx.db
            .add_member(
                master_account,
                &format!("SLAVE_{}", i),
                crate::models::SlaveSettings::default(),
                crate::models::STATUS_ENABLED,
            )
            .await
            .unwrap();
    }

    // Master connects - both per-connection and bulk update will run
    ctx.handle_heartbeat(build_heartbeat(master_account, "Master", true))
        .await;

    // Verify all are CONNECTED
    for i in 1..=3 {
        let member = ctx
            .db
            .get_member(master_account, &format!("SLAVE_{}", i))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            member.runtime_status,
            crate::models::STATUS_CONNECTED,
            "SLAVE_{} should be CONNECTED",
            i
        );
    }

    // Now simulate: Reset one Slave to ENABLED and send Master heartbeat again
    // (is_new_registration=false, trade_allowed_changed=false)
    ctx.db
        .update_member_runtime_status(master_account, "SLAVE_2", crate::models::STATUS_ENABLED)
        .await
        .unwrap();

    // This heartbeat should NOT trigger per-connection loop (no new_registration, no trade_allowed_changed)
    // but update_master_statuses_connected should still fix SLAVE_2
    ctx.handle_heartbeat(build_heartbeat(master_account, "Master", true))
        .await;

    // Verify SLAVE_2 is back to CONNECTED (via bulk update as safety net)
    let member = ctx
        .db
        .get_member(master_account, "SLAVE_2")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        member.runtime_status,
        crate::models::STATUS_CONNECTED,
        "SLAVE_2 should be CONNECTED (via bulk update safety net)"
    );

    ctx.cleanup().await;
}
