//! E2E tests for runtime status updates
//!
//! These tests verify that the RuntimeStatusUpdater correctly transitions Slave runtime_status
//! based on ZeroMQ Heartbeat traffic (Disabled → Enabled → Connected).

use e2e_tests::helpers::default_test_slave_settings;
use e2e_tests::relay_server_process::RelayServerProcess;
use e2e_tests::{
    MasterEaSimulator, SlaveEaSimulator, STATUS_CONNECTED, STATUS_DISABLED, STATUS_ENABLED,
};
use sankey_copier_relay_server::db::Database;
use sankey_copier_relay_server::models::MasterSettings;
use tokio::time::{sleep, Duration};

const SETTLE_WAIT_MS: u64 = 250;

/// Test that slave runtime_status tracks master cluster events
/// Flow: DISABLED → ENABLED (on slave heartbeat) → CONNECTED (on master heartbeat)
#[tokio::test]
async fn test_slave_runtime_status_tracks_master_cluster_events() {
    let server = RelayServerProcess::start().expect("Failed to start server");
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
    let mut slave = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_address(),
        &server.zmq_pub_address(),
        slave_account,
        master_account,
    )
    .expect("Failed to create slave simulator");

    // Enable auto-trading and start OnTimer loop (sends heartbeat automatically)
    slave.set_trade_allowed(true);
    slave.start().expect("slave start should succeed");
    sleep(Duration::from_millis(SETTLE_WAIT_MS)).await;

    // After slave heartbeat - should transition to ENABLED
    assert_runtime_status(&db, master_account, slave_account, STATUS_ENABLED).await;

    // Create master simulator
    let mut master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_address(),
        master_account,
    )
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
        member.runtime_status, expected,
        "Expected runtime_status {} but got {}",
        expected, member.runtime_status
    );
}
