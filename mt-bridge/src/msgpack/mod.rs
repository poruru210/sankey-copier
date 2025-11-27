// Location: mt-bridge/src/msgpack/mod.rs
// Purpose: Module definition and public API exports for MessagePack functionality
// Why: Provides a clean public interface while organizing code into focused modules

// Module declarations
mod ffi;
mod helpers;
mod serialization;
mod traits;
mod types;

#[cfg(test)]
mod tests;

// Re-export public types for external use
pub use types::{
    HeartbeatMessage, LotCalculationMode, MasterConfigMessage, PositionInfo,
    PositionSnapshotMessage, RequestConfigMessage, SlaveConfigMessage, SymbolMapping, SyncMode,
    SyncRequestMessage, TradeFilters, TradeSignalMessage, UnregisterMessage,
};

// Re-export traits for polymorphic config handling
pub use traits::{ConfigMessage, MasterConfig, SlaveConfig};

// Re-export FFI functions (already #[no_mangle] pub extern "C")
pub use ffi::{
    free_string, master_config_free, master_config_get_int, master_config_get_string,
    parse_master_config, parse_slave_config, parse_trade_signal, slave_config_free,
    slave_config_get_bool, slave_config_get_double, slave_config_get_int, slave_config_get_string,
    slave_config_get_symbol_mapping_source, slave_config_get_symbol_mapping_target,
    slave_config_get_symbol_mappings_count, trade_signal_free, trade_signal_get_double,
    trade_signal_get_int, trade_signal_get_string,
};

// Re-export serialization FFI functions
pub use serialization::{
    copy_serialized_buffer, get_serialized_buffer, serialize_heartbeat, serialize_request_config,
    serialize_trade_signal, serialize_unregister,
};
