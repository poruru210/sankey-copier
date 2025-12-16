pub mod connection;
pub mod global_settings;
pub mod mt_installation;
pub mod slave_config;
pub mod snapshot;
pub mod symbol_converter;
pub mod trade_group;
pub mod trade_group_member;

// Re-export specific items for easier access
pub use connection::*;
pub use global_settings::*;
pub use mt_installation::*;
pub use slave_config::*;
pub use snapshot::*;
pub use symbol_converter::*;
pub use trade_group::*;
pub use trade_group_member::*;

// mod settings_conversion_tests;

// Re-export shared types from DLL
// These are external to our domain but used within it.
// We might want to wrap them eventually, but re-exporting here works for now.
pub use sankey_copier_zmq::{
    HeartbeatMessage, MasterConfigMessage, OrderType, PositionSnapshotMessage, RegisterMessage,
    RequestConfigMessage, SlaveConfigMessage, SymbolMapping, SyncRequestMessage, TradeAction,
    TradeFilters, TradeSignal, UnregisterMessage, WarningCode, STATUS_CONNECTED, STATUS_DISABLED,
    STATUS_ENABLED, STATUS_NO_CONFIG,
};
