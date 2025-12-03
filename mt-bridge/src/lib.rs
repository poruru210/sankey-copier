// Top-level modules
pub mod ffi;
pub mod ffi_helpers;
pub mod msgpack;
pub mod traits;
pub mod types;
pub mod victoria_logs;

#[cfg(test)]
mod symbol_filter_tests;

// Re-export message types for use in relay-server
pub use types::{
    HeartbeatMessage, LotCalculationMode, MasterConfigMessage, PositionInfo,
    PositionSnapshotMessage, RequestConfigMessage, SlaveConfigMessage, SymbolMapping, SyncMode,
    SyncRequestMessage, TradeFilters, TradeSignalMessage, UnregisterMessage, VLogsConfigMessage,
    WarningCode,
};

// Re-export traits for polymorphic config handling
pub use traits::{ConfigMessage, MasterConfig, SlaveConfig};

// Re-export GlobalConfigMessage from msgpack (if it exists there)
pub use msgpack::GlobalConfigMessage;
