// e2e-tests/tests/smoke_test.rs
//
// Basic smoke tests to verify the E2E test infrastructure works correctly.
// These tests start the real relay-server binary and perform basic operations.

use e2e_tests::TestSandbox;
use e2e_tests::STATUS_CONNECTED;
use std::time::Duration;
use tokio::time::sleep;

/// Test that the relay-server process can be started and stopped via Sandbox
#[tokio::test]
async fn test_server_starts_and_stops() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();

    println!("Relay server started:");
    println!("  ZMQ PULL: {}", server.zmq_pull_address());
    println!("  ZMQ PUB:  {}", server.zmq_pub_address());
    println!("  DB Path:  {}", server.db_path().display());

    assert!(server.zmq_pull_port > 0);
    assert!(server.zmq_pub_port > 0);

    // Give server time to fully initialize
    sleep(Duration::from_secs(1)).await;

    // Server will be shutdown on drop
    println!("Server shutdown successful");
}

/// Test that Master EA can connect and send heartbeat
#[tokio::test]
async fn test_master_ea_connection() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");

    // Give server time to initialize
    sleep(Duration::from_millis(500)).await;

    // Create and connect Master EA simulator
    let mut master = sandbox.create_master("master-smoke-test")
        .expect("Failed to create master simulator");

    // Enable auto-trading and start OnTimer loop (sends heartbeat automatically)
    master.set_trade_allowed(true);
    master.start().expect("Failed to start master");

    println!("Master EA connected and started (heartbeat sent automatically)");

    // Give server time to process
    sleep(Duration::from_millis(500)).await;
}

/// Test that Slave EA can connect and subscribe
#[tokio::test]
async fn test_slave_ea_connection() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");

    // Give server time to initialize
    sleep(Duration::from_millis(500)).await;

    // Create and connect Slave EA simulator
    let mut slave = sandbox.create_slave(
        "slave-smoke-test",
        "master-smoke-test", // master_account to subscribe to
    )
    .expect("Failed to create slave simulator");

    // Enable auto-trading and start OnTimer loop (sends heartbeat automatically)
    slave.set_trade_allowed(true);
    slave.start().expect("Failed to start slave");

    println!("Slave EA connected and started (heartbeat sent automatically)");

    // Give server time to process
    sleep(Duration::from_millis(500)).await;
}

/// Test Master-Slave basic communication
#[tokio::test]
async fn test_master_slave_basic_communication() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    sleep(Duration::from_millis(2000)).await;

    // Create Master
    let mut master = sandbox.create_master("master-comm-test")
        .expect("Failed to create master");

    // Create Slave subscribed to the master
    let mut slave = sandbox.create_slave(
        "slave-comm-test",
        "master-comm-test", // subscribe to master
    )
    .expect("Failed to create slave");

    // Enable auto-trading and start both simulators
    master.set_trade_allowed(true);
    master.start().expect("Master start failed");
    slave.set_trade_allowed(true);
    slave.start().expect("Slave start failed");

    // Wait for slave to connect and receive config
    slave
        .wait_for_status(STATUS_CONNECTED, 5000)
        .expect("Slave failed to connect");

    println!("Master and Slave both connected and running");
}
