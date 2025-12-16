use super::MessageHandler;
use crate::adapters::infrastructure::connection_manager::ConnectionManager as ConnectionManagerImpl;
use crate::adapters::outbound::messaging::ZmqConfigPublisher;
use crate::adapters::outbound::persistence::Database;
use crate::domain::models::{
    HeartbeatMessage,
    OrderType,
    TradeAction,
    TradeSignal,
    // LotCalculationMode, // Unused
};
use crate::domain::services::copy_engine::CopyEngine;
// use crate::ports::{ConnectionManager, TradeGroupRepository};
use crate::application::runtime_status_updater::RuntimeStatusMetrics;
use chrono::Utc;
use std::ops::Deref;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Test context wrapper for MessageHandler with proper cleanup
///
/// This struct ensures ZeroMQ resources are properly released after each test.
/// When dropped, it waits briefly for background ZMQ tasks to complete.
pub(crate) struct TestContext {
    pub handler: MessageHandler,
    // Store Arc references to ensure proper drop order
    _publisher: Arc<ZmqConfigPublisher>,
    /// Broadcast receiver for testing WebSocket notifications
    pub _broadcast_rx: broadcast::Receiver<String>, // Prefixed with _ to suppress warning
}

impl TestContext {
    /// Create a new test context with in-memory database
    /// Uses dynamic ports (tcp://127.0.0.1:*) to avoid port conflicts
    pub async fn new() -> Self {
        let connection_manager = Arc::new(ConnectionManagerImpl::new(30));
        let copy_engine = Arc::new(CopyEngine::new());

        let (broadcast_tx, broadcast_rx) = broadcast::channel::<String>(100);

        // Create test database (in-memory)
        let db = Arc::new(Database::new("sqlite::memory:").await.unwrap());

        // Create unified ZmqConfigPublisher for tests with dynamic port
        // (handles both config and trade signals)
        let publisher = Arc::new(ZmqConfigPublisher::new("tcp://127.0.0.1:*").unwrap());

        // Create runtime status updater
        let metrics = Arc::new(RuntimeStatusMetrics::default());
        let runtime_updater = Arc::new(
            crate::application::runtime_status_updater::RuntimeStatusUpdater::with_metrics(
                db.clone(),
                connection_manager.clone(),
                metrics.clone(),
            ),
        );

        // Create snapshot broadcaster for StatusService (UpdateBroadcaster)
        let snapshot_broadcaster =
            Arc::new(crate::adapters::inbound::http::SnapshotBroadcaster::new(
                broadcast_tx.clone(),
                connection_manager.clone(),
                db.clone(),
            ));

        // Construct StatusService with real components (acting as adapters)
        let status_service = crate::application::StatusService::new(
            connection_manager.clone(),
            db.clone(),
            publisher.clone(),
            runtime_updater,
            Some(snapshot_broadcaster),
            None,
        );

        let handler = MessageHandler::new(
            connection_manager,
            copy_engine,
            broadcast_tx,
            db,
            publisher.clone(),
            None, // vlogs_controller - not needed for tests
            Arc::new(RuntimeStatusMetrics::default()),
            status_service, // Inject StatusService
        );

        Self {
            handler,
            _publisher: publisher,
            _broadcast_rx: broadcast_rx,
        }
    }

    /// Explicitly cleanup ZeroMQ resources
    ///
    /// Drops Arc references and waits for background tasks to complete.
    /// Call this at the end of tests for clean shutdown.
    #[allow(dead_code)]
    pub async fn cleanup(self) {
        // Drop self to release Arc references
        drop(self);
        // Brief wait for ZMQ background tasks to finish cleanup
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
}

impl Deref for TestContext {
    type Target = MessageHandler;

    fn deref(&self) -> &Self::Target {
        &self.handler
    }
}

impl Drop for TestContext {
    fn drop(&mut self) {
        // Resources will be cleaned up when Arc references are dropped
    }
}

/// Create a test context with MessageHandler and proper resource management
pub(crate) async fn create_test_context() -> TestContext {
    TestContext::new().await
}

/// Build a reusable HeartbeatMessage for tests
pub(crate) fn build_heartbeat(
    account_id: &str,
    ea_type: &str,
    is_trade_allowed: bool,
) -> HeartbeatMessage {
    HeartbeatMessage {
        message_type: "Heartbeat".to_string(),
        account_id: account_id.to_string(),
        balance: 10_000.0,
        equity: 10_000.0,
        open_positions: 0,
        timestamp: Utc::now().to_rfc3339(),
        version: "1.0.0".to_string(),
        ea_type: ea_type.to_string(),
        platform: "MT5".to_string(),
        account_number: 123456,
        broker: "TestBroker".to_string(),
        account_name: "TestAccount".to_string(),
        server: "TestServer".to_string(),
        currency: "USD".to_string(),
        leverage: 100,
        is_trade_allowed,
        symbol_prefix: None,
        symbol_suffix: None,
        symbol_map: None,
    }
}

/// Create a test TradeSignal
pub(crate) fn create_test_trade_signal() -> TradeSignal {
    TradeSignal {
        action: TradeAction::Open,
        ticket: 12345,
        symbol: Some("EURUSD".to_string()),
        order_type: Some(OrderType::Buy),
        lots: Some(0.1),
        open_price: Some(1.1000),
        stop_loss: Some(1.0950),
        take_profit: Some(1.1050),
        magic_number: Some(0),
        comment: Some("Test trade".to_string()),
        timestamp: Utc::now(),
        source_account: "MASTER_001".to_string(),
        close_ratio: None,
    }
}
