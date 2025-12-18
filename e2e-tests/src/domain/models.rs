// e2e-tests/src/types.rs
//
// Common type definitions and constants for EA Simulators.

// Re-export types for test files
pub use sankey_copier_zmq::SlaveConfigMessage as SlaveConfig;
pub use sankey_copier_zmq::{
    GlobalConfigMessage, HeartbeatMessage as Heartbeat, MasterConfigMessage, OrderType,
    PositionInfo, PositionSnapshotMessage, RegisterMessage, RequestConfigMessage, SymbolMapping,
    SyncMode, SyncRequestMessage, TradeAction, TradeFilters, TradeSignal, UnregisterMessage,
};

/// Buffer size for ZMQ message reception
pub const BUFFER_SIZE: usize = 65536;

/// Topic buffer size for FFI calls
pub const TOPIC_BUFFER_SIZE: i32 = 256;

// =============================================================================
// MQL5 Common.mqh L28-49 準拠定数
// =============================================================================

/// Heartbeat interval in seconds
///
/// MQL5 production: HEARTBEAT_INTERVAL_SECONDS = 30
/// E2E tests: 1 second for faster test execution
pub const HEARTBEAT_INTERVAL_SECONDS: u64 = 1;

/// OnTimer interval in milliseconds (Slave SignalPollingIntervalMs default)
pub const ONTIMER_INTERVAL_MS: u64 = 100;

/// Status: No configuration received yet (MQL5: STATUS_NO_CONFIG = -1)
pub const STATUS_NO_CONFIG: i32 = -1;

/// Status: Slave is disabled (MQL5: STATUS_DISABLED = 0)
pub const STATUS_DISABLED: i32 = 0;

/// Status: Slave is enabled, Master disconnected (MQL5: STATUS_ENABLED = 1)
pub const STATUS_ENABLED: i32 = 1;

/// Status: Slave is enabled, Master connected (MQL5: STATUS_CONNECTED = 2)
pub const STATUS_CONNECTED: i32 = 2;

// =============================================================================
// Legacy constants (for backward compatibility during migration)
// =============================================================================

/// Default heartbeat interval in milliseconds (legacy, use HEARTBEAT_INTERVAL_SECONDS instead)
#[deprecated(note = "Use HEARTBEAT_INTERVAL_SECONDS instead")]
pub const HEARTBEAT_INTERVAL_MS: u64 = 5000;

// =============================================================================
// EA Type Enum
// =============================================================================

/// EA type identifier for heartbeat customization
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EaType {
    Master,
    Slave,
}

impl EaType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EaType::Master => "Master",
            EaType::Slave => "Slave",
        }
    }
}

// =============================================================================
// Heartbeat Parameters
// =============================================================================

/// Parameters for heartbeat message customization
#[derive(Clone)]
pub struct HeartbeatParams {
    pub balance: f64,
    pub equity: f64,
    pub version: String,
    pub account_number: i64,
    pub account_name: String,
    pub leverage: i64,
}

impl HeartbeatParams {
    /// Create default parameters for Master EA
    pub fn master_default() -> Self {
        Self {
            balance: 50000.0,
            equity: 50000.0,
            version: "test-master-1.0.0".to_string(),
            account_number: 12345,
            account_name: "MasterTestAccount".to_string(),
            leverage: 500,
        }
    }

    /// Create default parameters for Slave EA
    pub fn slave_default() -> Self {
        Self {
            balance: 10000.0,
            equity: 10000.0,
            version: "test-slave-1.0.0".to_string(),
            account_number: 54321,
            account_name: "SlaveTestAccount".to_string(),
            leverage: 100,
        }
    }
}
