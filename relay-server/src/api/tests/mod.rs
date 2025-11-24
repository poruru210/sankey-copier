//! Test utilities for API module
//!
//! Provides shared test utilities for creating test objects
//! like connections, app state, heartbeats, and settings.

mod trade_group_members_tests;

use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::api::AppState;
use crate::{
    config::Config, connection_manager::ConnectionManager, db::Database, log_buffer::LogBuffer,
    zeromq::ZmqConfigPublisher,
};

/// Create a test AppState with in-memory database
pub(crate) async fn create_test_app_state() -> AppState {
    static PORT_COUNTER: AtomicU16 = AtomicU16::new(15557);

    let db = Arc::new(Database::new("sqlite::memory:").await.unwrap());
    let (tx, _) = broadcast::channel(100);
    let connection_manager = Arc::new(ConnectionManager::new(60)); // 60 second timeout

    // Use unique port for each test to avoid "Address in use" errors
    let port = PORT_COUNTER.fetch_add(1, Ordering::SeqCst);
    let config_sender = Arc::new(
        ZmqConfigPublisher::new(&format!("tcp://127.0.0.1:{}", port))
            .expect("Failed to create test config publisher"),
    );
    let log_buffer = LogBuffer::default();
    let config = Arc::new(Config::default());

    AppState {
        db,
        tx,
        connection_manager,
        config_sender,
        log_buffer,
        allowed_origins: vec![],
        cors_disabled: true,
        config,
    }
}
