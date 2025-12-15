// Location: mt-bridge/src/traits.rs
// Purpose: Trait hierarchy for configuration messages
// Why: Provides polymorphic interface for Master and Slave config messages

use crate::types::{MasterConfigMessage, SlaveConfigMessage, SymbolMapping, TradeFilters};
use serde::Serialize;

// ============================================================================
// Trait Hierarchy for Config Messages
// ============================================================================

/// Base trait for all configuration messages
/// Provides common interface for account identification, versioning, and symbol transformation
pub trait ConfigMessage: Serialize {
    /// Get the account ID (used as ZMQ topic)
    fn account_id(&self) -> &str;

    /// Get the configuration version number
    fn config_version(&self) -> u32;

    /// Get the Unix timestamp in milliseconds
    fn timestamp(&self) -> i64;

    /// Get the ZMQ topic for pub/sub
    fn zmq_topic(&self) -> String {
        format!("config/{}", self.account_id())
    }

    /// Get the symbol prefix (common to both Master and Slave)
    fn symbol_prefix(&self) -> Option<&str>;

    /// Get the symbol suffix (common to both Master and Slave)
    fn symbol_suffix(&self) -> Option<&str>;
}

/// Master EA configuration trait
/// Extends ConfigMessage with Master-specific functionality
pub trait MasterConfig: ConfigMessage {
    // Currently no Master-specific methods beyond ConfigMessage
    // Reserved for future Master-specific functionality
}

/// Slave EA configuration trait
/// Extends ConfigMessage with Slave-specific functionality
pub trait SlaveConfig: ConfigMessage {
    /// Get the master account ID this slave is copying from
    fn master_account(&self) -> &str;

    /// Get the connection status (0=DISABLED, 1=ENABLED, 2=CONNECTED)
    fn status(&self) -> i32;

    /// Get the lot multiplier
    fn lot_multiplier(&self) -> Option<f64>;

    /// Check if trades should be reversed
    fn reverse_trade(&self) -> bool;

    /// Get the symbol mappings
    fn symbol_mappings(&self) -> &[SymbolMapping];

    /// Get the trade filters
    fn filters(&self) -> &TradeFilters;
}

// ============================================================================
// Trait Implementations
// ============================================================================

// MasterConfigMessage implementations
impl ConfigMessage for MasterConfigMessage {
    fn account_id(&self) -> &str {
        &self.account_id
    }

    fn config_version(&self) -> u32 {
        self.config_version
    }

    fn timestamp(&self) -> i64 {
        self.timestamp
    }

    fn symbol_prefix(&self) -> Option<&str> {
        self.symbol_prefix.as_deref()
    }

    fn symbol_suffix(&self) -> Option<&str> {
        self.symbol_suffix.as_deref()
    }
}

impl MasterConfig for MasterConfigMessage {
    // No Master-specific methods yet
}

// SlaveConfigMessage implementations
impl ConfigMessage for SlaveConfigMessage {
    fn account_id(&self) -> &str {
        &self.account_id
    }

    fn config_version(&self) -> u32 {
        self.config_version
    }

    fn timestamp(&self) -> i64 {
        self.timestamp
    }

    fn symbol_prefix(&self) -> Option<&str> {
        self.symbol_prefix.as_deref()
    }

    fn symbol_suffix(&self) -> Option<&str> {
        self.symbol_suffix.as_deref()
    }
}

impl SlaveConfig for SlaveConfigMessage {
    fn master_account(&self) -> &str {
        &self.master_account
    }

    fn status(&self) -> i32 {
        self.status
    }

    fn lot_multiplier(&self) -> Option<f64> {
        self.lot_multiplier
    }

    fn reverse_trade(&self) -> bool {
        self.reverse_trade
    }

    fn symbol_mappings(&self) -> &[SymbolMapping] {
        &self.symbol_mappings
    }

    fn filters(&self) -> &TradeFilters {
        &self.filters
    }
}
