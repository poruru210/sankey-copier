// Top-level modules
pub mod constants;
pub mod ea_state;
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
    PositionSnapshotMessage, RegisterMessage, RequestConfigMessage, SlaveConfigMessage,
    SymbolMapping, SyncMode, SyncRequestMessage, TradeFilters, TradeSignalMessage,
    UnregisterMessage, VLogsConfigMessage, WarningCode,
};

// Re-export traits for polymorphic config handling
pub use traits::{ConfigMessage, MasterConfig, SlaveConfig};

// Re-export EA state management
pub use ea_state::EaState;

// Re-export GlobalConfigMessage from msgpack (if it exists there)
pub use msgpack::GlobalConfigMessage;

// Re-export constants for protocol consistency
pub use constants::{
    build_config_topic, build_sync_topic, build_trade_topic, OrderType, TradeAction,
    MSG_TYPE_HEARTBEAT, MSG_TYPE_POSITION_SNAPSHOT, MSG_TYPE_REGISTER, MSG_TYPE_REQUEST_CONFIG,
    MSG_TYPE_SYNC_REQUEST, MSG_TYPE_TRADE_SIGNAL, MSG_TYPE_UNREGISTER, STATUS_CONNECTED,
    STATUS_DISABLED, STATUS_ENABLED, STATUS_NO_CONFIG, TOPIC_CONFIG_PREFIX, TOPIC_GLOBAL_CONFIG,
    TOPIC_SYNC_PREFIX, TOPIC_TRADE_PREFIX,
};
