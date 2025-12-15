//! E2E tests for synchronization protocol between Master and Slave EAs
//!
//! These tests verify the PositionSnapshot distribution and SyncRequest routing
//! functionality of the relay server.
//!
//! Categories:
//! 1. PositionSnapshot Distribution - Master sends position snapshots to slaves
//! 2. SyncRequest Routing - Slaves request sync from master
//! 3. Full Sync Cycle - Complete sync flow testing

use e2e_tests::helpers::{default_test_slave_settings, setup_test_scenario};
use e2e_tests::MasterEaSimulator;
use e2e_tests::TestSandbox;
use sankey_copier_relay_server::adapters::outbound::persistence::Database;
use tokio::time::{sleep, Duration};

// =============================================================================
// Category 1: PositionSnapshot Distribution Tests
// =============================================================================

/// Test: Master sends PositionSnapshot → Single Slave receives it
#[tokio::test]
async fn test_position_snapshot_single_slave() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();
    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to database");

    let master_account = "MASTER_SYNC_001";
    let slave_account = "SLAVE_SYNC_001";

    // Setup scenario
    setup_test_scenario(&db, master_account, &[slave_account], |_| {
        default_test_slave_settings()
    })
    .await
    .expect("Failed to setup scenario");

    // Create simulators
    let mut master = sandbox
        .create_master(master_account, true)
        .expect("Failed to create master");

    let mut slave = sandbox
        .create_slave(slave_account, master_account, true)
        .expect("Failed to create slave");

    // Subscribe master to sync requests
    master
        .subscribe_to_sync_requests()
        .expect("Failed to subscribe to sync requests");

    // Start EAs with auto-trading enabled
    master.set_trade_allowed(true);
    master.start().expect("Failed to start master");
    slave.set_trade_allowed(true);
    slave.start().expect("Failed to start slave");

    // Wait for slave to receive config (which triggers sync/ topic subscription)
    slave
        .wait_for_status(sankey_copier_zmq::STATUS_CONNECTED, 10000)
        .expect("Failed to wait for status")
        .expect("Slave should receive CONNECTED status");

    // Give time for ZMQ subscription to become effective
    sleep(Duration::from_millis(100)).await;

    // Master sends PositionSnapshot with test positions
    let positions = vec![
        MasterEaSimulator::create_test_position(1001, "EURUSD", "Buy", 0.5, 1.0850),
        MasterEaSimulator::create_test_position(1002, "GBPUSD", "Sell", 0.3, 1.2650),
    ];

    master
        .send_position_snapshot(positions)
        .expect("Failed to send snapshot");

    // Give time for message routing
    sleep(Duration::from_millis(200)).await;

    // Slave should receive the PositionSnapshot
    let received = slave
        .try_receive_position_snapshot(1000)
        .expect("Failed to receive");

    assert!(received.is_some(), "Slave should receive PositionSnapshot");

    let snapshot = received.unwrap();
    assert_eq!(snapshot.source_account, master_account);
    assert_eq!(snapshot.positions.len(), 2);
    assert_eq!(snapshot.positions[0].ticket, 1001);
    assert_eq!(snapshot.positions[0].symbol, "EURUSD");
    assert_eq!(snapshot.positions[1].ticket, 1002);
    assert_eq!(snapshot.positions[1].symbol, "GBPUSD");
}

/// Test: Master sends PositionSnapshot → Multiple Slaves receive it
#[tokio::test]
async fn test_position_snapshot_multiple_slaves() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();
    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to database");

    let master_account = "MASTER_SYNC_002";
    let slave_accounts = ["SLAVE_SYNC_002A", "SLAVE_SYNC_002B"];

    // Setup scenario
    setup_test_scenario(&db, master_account, &slave_accounts, |_| {
        default_test_slave_settings()
    })
    .await
    .expect("Failed to setup scenario");

    // Create simulators
    let mut master = sandbox
        .create_master(master_account, true)
        .expect("Failed to create master");

    let mut slave1 = sandbox
        .create_slave(slave_accounts[0], master_account, true)
        .expect("Failed to create slave1");

    let mut slave2 = sandbox
        .create_slave(slave_accounts[1], master_account, true)
        .expect("Failed to create slave2");

    // Start EAs with auto-trading enabled
    master.set_trade_allowed(true);
    master.start().expect("Failed to start master");
    slave1.set_trade_allowed(true);
    slave1.start().expect("Failed to start slave1");
    slave2.set_trade_allowed(true);
    slave2.start().expect("Failed to start slave2");

    // Wait for slaves to receive config (which triggers sync/ topic subscription)
    slave1
        .wait_for_status(sankey_copier_zmq::STATUS_CONNECTED, 10000)
        .expect("Failed to wait for status")
        .expect("Slave1 should receive CONNECTED status");
    slave2
        .wait_for_status(sankey_copier_zmq::STATUS_CONNECTED, 10000)
        .expect("Failed to wait for status")
        .expect("Slave2 should receive CONNECTED status");

    // Give time for ZMQ subscription to become effective
    sleep(Duration::from_millis(100)).await;

    // Master sends PositionSnapshot
    let positions = vec![MasterEaSimulator::create_test_position(
        2001, "USDJPY", "Buy", 1.0, 149.50,
    )];

    master
        .send_position_snapshot(positions)
        .expect("Failed to send snapshot");

    // Give time for message routing
    sleep(Duration::from_millis(200)).await;

    // Both slaves should receive the PositionSnapshot
    let received1 = slave1
        .try_receive_position_snapshot(1000)
        .expect("Failed to receive on slave1");

    let received2 = slave2
        .try_receive_position_snapshot(1000)
        .expect("Failed to receive on slave2");

    assert!(
        received1.is_some(),
        "Slave1 should receive PositionSnapshot"
    );
    assert!(
        received2.is_some(),
        "Slave2 should receive PositionSnapshot"
    );

    let snapshot1 = received1.unwrap();
    let snapshot2 = received2.unwrap();

    assert_eq!(snapshot1.source_account, master_account);
    assert_eq!(snapshot2.source_account, master_account);
    assert_eq!(snapshot1.positions.len(), 1);
    assert_eq!(snapshot2.positions.len(), 1);
}

/// Test: Empty PositionSnapshot (Master has no positions)
#[tokio::test]
async fn test_position_snapshot_empty() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();
    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to database");

    let master_account = "MASTER_SYNC_003";
    let slave_account = "SLAVE_SYNC_003";

    // Setup scenario
    setup_test_scenario(&db, master_account, &[slave_account], |_| {
        default_test_slave_settings()
    })
    .await
    .expect("Failed to setup scenario");

    // Create simulators
    let mut master = sandbox
        .create_master(master_account, true)
        .expect("Failed to create master");

    let mut slave = sandbox
        .create_slave(slave_account, master_account, true)
        .expect("Failed to create slave");

    // Subscribe to sync topic early (before start) to avoid slow joiner issues
    slave
        .subscribe_to_sync_topic()
        .expect("Failed to subscribe to sync topic");

    // Start EAs with auto-trading enabled
    master.set_trade_allowed(true);
    master.start().expect("Failed to start master");
    slave.set_trade_allowed(true);
    slave.start().expect("Failed to start slave");

    // Wait for slave to receive config
    slave
        .wait_for_status(sankey_copier_zmq::STATUS_CONNECTED, 10000)
        .expect("Failed to wait for status")
        .expect("Slave should receive CONNECTED status");

    // Give time for ZMQ subscription to become effective
    sleep(Duration::from_millis(100)).await;

    // Master sends empty PositionSnapshot
    master
        .send_position_snapshot(vec![])
        .expect("Failed to send empty snapshot");

    // Give time for message routing
    sleep(Duration::from_millis(200)).await;

    // Slave should receive the empty snapshot
    let received = slave
        .try_receive_position_snapshot(1000)
        .expect("Failed to receive");

    assert!(received.is_some(), "Slave should receive empty snapshot");

    let snapshot = received.unwrap();
    assert_eq!(snapshot.positions.len(), 0);
}

// =============================================================================
// Category 2: SyncRequest Routing Tests
// =============================================================================

/// Test: Slave sends SyncRequest → Master receives it
#[tokio::test]
async fn test_sync_request_to_master() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();
    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to database");

    let master_account = "MASTER_SYNC_004";
    let slave_account = "SLAVE_SYNC_004";

    // Setup scenario
    setup_test_scenario(&db, master_account, &[slave_account], |_| {
        default_test_slave_settings()
    })
    .await
    .expect("Failed to setup scenario");

    // Create simulators
    let mut master = sandbox
        .create_master(master_account, true)
        .expect("Failed to create master");

    let mut slave = sandbox
        .create_slave(slave_account, master_account, true)
        .expect("Failed to create slave");

    // Master subscribes to sync requests
    master
        .subscribe_to_sync_requests()
        .expect("Failed to subscribe");

    // Start EAs with auto-trading enabled
    master.set_trade_allowed(true);
    master.start().expect("Failed to start master");
    slave.set_trade_allowed(true);
    slave.start().expect("Failed to start slave");

    // Wait for master to receive config (to ensure it's properly registered)
    master
        .wait_for_status(sankey_copier_zmq::STATUS_CONNECTED, 10000)
        .expect("Failed to wait for status")
        .expect("Master should receive CONNECTED status");

    // Slave sends SyncRequest
    slave
        .send_sync_request(None)
        .expect("Failed to send SyncRequest");

    // Give time for ZMQ sync/ subscription and message routing
    sleep(Duration::from_millis(200)).await;

    // Master should receive the SyncRequest
    let received = master
        .try_receive_sync_request(1000)
        .expect("Failed to receive");

    assert!(received.is_some(), "Master should receive SyncRequest");

    let request = received.unwrap();
    assert_eq!(request.slave_account, slave_account);
    assert_eq!(request.master_account, master_account);
    assert!(request.last_sync_time.is_none());
}

/// Test: SyncRequest with last_sync_time
#[tokio::test]
async fn test_sync_request_with_last_sync_time() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();
    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to database");

    let master_account = "MASTER_SYNC_005";
    let slave_account = "SLAVE_SYNC_005";

    // Setup scenario
    setup_test_scenario(&db, master_account, &[slave_account], |_| {
        default_test_slave_settings()
    })
    .await
    .expect("Failed to setup scenario");

    // Create simulators
    let mut master = sandbox
        .create_master(master_account, true)
        .expect("Failed to create master");

    let mut slave = sandbox
        .create_slave(slave_account, master_account, true)
        .expect("Failed to create slave");

    // Master subscribes to sync requests
    master
        .subscribe_to_sync_requests()
        .expect("Failed to subscribe");

    // Start EAs with auto-trading enabled
    master.set_trade_allowed(true);
    master.start().expect("Failed to start master");
    slave.set_trade_allowed(true);
    slave.start().expect("Failed to start slave");

    // Wait for master to receive config (to ensure it's properly registered)
    master
        .wait_for_status(sankey_copier_zmq::STATUS_CONNECTED, 10000)
        .expect("Failed to wait for status")
        .expect("Master should receive CONNECTED status");

    // Slave sends SyncRequest with last_sync_time
    let last_sync = chrono::Utc::now().to_rfc3339();
    slave
        .send_sync_request(Some(last_sync.clone()))
        .expect("Failed to send SyncRequest");

    // Give time for ZMQ sync/ subscription and message routing
    sleep(Duration::from_millis(200)).await;

    // Master should receive the SyncRequest
    let received = master
        .try_receive_sync_request(1000)
        .expect("Failed to receive");

    assert!(received.is_some(), "Master should receive SyncRequest");

    let request = received.unwrap();
    assert!(request.last_sync_time.is_some());
    assert_eq!(request.last_sync_time.unwrap(), last_sync);
}

/// Test: SyncRequest from non-member slave should be rejected
#[tokio::test]
async fn test_sync_request_non_member_rejected() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();
    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to database");

    let master_account = "MASTER_SYNC_006";
    let slave_account = "SLAVE_SYNC_006";
    let non_member_slave = "NON_MEMBER_SLAVE";

    // Setup scenario - only add slave_account as member
    setup_test_scenario(&db, master_account, &[slave_account], |_| {
        default_test_slave_settings()
    })
    .await
    .expect("Failed to setup scenario");

    // Create simulators
    let mut master = sandbox
        .create_master(master_account, true)
        .expect("Failed to create master");

    // Create non-member slave (not registered in trade group)
    let mut non_member = sandbox
        .create_slave(non_member_slave, master_account, true)
        .expect("Failed to create non-member slave");

    // Master subscribes to sync requests
    master
        .subscribe_to_sync_requests()
        .expect("Failed to subscribe");

    // Start EAs with auto-trading enabled
    master.set_trade_allowed(true);
    master.start().expect("Failed to start master");
    non_member.set_trade_allowed(true);
    non_member.start().expect("Failed to start non_member");
    sleep(Duration::from_millis(2000)).await;

    // Non-member sends SyncRequest
    non_member
        .send_sync_request(None)
        .expect("Failed to send SyncRequest");

    // Give time for message processing
    sleep(Duration::from_millis(200)).await;

    // Master should NOT receive the SyncRequest (rejected by relay)
    let received = master
        .try_receive_sync_request(500)
        .expect("Failed to receive");

    assert!(
        received.is_none(),
        "Non-member SyncRequest should be rejected"
    );
}

// =============================================================================
// Category 3: Full Sync Cycle Tests
// =============================================================================

/// Test: Full sync cycle - Slave requests → Master responds with snapshot
#[tokio::test]
async fn test_full_sync_cycle() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();
    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to database");

    let master_account = "MASTER_SYNC_007";
    let slave_account = "SLAVE_SYNC_007";

    // Setup scenario
    setup_test_scenario(&db, master_account, &[slave_account], |_| {
        default_test_slave_settings()
    })
    .await
    .expect("Failed to setup scenario");

    // Create simulators
    let mut master = sandbox
        .create_master(master_account, true)
        .expect("Failed to create master");

    let mut slave = sandbox
        .create_slave(slave_account, master_account, true)
        .expect("Failed to create slave");

    // Setup subscriptions
    master
        .subscribe_to_sync_requests()
        .expect("Failed to subscribe");

    // Start EAs with auto-trading enabled
    master.set_trade_allowed(true);
    master.start().expect("Failed to start master");
    slave.set_trade_allowed(true);
    slave.start().expect("Failed to start slave");

    // Wait for both to receive config (ensures master is registered and slave has sync/ subscription)
    master
        .wait_for_status(sankey_copier_zmq::STATUS_CONNECTED, 10000)
        .expect("Failed to wait for status")
        .expect("Master should receive CONNECTED status");
    slave
        .wait_for_status(sankey_copier_zmq::STATUS_CONNECTED, 10000)
        .expect("Failed to wait for status")
        .expect("Slave should receive CONNECTED status");

    // Give time for ZMQ subscription to become effective
    sleep(Duration::from_millis(100)).await;

    // Step 1: Slave sends SyncRequest
    slave
        .send_sync_request(None)
        .expect("Failed to send SyncRequest");

    // Give time for message routing
    sleep(Duration::from_millis(200)).await;

    // Step 2: Master receives SyncRequest
    let request = master
        .try_receive_sync_request(1000)
        .expect("Failed to receive SyncRequest")
        .expect("Should receive SyncRequest");

    assert_eq!(request.slave_account, slave_account);

    // Step 3: Master responds with PositionSnapshot
    let positions = vec![
        MasterEaSimulator::create_test_position(7001, "EURUSD", "Buy", 0.5, 1.0850),
        MasterEaSimulator::create_test_position(7002, "AUDUSD", "Sell", 0.2, 0.6520),
    ];

    master
        .send_position_snapshot(positions)
        .expect("Failed to send snapshot");

    // Give time for message routing
    sleep(Duration::from_millis(200)).await;

    // Step 4: Slave receives PositionSnapshot
    let snapshot = slave
        .try_receive_position_snapshot(1000)
        .expect("Failed to receive snapshot")
        .expect("Should receive PositionSnapshot");

    assert_eq!(snapshot.source_account, master_account);
    assert_eq!(snapshot.positions.len(), 2);

    // Verify position details
    assert_eq!(snapshot.positions[0].ticket, 7001);
    assert_eq!(snapshot.positions[0].symbol, "EURUSD");
    assert_eq!(snapshot.positions[0].order_type, "Buy");
    assert!((snapshot.positions[0].lots - 0.5).abs() < 0.0001);

    assert_eq!(snapshot.positions[1].ticket, 7002);
    assert_eq!(snapshot.positions[1].symbol, "AUDUSD");
    assert_eq!(snapshot.positions[1].order_type, "Sell");
}
