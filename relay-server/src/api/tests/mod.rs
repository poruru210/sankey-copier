//! Test utilities for API module
//!
//! Provides shared test utilities for creating test objects
//! like connections, app state, heartbeats, and settings.

mod helpers_tests;
mod trade_group_members_tests;

use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use chrono::Utc;

use crate::{
    config::Config,
    connection_manager::ConnectionManager,
    db::Database,
    log_buffer::LogBuffer,
    models::{ConnectionStatus, CopySettings, EaConnection, EaType, Platform, SymbolMapping, TradeFilters},
    zeromq::ZmqConfigPublisher,
};
use crate::api::AppState;

/// Create a test EA connection
pub(crate) fn create_test_connection(is_trade_allowed: bool) -> EaConnection {
    EaConnection {
        account_id: "MASTER".to_string(),
        ea_type: EaType::Master,
        platform: Platform::MT5,
        account_number: 12345,
        broker: "Broker".to_string(),
        account_name: "Name".to_string(),
        server: "Server".to_string(),
        balance: 1000.0,
        equity: 1000.0,
        currency: "USD".to_string(),
        leverage: 100,
        last_heartbeat: Utc::now(),
        status: ConnectionStatus::Online,
        connected_at: Utc::now(),
        is_trade_allowed,
    }
}

/// Create a test AppState with in-memory database
pub(crate) async fn create_test_app_state() -> AppState {
    static PORT_COUNTER: AtomicU16 = AtomicU16::new(15557);

    let db = Arc::new(Database::new("sqlite::memory:").await.unwrap());
    let (tx, _) = broadcast::channel(100);
    let settings_cache = Arc::new(RwLock::new(vec![]));
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
        settings_cache,
        connection_manager,
        config_sender,
        log_buffer,
        allowed_origins: vec![],
        cors_disabled: true,
        config,
    }
}

/// Create a test heartbeat message
pub(crate) fn create_test_heartbeat(
    account_id: &str,
    is_trade_allowed: bool,
) -> crate::models::HeartbeatMessage {
    crate::models::HeartbeatMessage {
        message_type: "Heartbeat".to_string(),
        account_id: account_id.to_string(),
        balance: 10000.0,
        equity: 10000.0,
        open_positions: 0,
        timestamp: chrono::Utc::now().to_rfc3339(),
        version: "1.0.0".to_string(),
        ea_type: "Master".to_string(),
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

/// Create a test CopySettings object
pub(crate) fn create_test_copy_settings() -> CopySettings {
    CopySettings {
        id: 1,
        status: 1,
        master_account: "MASTER123".to_string(),
        slave_account: "SLAVE456".to_string(),
        lot_multiplier: Some(2.0),
        reverse_trade: false,
        symbol_prefix: Some("pro.".to_string()),
        symbol_suffix: Some(".m".to_string()),
        symbol_mappings: vec![SymbolMapping {
            source_symbol: "EURUSD".to_string(),
            target_symbol: "EURUSD.e".to_string(),
        }],
        filters: TradeFilters {
            allowed_symbols: Some(vec!["EURUSD".to_string()]),
            blocked_symbols: None,
            allowed_magic_numbers: Some(vec![12345]),
            blocked_magic_numbers: None,
        },
    }
}
