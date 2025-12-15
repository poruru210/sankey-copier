//! Tests for WebSocket snapshot broadcaster

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

use crate::adapters::inbound::http::SnapshotBroadcaster;
use crate::adapters::outbound::persistence::Database;
use crate::connection_manager::ConnectionManager;
use crate::domain::models::HeartbeatMessage;

/// Create a test HeartbeatMessage
fn create_test_heartbeat(account_id: &str, ea_type: &str) -> HeartbeatMessage {
    HeartbeatMessage {
        message_type: "Heartbeat".to_string(),
        account_id: account_id.to_string(),
        balance: 10000.0,
        equity: 10000.0,
        open_positions: 0,
        timestamp: chrono::Utc::now().to_rfc3339(),
        version: "test".to_string(),
        ea_type: ea_type.to_string(),
        platform: "MT5".to_string(),
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

/// Test that subscriber count starts at zero
#[tokio::test]
async fn test_snapshot_broadcaster_initial_state() {
    let (tx, _rx) = broadcast::channel(100);
    let connection_manager = Arc::new(ConnectionManager::new(30));
    let db = Arc::new(Database::new("sqlite::memory:").await.unwrap());
    let broadcaster = SnapshotBroadcaster::new(tx, connection_manager, db);

    assert_eq!(broadcaster.subscriber_count(), 0);
}

/// Test that on_connect increments subscriber count
#[tokio::test]
async fn test_snapshot_broadcaster_on_connect() {
    let (tx, _rx) = broadcast::channel(100);
    let connection_manager = Arc::new(ConnectionManager::new(30));
    let db = Arc::new(Database::new("sqlite::memory:").await.unwrap());
    let broadcaster = SnapshotBroadcaster::new(tx, connection_manager, db);

    broadcaster.on_connect().await;
    assert_eq!(broadcaster.subscriber_count(), 1);

    broadcaster.on_connect().await;
    assert_eq!(broadcaster.subscriber_count(), 2);
}

/// Test that on_disconnect decrements subscriber count
#[tokio::test]
async fn test_snapshot_broadcaster_on_disconnect() {
    let (tx, _rx) = broadcast::channel(100);
    let connection_manager = Arc::new(ConnectionManager::new(30));
    let db = Arc::new(Database::new("sqlite::memory:").await.unwrap());
    let broadcaster = SnapshotBroadcaster::new(tx, connection_manager, db);

    broadcaster.on_connect().await;
    broadcaster.on_connect().await;
    assert_eq!(broadcaster.subscriber_count(), 2);

    broadcaster.on_disconnect().await;
    assert_eq!(broadcaster.subscriber_count(), 1);

    broadcaster.on_disconnect().await;
    assert_eq!(broadcaster.subscriber_count(), 0);
}

/// Test that timer task starts on first subscriber and broadcasts snapshots
#[tokio::test]
async fn test_snapshot_broadcaster_sends_snapshot() {
    let (tx, mut rx) = broadcast::channel(100);
    let connection_manager = Arc::new(ConnectionManager::new(30));
    let db = Arc::new(Database::new("sqlite::memory:").await.unwrap());
    let broadcaster = SnapshotBroadcaster::new(tx, connection_manager.clone(), db);

    // Add a test connection to the manager
    let heartbeat = create_test_heartbeat("TEST_123", "Master");
    connection_manager.update_heartbeat(heartbeat).await;

    // Connect a subscriber (starts the timer)
    broadcaster.on_connect().await;

    // Wait for the first snapshot (should come within 3 seconds + some buffer)
    let result = tokio::time::timeout(Duration::from_secs(5), rx.recv()).await;

    // Disconnect to stop the timer
    broadcaster.on_disconnect().await;

    // Verify we received a snapshot
    assert!(result.is_ok(), "Should receive a snapshot within timeout");
    let message = result.unwrap().unwrap();
    // Changed from connections_snapshot to system_snapshot
    assert!(
        message.starts_with("system_snapshot:"),
        "Message should be a system snapshot, got: {}",
        &message[..message.len().min(50)]
    );

    // Parse the snapshot and verify content
    let json_part = message.strip_prefix("system_snapshot:").unwrap();
    let snapshot: serde_json::Value = serde_json::from_str(json_part).unwrap();
    let connections = snapshot["connections"].as_array().unwrap();
    assert_eq!(connections.len(), 1);
    assert_eq!(connections[0]["account_id"], "TEST_123");
}

/// Test that timer task stops when last subscriber disconnects
#[tokio::test]
async fn test_snapshot_broadcaster_timer_stops() {
    let (tx, mut rx) = broadcast::channel(100);
    let connection_manager = Arc::new(ConnectionManager::new(30));
    let db = Arc::new(Database::new("sqlite::memory:").await.unwrap());
    let broadcaster = SnapshotBroadcaster::new(tx, connection_manager, db);

    // Connect and immediately disconnect
    broadcaster.on_connect().await;
    broadcaster.on_disconnect().await;

    // Wait a bit for the timer to potentially send something
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Drain any pending messages
    while rx.try_recv().is_ok() {}

    // Wait another interval - should NOT receive any more snapshots
    let result = tokio::time::timeout(Duration::from_secs(4), rx.recv()).await;

    // The timer should be stopped, so we should timeout
    assert!(
        result.is_err(),
        "Should not receive snapshots after last subscriber disconnects"
    );
}
