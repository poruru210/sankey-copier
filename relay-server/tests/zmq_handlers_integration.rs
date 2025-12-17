// relay-server/tests/zmq_handlers_integration.rs
//
// Integration tests for ZMQ message handlers.
// Tests Register, PositionSnapshot, and SyncRequest handlers.

use sankey_copier_relay_server::adapters::inbound::http::SnapshotBroadcaster;
use sankey_copier_relay_server::adapters::inbound::zmq::MessageHandler;
use sankey_copier_relay_server::adapters::infrastructure::connection_manager::ConnectionManager;
use sankey_copier_relay_server::adapters::outbound::messaging::{ZmqConfigPublisher, ZmqMessage};
use sankey_copier_relay_server::adapters::outbound::persistence::Database;
use sankey_copier_relay_server::application::runtime_status_updater::{
    RuntimeStatusMetrics, RuntimeStatusUpdater,
};
use sankey_copier_relay_server::application::StatusService;
use sankey_copier_relay_server::domain::models::{
    MasterSettings, PositionSnapshotMessage, RegisterMessage, SlaveSettings, SyncRequestMessage,
    STATUS_CONNECTED,
};
use sankey_copier_relay_server::domain::services::copy_engine::CopyEngine;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Test context for ZMQ handler integration tests.
struct TestContext {
    handler: MessageHandler,
    db: Arc<Database>,
    connection_manager: Arc<ConnectionManager>,
    _publisher: Arc<ZmqConfigPublisher>,
}

impl TestContext {
    async fn new() -> Self {
        let connection_manager = Arc::new(ConnectionManager::new(30));
        let copy_engine = Arc::new(CopyEngine::new());
        let (broadcast_tx, _rx) = broadcast::channel::<String>(100);
        let db = Arc::new(Database::new("sqlite::memory:").await.unwrap());
        let publisher = Arc::new(ZmqConfigPublisher::new("tcp://127.0.0.1:*").unwrap());

        let metrics = Arc::new(RuntimeStatusMetrics::default());
        let runtime_updater = Arc::new(RuntimeStatusUpdater::with_metrics(
            db.clone(),
            connection_manager.clone(),
            metrics.clone(),
        ));

        let snapshot_broadcaster = Arc::new(SnapshotBroadcaster::new(
            broadcast_tx.clone(),
            connection_manager.clone(),
            db.clone(),
        ));

        let ws_broadcaster = Arc::new(
            sankey_copier_relay_server::adapters::outbound::messaging::WebsocketBroadcaster::new(
                broadcast_tx.clone(),
            ),
        );

        let disconnection_service = Arc::new(
            sankey_copier_relay_server::application::disconnection_service::RealDisconnectionService::new(
                connection_manager.clone(),
                db.clone(),
                publisher.clone(),
                ws_broadcaster.clone(),
                metrics.clone(),
            ),
        );

        let status_service = StatusService::new(
            connection_manager.clone(),
            db.clone(),
            publisher.clone(),
            runtime_updater,
            Some(snapshot_broadcaster),
            None,
        );

        let handler = MessageHandler::new(
            connection_manager.clone(),
            copy_engine,
            broadcast_tx,
            db.clone(),
            publisher.clone(),
            None,
            metrics,
            status_service,
            disconnection_service,
            Arc::new(sankey_copier_relay_server::config::Config::default()),
        );

        Self {
            handler,
            db,
            connection_manager,
            _publisher: publisher,
        }
    }
}

/// Helper function to build a RegisterMessage with all required fields
fn build_register_message(account_id: &str, ea_type: &str) -> RegisterMessage {
    RegisterMessage {
        message_type: "Register".to_string(),
        account_id: account_id.to_string(),
        ea_type: ea_type.to_string(),
        platform: "MT5".to_string(),
        account_number: 12345,
        broker: "TestBroker".to_string(),
        account_name: "TestAccount".to_string(),
        server: "TestServer".to_string(),
        currency: "USD".to_string(),
        leverage: 100,
        timestamp: chrono::Utc::now().to_rfc3339(),
        symbol_context: None,
        is_trade_allowed: false,
    }
}

/// Helper function to build a PositionSnapshotMessage
fn build_position_snapshot(source_account: &str) -> PositionSnapshotMessage {
    PositionSnapshotMessage {
        message_type: "PositionSnapshot".to_string(),
        source_account: source_account.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        positions: vec![],
    }
}

/// Helper function to build a SyncRequestMessage
fn build_sync_request(master_account: &str, slave_account: &str) -> SyncRequestMessage {
    SyncRequestMessage {
        message_type: "SyncRequest".to_string(),
        master_account: master_account.to_string(),
        slave_account: slave_account.to_string(),
        last_sync_time: None,
        timestamp: chrono::Utc::now().to_rfc3339(),
    }
}

// =============================================================================
// Register Handler Tests
// =============================================================================

#[tokio::test]
async fn test_register_master_with_existing_trade_group() {
    let ctx = TestContext::new().await;
    let master_account = "MASTER_REG_001";

    // Setup: Create TradeGroup before registration
    ctx.db.create_trade_group(master_account).await.unwrap();
    ctx.db
        .update_master_settings(master_account, MasterSettings::default())
        .await
        .unwrap();

    // Register Master EA
    let msg = build_register_message(master_account, "Master");
    ctx.handler.handle_message(ZmqMessage::Register(msg)).await;

    // Verify: Master should be registered in ConnectionManager
    let conn = ctx.connection_manager.get_master(master_account).await;
    assert!(conn.is_some(), "Master should be registered");
    assert_eq!(conn.unwrap().account_id, master_account);
}

#[tokio::test]
async fn test_register_master_without_trade_group() {
    let ctx = TestContext::new().await;
    let master_account = "MASTER_REG_002";

    // No TradeGroup exists - registration should still work but no config sent
    let msg = build_register_message(master_account, "Master");
    ctx.handler.handle_message(ZmqMessage::Register(msg)).await;

    // Verify: Master should be registered even without TradeGroup
    let conn = ctx.connection_manager.get_master(master_account).await;
    assert!(conn.is_some(), "Master should be registered");
}

#[tokio::test]
async fn test_register_slave_with_master_connection() {
    let ctx = TestContext::new().await;
    let master_account = "MASTER_REG_003";
    let slave_account = "SLAVE_REG_001";

    // Setup: Create TradeGroup and add slave as member
    ctx.db.create_trade_group(master_account).await.unwrap();
    ctx.db
        .add_member(
            master_account,
            slave_account,
            SlaveSettings::default(),
            STATUS_CONNECTED,
        )
        .await
        .unwrap();

    // Register Slave EA
    let msg = build_register_message(slave_account, "Slave");
    ctx.handler.handle_message(ZmqMessage::Register(msg)).await;

    // Verify: Slave should be registered
    let conn = ctx.connection_manager.get_slave(slave_account).await;
    assert!(conn.is_some(), "Slave should be registered");
}

#[tokio::test]
async fn test_register_slave_without_master_connection() {
    let ctx = TestContext::new().await;
    let slave_account = "SLAVE_REG_002";

    // No Master connection - registration should still work
    let msg = build_register_message(slave_account, "Slave");
    ctx.handler.handle_message(ZmqMessage::Register(msg)).await;

    // Verify: Slave should be registered
    let conn = ctx.connection_manager.get_slave(slave_account).await;
    assert!(
        conn.is_some(),
        "Slave should be registered even without master"
    );
}

// =============================================================================
// PositionSnapshot Handler Tests
// =============================================================================

#[tokio::test]
async fn test_position_snapshot_with_slaves() {
    let ctx = TestContext::new().await;
    let master_account = "MASTER_SNAP_001";
    let slave_account = "SLAVE_SNAP_001";

    // Setup: Create TradeGroup and add slave
    ctx.db.create_trade_group(master_account).await.unwrap();
    ctx.db
        .add_member(
            master_account,
            slave_account,
            SlaveSettings::default(),
            STATUS_CONNECTED,
        )
        .await
        .unwrap();

    // Send PositionSnapshot
    let snapshot = build_position_snapshot(master_account);

    // This should not panic - it routes to slaves
    ctx.handler
        .handle_message(ZmqMessage::PositionSnapshot(snapshot))
        .await;

    // Verify: Members should still exist (snapshot doesn't modify DB)
    let members = ctx.db.get_members(master_account).await.unwrap();
    assert_eq!(members.len(), 1);
}

#[tokio::test]
async fn test_position_snapshot_without_slaves() {
    let ctx = TestContext::new().await;
    let master_account = "MASTER_SNAP_002";

    // Setup: Create TradeGroup without slaves
    ctx.db.create_trade_group(master_account).await.unwrap();

    // Send PositionSnapshot
    let snapshot = build_position_snapshot(master_account);

    // Should not panic - just logs "no slaves connected"
    ctx.handler
        .handle_message(ZmqMessage::PositionSnapshot(snapshot))
        .await;
}

#[tokio::test]
async fn test_position_snapshot_unknown_master() {
    let ctx = TestContext::new().await;
    let unknown_master = "UNKNOWN_MASTER";

    // Send PositionSnapshot from unknown master
    let snapshot = build_position_snapshot(unknown_master);

    // Should not panic - no members to route to
    ctx.handler
        .handle_message(ZmqMessage::PositionSnapshot(snapshot))
        .await;
}

// =============================================================================
// SyncRequest Handler Tests
// =============================================================================

#[tokio::test]
async fn test_sync_request_valid_member() {
    let ctx = TestContext::new().await;
    let master_account = "MASTER_SYNC_001";
    let slave_account = "SLAVE_SYNC_001";

    // Setup: Create TradeGroup and add slave as member
    ctx.db.create_trade_group(master_account).await.unwrap();
    ctx.db
        .add_member(
            master_account,
            slave_account,
            SlaveSettings::default(),
            STATUS_CONNECTED,
        )
        .await
        .unwrap();

    // Send SyncRequest from valid member
    let request = build_sync_request(master_account, slave_account);

    // Should route to master (via ZMQ publish)
    ctx.handler
        .handle_message(ZmqMessage::SyncRequest(request))
        .await;
}

#[tokio::test]
async fn test_sync_request_invalid_member() {
    let ctx = TestContext::new().await;
    let master_account = "MASTER_SYNC_002";
    let invalid_slave = "INVALID_SLAVE";

    // Setup: Create TradeGroup without this slave
    ctx.db.create_trade_group(master_account).await.unwrap();

    // Send SyncRequest from non-member
    let request = build_sync_request(master_account, invalid_slave);

    // Should be rejected (logged as warning, not routed)
    ctx.handler
        .handle_message(ZmqMessage::SyncRequest(request))
        .await;
}

#[tokio::test]
async fn test_sync_request_unknown_master() {
    let ctx = TestContext::new().await;
    let unknown_master = "UNKNOWN_MASTER";
    let slave_account = "SLAVE_SYNC_002";

    // No TradeGroup for this master
    let request = build_sync_request(unknown_master, slave_account);

    // Should not panic - just logs error
    ctx.handler
        .handle_message(ZmqMessage::SyncRequest(request))
        .await;
}
