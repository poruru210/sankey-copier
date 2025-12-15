//! Shared test utilities for message handler tests
//!
//! Provides helper functions to create test instances and test data.
//! Uses TestContext wrapper for proper ZeroMQ resource cleanup.

use super::*;
use crate::models::{HeartbeatMessage, OrderType, TradeAction, TradeFilters, TradeSignal};
use crate::runtime_status_updater::RuntimeStatusMetrics;
use chrono::Utc;
use std::ops::Deref;

// Test submodules
mod config_tests;
mod heartbeat_tests;
mod trade_signal_tests;
mod unregister_tests;
mod warning_codes_tests;

/// Test context wrapper for MessageHandler with proper cleanup
///
/// This struct ensures ZeroMQ resources are properly released after each test.
/// When dropped, it waits briefly for background ZMQ tasks to complete.
pub(crate) struct TestContext {
    pub handler: MessageHandler,
    // Store Arc references to ensure proper drop order
    _publisher: Arc<ZmqConfigPublisher>,
    /// Broadcast receiver for testing WebSocket notifications
    pub broadcast_rx: broadcast::Receiver<String>,
}

impl TestContext {
    /// Create a new test context with in-memory database
    /// Uses dynamic ports (tcp://127.0.0.1:*) to avoid port conflicts
    pub async fn new() -> Self {
        let connection_manager = Arc::new(ConnectionManager::new(30));
        let copy_engine = Arc::new(CopyEngine::new());

        let (broadcast_tx, broadcast_rx) = broadcast::channel::<String>(100);

        // Create test database (in-memory)
        let db = Arc::new(Database::new("sqlite::memory:").await.unwrap());

        // Create unified ZmqConfigPublisher for tests with dynamic port
        // (handles both config and trade signals)
        let publisher = Arc::new(ZmqConfigPublisher::new("tcp://127.0.0.1:*").unwrap());

        // Create runtime status adapter for StatusEvaluator
        let metrics = Arc::new(RuntimeStatusMetrics::default());
        let runtime_updater = crate::runtime_status_updater::RuntimeStatusUpdater::with_metrics(
            db.clone(),
            connection_manager.clone(),
            metrics.clone(),
        );
        let status_evaluator = Arc::new(
            crate::ports::adapters::RuntimeStatusEvaluatorAdapter::new(runtime_updater),
        );

        // Create snapshot broadcaster for StatusService (UpdateBroadcaster)
        let snapshot_broadcaster = Arc::new(crate::api::SnapshotBroadcaster::new(
            broadcast_tx.clone(),
            connection_manager.clone(),
            db.clone(),
        ));

        // Construct StatusService with real components (acting as adapters)
        let status_service = crate::services::StatusService::new(
            connection_manager.clone(),
            db.clone(),
            publisher.clone(),
            Some(status_evaluator),
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
            Some(status_service), // Inject StatusService
        );

        Self {
            handler,
            _publisher: publisher,
            broadcast_rx,
        }
    }

    /// Collect all pending broadcast messages (non-blocking)
    /// Returns messages that were sent via broadcast_tx
    #[allow(dead_code)]
    pub fn collect_broadcast_messages(&mut self) -> Vec<String> {
        let mut messages = Vec::new();
        loop {
            match self.broadcast_rx.try_recv() {
                Ok(msg) => messages.push(msg),
                Err(broadcast::error::TryRecvError::Empty) => break,
                Err(broadcast::error::TryRecvError::Lagged(_)) => continue,
                Err(broadcast::error::TryRecvError::Closed) => break,
            }
        }
        messages
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
        // The ZmqPublisher/ZmqConfigPublisher tasks will exit when their
        // channel senders are dropped, causing blocking_recv() to return None
    }
}

/// Create a test context with MessageHandler and proper resource management
/// Uses dynamic ports (tcp://127.0.0.1:*) to avoid port conflicts
///
/// Returns TestContext which derefs to MessageHandler for easy use.
/// Resources are cleaned up when TestContext is dropped.
pub(crate) async fn create_test_context() -> TestContext {
    TestContext::new().await
}

/// Create a test MessageHandler instance with in-memory database
/// Uses dynamic ports (tcp://127.0.0.1:*) to avoid port conflicts
///
/// DEPRECATED: Use create_test_context() for proper cleanup.
/// This function returns only MessageHandler, losing ZMQ cleanup tracking.
#[allow(dead_code)]
pub(crate) async fn create_test_handler() -> MessageHandler {
    // Note: This creates ZMQ resources that won't be tracked for cleanup.
    // The resources will still be cleaned up when dropped, but without
    // explicit lifecycle management.
    let connection_manager = Arc::new(ConnectionManager::new(30));
    let copy_engine = Arc::new(CopyEngine::new());
    let (broadcast_tx, _) = broadcast::channel::<String>(100);
    let db = Arc::new(Database::new("sqlite::memory:").await.unwrap());
    let publisher = Arc::new(ZmqConfigPublisher::new("tcp://127.0.0.1:*").unwrap());

    MessageHandler::new(
        connection_manager,
        copy_engine,
        broadcast_tx,
        db,
        publisher,
        None,
        Arc::new(RuntimeStatusMetrics::default()),
        None, // status_service - use legacy heartbeat logic
    )
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
