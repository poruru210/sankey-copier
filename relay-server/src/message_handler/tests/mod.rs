//! Shared test utilities for message handler tests
//!
//! Provides helper functions to create test instances and test data.

use super::*;
use crate::models::{
    CopySettings, HeartbeatMessage, OrderType, TradeAction, TradeFilters, TradeSignal,
};
use chrono::Utc;
use std::sync::atomic::{AtomicU16, Ordering};

// Test submodules
mod config_tests;
mod heartbeat_tests;
mod trade_signal_tests;
mod unregister_tests;

// Port counter for unique test ports
static PORT_COUNTER: AtomicU16 = AtomicU16::new(7000);

/// Create a test MessageHandler instance with in-memory database
pub(crate) async fn create_test_handler() -> MessageHandler {
    let connection_manager = Arc::new(ConnectionManager::new(30));
    let copy_engine = Arc::new(CopyEngine::new());

    // Use unique port for each test to avoid "Address in use" errors
    let port = PORT_COUNTER.fetch_add(1, Ordering::SeqCst);
    let zmq_sender = Arc::new(ZmqSender::new(&format!("tcp://127.0.0.1:{}", port)).unwrap());

    let (broadcast_tx, _) = broadcast::channel::<String>(100);

    // Create test database (in-memory)
    let db = Arc::new(Database::new("sqlite::memory:").await.unwrap());

    // Create ZmqConfigPublisher for tests
    let config_port = PORT_COUNTER.fetch_add(1, Ordering::SeqCst);
    let config_sender =
        Arc::new(ZmqConfigPublisher::new(&format!("tcp://127.0.0.1:{}", config_port)).unwrap());

    MessageHandler::new(
        connection_manager,
        copy_engine,
        zmq_sender,
        broadcast_tx,
        db,
        config_sender,
    )
}

/// Create a test TradeSignal
pub(crate) fn create_test_trade_signal() -> TradeSignal {
    TradeSignal {
        action: TradeAction::Open,
        ticket: 12345,
        symbol: "EURUSD".to_string(),
        order_type: OrderType::Buy,
        lots: 0.1,
        open_price: 1.1000,
        stop_loss: Some(1.0950),
        take_profit: Some(1.1050),
        magic_number: 0,
        comment: "Test trade".to_string(),
        timestamp: Utc::now(),
        source_account: "MASTER_001".to_string(),
    }
}

/// Create a test CopySettings
pub(crate) fn create_test_copy_settings() -> CopySettings {
    CopySettings {
        id: 1,
        status: 2, // STATUS_CONNECTED
        master_account: "MASTER_001".to_string(),
        slave_account: "SLAVE_001".to_string(),
        lot_multiplier: Some(1.0),
        reverse_trade: false,
        symbol_prefix: None,
        symbol_suffix: None,
        symbol_mappings: vec![],
        filters: TradeFilters {
            allowed_symbols: None,
            blocked_symbols: None,
            allowed_magic_numbers: None,
            blocked_magic_numbers: None,
        },
    }
}
