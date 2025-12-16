//! E2E tests for runtime status updates
//!
//! These tests verify that the RuntimeStatusUpdater correctly transitions Slave runtime_status
//! based on ZeroMQ Heartbeat traffic (Disabled → Enabled → Connected).

use e2e_tests::helpers::default_test_slave_settings;
use e2e_tests::TestSandbox;
use e2e_tests::{STATUS_CONNECTED, STATUS_DISABLED, STATUS_ENABLED};
use sankey_copier_relay_server::adapters::outbound::persistence::Database;
use sankey_copier_relay_server::domain::models::MasterSettings;
use tokio::time::{sleep, Duration};

const SETTLE_WAIT_MS: u64 = 2000;

/// Test that slave runtime_status tracks master cluster events
/// Flow: DISABLED → ENABLED (on slave heartbeat) → CONNECTED (on master heartbeat)
#[tokio::test]
async fn test_slave_runtime_status_tracks_master_cluster_events() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();
    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to database");

    let master_account = "RUNTIME_MASTER_001";
    let slave_account = "RUNTIME_SLAVE_001";

    // Seed trade group with initial DISABLED status
    seed_trade_group(&db, master_account, slave_account)
        .await
        .expect("failed to seed trade group");

    // Verify initial status is DISABLED
    assert_runtime_status(&db, master_account, slave_account, STATUS_DISABLED).await;

    // Create slave simulator
    let mut slave = sandbox
        .create_slave(slave_account, master_account, true)
        .expect("Failed to create slave simulator");

    // Enable auto-trading and start OnTimer loop (sends heartbeat automatically)
    slave.set_trade_allowed(true);
    slave.start().expect("slave start should succeed");
    sleep(Duration::from_millis(2000)).await;

    // After slave heartbeat - should transition to ENABLED
    assert_runtime_status(&db, master_account, slave_account, STATUS_ENABLED).await;

    // Create master simulator
    let mut master = sandbox
        .create_master(master_account, true)
        .expect("Failed to create master simulator");

    // Enable auto-trading and start OnTimer loop (sends heartbeat automatically)
    master.set_trade_allowed(true);
    master.start().expect("master start should succeed");
    sleep(Duration::from_millis(SETTLE_WAIT_MS)).await;

    // After master heartbeat - should transition to CONNECTED
    assert_runtime_status(&db, master_account, slave_account, STATUS_CONNECTED).await;

    println!("✅ Runtime status transition test passed");
    println!("   DISABLED → ENABLED (slave heartbeat) → CONNECTED (master heartbeat)");
}

/// Seed trade group with master and slave
async fn seed_trade_group(
    db: &Database,
    master_account: &str,
    slave_account: &str,
) -> anyhow::Result<()> {
    // Create trade group for master
    db.create_trade_group(master_account).await?;

    // Enable master
    let master_settings = MasterSettings {
        enabled: true,
        config_version: 1,
        ..Default::default()
    };
    db.update_master_settings(master_account, master_settings)
        .await?;

    // Add slave member with DISABLED status
    let slave_settings = default_test_slave_settings();
    db.add_member(
        master_account,
        slave_account,
        slave_settings,
        STATUS_DISABLED,
    )
    .await?;

    // Enable the member (enabled_flag) but keep runtime_status as DISABLED
    db.update_member_enabled_flag(master_account, slave_account, true)
        .await?;
    db.update_member_runtime_status(master_account, slave_account, STATUS_DISABLED)
        .await?;

    Ok(())
}

/// Assert that the member has the expected runtime status
async fn assert_runtime_status(
    db: &Database,
    master_account: &str,
    slave_account: &str,
    expected: i32,
) {
    let member = db
        .get_member(master_account, slave_account)
        .await
        .expect("DB query should succeed")
        .expect("member should exist");
    assert_eq!(
        member.status, expected,
        "Expected status {} but got {}",
        expected, member.status
    );
}

// =============================================================================
// Dual EA Per Account Tests (Same account_id for Master and Slave)
// =============================================================================

/// Test that Master and Slave EAs with the same account_id can both be registered
/// This verifies the ConnectionManager composite key (account_id, ea_type) works correctly
#[tokio::test]
async fn test_same_account_id_master_and_slave_both_registered() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();
    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to database");

    // Use the SAME account_id for both Master and Slave
    let shared_account = "DUAL_EA_SAME_001";
    let other_slave = "DUAL_EA_SLAVE_ONLY_001";

    // Setup: shared_account is a Master with another slave
    // Also, shared_account can connect as a Slave to a different Master (not in this test)
    db.create_trade_group(shared_account)
        .await
        .expect("Failed to create trade group");

    let master_settings = MasterSettings {
        enabled: true,
        config_version: 1,
        ..Default::default()
    };
    db.update_master_settings(shared_account, master_settings)
        .await
        .expect("Failed to update master settings");

    // Add another slave to this master
    db.add_member(
        shared_account,
        other_slave,
        default_test_slave_settings(),
        STATUS_DISABLED,
    )
    .await
    .expect("Failed to add member");
    db.update_member_enabled_flag(shared_account, other_slave, true)
        .await
        .expect("Failed to enable member");

    // Create Master EA simulator (with shared_account as account_id)
    let mut master_ea = sandbox
        .create_master(shared_account, true)
        .expect("Failed to create Master simulator");

    // Create Slave EA simulator (with shared_account as account_id, connected to a different master)
    // For this test, we'll simulate a Slave with the same account_id
    // In real scenario, this would be the same MT account running both Master and Slave EAs
    let mut slave_ea = sandbox
        .create_slave(shared_account, "SOME_OTHER_MASTER", true)
        .expect("Failed to create Slave simulator");

    // Allow ZMQ connections to stabilize (avoid "slow joiner" problem)
    sleep(Duration::from_millis(500)).await;

    // Start Master EA first
    master_ea.set_trade_allowed(true);
    master_ea.start().expect("Master should start");

    // Wait for Master heartbeat to be processed (Master OnTimer runs every 100ms)
    sleep(Duration::from_millis(1000)).await;

    // Then start Slave EA
    slave_ea.set_trade_allowed(true);
    slave_ea.start().expect("Slave should start");

    // Wait for Slave heartbeat to be processed
    sleep(Duration::from_millis(1000)).await;

    // Verify via API: GET /api/connections should show both EAs
    // Note: Test environment uses self-signed certificates
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .expect("Failed to create HTTP client");
    let response = client
        .get(format!("{}/api/connections", server.http_base_url()))
        .send()
        .await
        .expect("HTTP request should succeed");

    assert!(response.status().is_success(), "API should return success");

    let connections: Vec<serde_json::Value> =
        response.json().await.expect("Should parse JSON response");

    // Filter connections for shared_account
    let shared_connections: Vec<_> = connections
        .iter()
        .filter(|c| c["account_id"].as_str() == Some(shared_account))
        .collect();

    assert_eq!(
        shared_connections.len(),
        2,
        "Should have 2 connections for the same account_id (Master and Slave). Got {} connections. All connections: {:?}",
        shared_connections.len(),
        connections.iter().map(|c| format!("{}:{}", c["account_id"].as_str().unwrap_or("?"), c["ea_type"].as_str().unwrap_or("?"))).collect::<Vec<_>>()
    );

    // Verify both EA types are present
    let ea_types: Vec<_> = shared_connections
        .iter()
        .filter_map(|c| c["ea_type"].as_str())
        .collect();

    assert!(
        ea_types.contains(&"Master"),
        "Should have Master EA for shared account"
    );
    assert!(
        ea_types.contains(&"Slave"),
        "Should have Slave EA for shared account"
    );

    println!("✅ Same account_id Master and Slave both registered test passed");
    println!("   account_id: {}", shared_account);
    println!("   EA types: {:?}", ea_types);
}
