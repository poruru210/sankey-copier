// e2e-tests/src/lib.rs
//
// Shared EA Simulator implementations for E2E testing.
// Refactored to Hexagonal Architecture.

pub mod adapters;
pub mod application;
pub mod domain;

// Re-exports for convenience and backward compatibility
pub use crate::adapters::infrastructure::config::EaIniConfig;
pub use crate::adapters::infrastructure::ffi::helpers;
pub use crate::adapters::infrastructure::process::RelayServerProcess;
pub use crate::application::runner::PlatformRunner;
pub use crate::application::sandbox::TestSandbox;
pub use crate::application::simulators::master::MasterEaSimulator;
pub use crate::application::simulators::slave::SlaveEaSimulator;
pub use crate::domain::simulators::EaSimulatorBase;

pub use crate::domain::models::{
    EaType, GlobalConfigMessage, Heartbeat, HeartbeatParams, MasterConfigMessage, OrderType,
    PositionInfo, PositionSnapshotMessage, RegisterMessage, RequestConfigMessage, SlaveConfig,
    SymbolMapping, SyncMode, SyncRequestMessage, TradeAction, TradeFilters, TradeSignal,
    UnregisterMessage, BUFFER_SIZE, HEARTBEAT_INTERVAL_SECONDS, ONTIMER_INTERVAL_MS,
    STATUS_CONNECTED, STATUS_DISABLED, STATUS_ENABLED, STATUS_NO_CONFIG, TOPIC_BUFFER_SIZE,
};
