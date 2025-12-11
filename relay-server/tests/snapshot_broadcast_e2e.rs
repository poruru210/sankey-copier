// relay-server/tests/snapshot_broadcast_e2e.rs
//
// E2E tests for WebSocket snapshot broadcast functionality.
// Verifies that connection state changes are correctly reflected in snapshots.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

use sankey_copier_relay_server::api::SnapshotBroadcaster;
use sankey_copier_relay_server::connection_manager::ConnectionManager;
use sankey_copier_relay_server::models::{EaConnection, HeartbeatMessage};

/// Create a test HeartbeatMessage with configurable parameters
fn create_heartbeat(
    account_id: &str,
    ea_type: &str,
    balance: f64,
    equity: f64,
    is_trade_allowed: bool,
) -> HeartbeatMessage {
    HeartbeatMessage {
        message_type: "Heartbeat".to_string(),
        account_id: account_id.to_string(),
        balance,
        equity,
        open_positions: 0,
        timestamp: chrono::Utc::now().to_rfc3339(),
        version: "1.0.0".to_string(),
        ea_type: ea_type.to_string(),
        platform: "MT5".to_string(),
        account_number: 12345,
        broker: "Test Broker".to_string(),
        account_name: "Test Account".to_string(),
        server: "Test-Server".to_string(),
        currency: "USD".to_string(),
        leverage: 100,
        is_trade_allowed,
        symbol_prefix: None,
        symbol_suffix: None,
        symbol_map: None,
    }
}

/// Parse connections from a snapshot message
fn parse_snapshot(message: &str) -> Vec<EaConnection> {
    let json_part = message
        .strip_prefix("connections_snapshot:")
        .expect("Message should start with connections_snapshot:");
    serde_json::from_str(json_part).expect("Should parse as EaConnection array")
}

/// Wait for the next snapshot and return it
async fn wait_for_snapshot(rx: &mut broadcast::Receiver<String>) -> Vec<EaConnection> {
    let result = tokio::time::timeout(Duration::from_secs(5), rx.recv()).await;
    let message = result
        .expect("Should receive snapshot within timeout")
        .expect("Should receive valid message");
    parse_snapshot(&message)
}

/// E2E Test: Master EA appears in snapshot when it starts sending heartbeats
#[tokio::test]
async fn test_master_startup_appears_in_snapshot() {
    let (tx, mut rx) = broadcast::channel(100);
    let connection_manager = Arc::new(ConnectionManager::new(30));
    let broadcaster = SnapshotBroadcaster::new(tx, connection_manager.clone());

    // Start subscriber
    broadcaster.on_connect().await;

    // Initially no connections - drain initial snapshot if any
    tokio::time::sleep(Duration::from_millis(100)).await;
    while rx.try_recv().is_ok() {}

    // Master EA starts (sends heartbeat)
    let heartbeat = create_heartbeat("MASTER_001", "Master", 10000.0, 10000.0, true);
    connection_manager.update_heartbeat(heartbeat).await;

    // Wait for next snapshot
    let connections = wait_for_snapshot(&mut rx).await;

    // Verify Master appears in snapshot
    assert_eq!(connections.len(), 1);
    assert_eq!(connections[0].account_id, "MASTER_001");
    assert_eq!(connections[0].ea_type.to_string(), "Master");
    assert_eq!(connections[0].balance, 10000.0);
    assert!(connections[0].is_trade_allowed);

    broadcaster.on_disconnect().await;
}

/// E2E Test: Slave EA appears in snapshot when it starts
#[tokio::test]
async fn test_slave_startup_appears_in_snapshot() {
    let (tx, mut rx) = broadcast::channel(100);
    let connection_manager = Arc::new(ConnectionManager::new(30));
    let broadcaster = SnapshotBroadcaster::new(tx, connection_manager.clone());

    broadcaster.on_connect().await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    while rx.try_recv().is_ok() {}

    // Slave EA starts
    let heartbeat = create_heartbeat("SLAVE_001", "Slave", 5000.0, 5000.0, true);
    connection_manager.update_heartbeat(heartbeat).await;

    let connections = wait_for_snapshot(&mut rx).await;

    assert_eq!(connections.len(), 1);
    assert_eq!(connections[0].account_id, "SLAVE_001");
    assert_eq!(connections[0].ea_type.to_string(), "Slave");
    assert_eq!(connections[0].balance, 5000.0);

    broadcaster.on_disconnect().await;
}

/// E2E Test: Balance/Equity changes are reflected in snapshots
#[tokio::test]
async fn test_balance_equity_change_reflected_in_snapshot() {
    let (tx, mut rx) = broadcast::channel(100);
    let connection_manager = Arc::new(ConnectionManager::new(30));
    let broadcaster = SnapshotBroadcaster::new(tx, connection_manager.clone());

    // Setup: Master with initial balance
    let heartbeat = create_heartbeat("MASTER_BALANCE", "Master", 10000.0, 10000.0, true);
    connection_manager.update_heartbeat(heartbeat).await;

    broadcaster.on_connect().await;

    // Drain any initial snapshots
    tokio::time::sleep(Duration::from_millis(100)).await;
    while rx.try_recv().is_ok() {}

    // Balance/Equity changes (simulating trade profit)
    let updated_heartbeat = create_heartbeat("MASTER_BALANCE", "Master", 10500.0, 10600.0, true);
    connection_manager.update_heartbeat(updated_heartbeat).await;

    // Wait for snapshot with updated values
    let connections = wait_for_snapshot(&mut rx).await;

    assert_eq!(connections.len(), 1);
    assert_eq!(connections[0].account_id, "MASTER_BALANCE");
    assert_eq!(connections[0].balance, 10500.0);
    assert_eq!(connections[0].equity, 10600.0);

    broadcaster.on_disconnect().await;
}

/// E2E Test: is_trade_allowed state change is reflected in snapshots
#[tokio::test]
async fn test_trade_allowed_change_reflected_in_snapshot() {
    let (tx, mut rx) = broadcast::channel(100);
    let connection_manager = Arc::new(ConnectionManager::new(30));
    let broadcaster = SnapshotBroadcaster::new(tx, connection_manager.clone());

    // Setup: Master with auto-trading enabled
    let heartbeat = create_heartbeat("MASTER_TRADE", "Master", 10000.0, 10000.0, true);
    connection_manager.update_heartbeat(heartbeat).await;

    broadcaster.on_connect().await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    while rx.try_recv().is_ok() {}

    // Disable auto-trading (user clicks MT5 button)
    let updated_heartbeat = create_heartbeat("MASTER_TRADE", "Master", 10000.0, 10000.0, false);
    connection_manager.update_heartbeat(updated_heartbeat).await;

    let connections = wait_for_snapshot(&mut rx).await;

    assert_eq!(connections.len(), 1);
    assert_eq!(connections[0].account_id, "MASTER_TRADE");
    assert!(
        !connections[0].is_trade_allowed,
        "is_trade_allowed should be false"
    );

    // Re-enable auto-trading
    let reenabled_heartbeat = create_heartbeat("MASTER_TRADE", "Master", 10000.0, 10000.0, true);
    connection_manager
        .update_heartbeat(reenabled_heartbeat)
        .await;

    let connections = wait_for_snapshot(&mut rx).await;
    assert!(
        connections[0].is_trade_allowed,
        "is_trade_allowed should be true again"
    );

    broadcaster.on_disconnect().await;
}

/// E2E Test: Multiple EAs (Master + Slave) appear in same snapshot
#[tokio::test]
async fn test_multiple_eas_in_snapshot() {
    let (tx, mut rx) = broadcast::channel(100);
    let connection_manager = Arc::new(ConnectionManager::new(30));
    let broadcaster = SnapshotBroadcaster::new(tx, connection_manager.clone());

    // Setup: Both Master and Slave
    let master_hb = create_heartbeat("MASTER_MULTI", "Master", 20000.0, 20000.0, true);
    let slave_hb = create_heartbeat("SLAVE_MULTI", "Slave", 5000.0, 5000.0, true);
    connection_manager.update_heartbeat(master_hb).await;
    connection_manager.update_heartbeat(slave_hb).await;

    broadcaster.on_connect().await;

    let connections = wait_for_snapshot(&mut rx).await;

    assert_eq!(connections.len(), 2);

    let master = connections.iter().find(|c| c.account_id == "MASTER_MULTI");
    let slave = connections.iter().find(|c| c.account_id == "SLAVE_MULTI");

    assert!(master.is_some(), "Master should be in snapshot");
    assert!(slave.is_some(), "Slave should be in snapshot");
    assert_eq!(master.unwrap().balance, 20000.0);
    assert_eq!(slave.unwrap().balance, 5000.0);

    broadcaster.on_disconnect().await;
}

/// E2E Test: EA state changes are reflected across multiple snapshots
#[tokio::test]
async fn test_sequential_state_changes_in_snapshots() {
    let (tx, mut rx) = broadcast::channel(100);
    let connection_manager = Arc::new(ConnectionManager::new(30));
    let broadcaster = SnapshotBroadcaster::new(tx, connection_manager.clone());

    broadcaster.on_connect().await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    while rx.try_recv().is_ok() {}

    // Step 1: Master EA starts
    let hb1 = create_heartbeat("MASTER_SEQ", "Master", 10000.0, 10000.0, true);
    connection_manager.update_heartbeat(hb1).await;

    let snap1 = wait_for_snapshot(&mut rx).await;
    assert_eq!(snap1.len(), 1);
    assert_eq!(snap1[0].balance, 10000.0);
    assert!(snap1[0].is_trade_allowed);

    // Step 2: Slave EA starts
    let hb2 = create_heartbeat("SLAVE_SEQ", "Slave", 5000.0, 5000.0, true);
    connection_manager.update_heartbeat(hb2).await;

    let snap2 = wait_for_snapshot(&mut rx).await;
    assert_eq!(snap2.len(), 2);

    // Step 3: Master balance increases (profit)
    let hb3 = create_heartbeat("MASTER_SEQ", "Master", 10500.0, 10600.0, true);
    connection_manager.update_heartbeat(hb3).await;

    let snap3 = wait_for_snapshot(&mut rx).await;
    let master = snap3.iter().find(|c| c.account_id == "MASTER_SEQ").unwrap();
    assert_eq!(master.balance, 10500.0);
    assert_eq!(master.equity, 10600.0);

    // Step 4: Master disables auto-trading
    let hb4 = create_heartbeat("MASTER_SEQ", "Master", 10500.0, 10600.0, false);
    connection_manager.update_heartbeat(hb4).await;

    let snap4 = wait_for_snapshot(&mut rx).await;
    let master = snap4.iter().find(|c| c.account_id == "MASTER_SEQ").unwrap();
    assert!(!master.is_trade_allowed);

    // Step 5: Slave balance changes
    let hb5 = create_heartbeat("SLAVE_SEQ", "Slave", 5100.0, 5150.0, true);
    connection_manager.update_heartbeat(hb5).await;

    let snap5 = wait_for_snapshot(&mut rx).await;
    let slave = snap5.iter().find(|c| c.account_id == "SLAVE_SEQ").unwrap();
    assert_eq!(slave.balance, 5100.0);
    assert_eq!(slave.equity, 5150.0);

    broadcaster.on_disconnect().await;
}

/// E2E Test: Same account running as both Master and Slave (Exness pattern)
#[tokio::test]
async fn test_same_account_as_master_and_slave() {
    let (tx, mut rx) = broadcast::channel(100);
    let connection_manager = Arc::new(ConnectionManager::new(30));
    let broadcaster = SnapshotBroadcaster::new(tx, connection_manager.clone());

    // Same account_id but different ea_type
    let master_hb = create_heartbeat("EXNESS_123", "Master", 10000.0, 10000.0, true);
    let slave_hb = create_heartbeat("EXNESS_123", "Slave", 10000.0, 10000.0, true);
    connection_manager.update_heartbeat(master_hb).await;
    connection_manager.update_heartbeat(slave_hb).await;

    broadcaster.on_connect().await;

    let connections = wait_for_snapshot(&mut rx).await;

    // Both should appear (same account_id, different ea_type)
    assert_eq!(connections.len(), 2);

    let master = connections
        .iter()
        .find(|c| c.account_id == "EXNESS_123" && c.ea_type.to_string() == "Master");
    let slave = connections
        .iter()
        .find(|c| c.account_id == "EXNESS_123" && c.ea_type.to_string() == "Slave");

    assert!(master.is_some(), "Master instance should be in snapshot");
    assert!(slave.is_some(), "Slave instance should be in snapshot");

    broadcaster.on_disconnect().await;
}
