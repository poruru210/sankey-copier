//! Test utilities for API module
//!
//! Provides shared test utilities for creating test objects
//! like connections, app state, heartbeats, and settings.

mod runtime_metrics_tests;
mod trade_group_members_tests;
mod websocket_tests;

use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::{
    api::{AppState, SnapshotBroadcaster},
    config::{Config, VictoriaLogsConfig},
    connection_manager::ConnectionManager,
    db::Database,
    log_buffer::LogBuffer,
    port_resolver::ResolvedPorts,
    runtime_status_updater::RuntimeStatusMetrics,
    victoria_logs::VLogsController,
    zeromq::ZmqConfigPublisher,
};

/// Create a test AppState with in-memory database (VictoriaLogs not configured)
pub(crate) async fn create_test_app_state() -> AppState {
    create_test_app_state_with_vlogs(false).await
}

/// Create a test AppState with optional VictoriaLogs controller
pub(crate) async fn create_test_app_state_with_vlogs(vlogs_configured: bool) -> AppState {
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

    // Create VLogsController if configured
    let vlogs_controller = if vlogs_configured {
        let enabled_flag = Arc::new(AtomicBool::new(true));
        let vlogs_config = VictoriaLogsConfig {
            enabled: true,
            host: "http://localhost:9428".to_string(),
            batch_size: 100,
            flush_interval_secs: 5,
            source: "test-relay".to_string(),
        };
        Some(VLogsController::new(enabled_flag, vlogs_config))
    } else {
        None
    };

    // Create default resolved ports for testing (2-port architecture)
    let resolved_ports = Arc::new(ResolvedPorts {
        http_port: 3000,
        receiver_port: 5555,
        sender_port: port, // Use the same port as the config_sender/publisher
        is_dynamic: false,
        generated_at: None,
    });

    // Create snapshot broadcaster for testing
    let snapshot_broadcaster = SnapshotBroadcaster::new(tx.clone(), connection_manager.clone());

    AppState {
        db,
        tx,
        connection_manager,
        config_sender,
        log_buffer,
        allowed_origins: vec![],
        cors_disabled: true,
        config,
        resolved_ports,
        vlogs_controller,
        runtime_status_metrics: Arc::new(RuntimeStatusMetrics::default()),
        snapshot_broadcaster,
    }
}
