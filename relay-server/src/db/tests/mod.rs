//! Test utilities module
//!
//! Shared test utilities and helper functions for database tests

use crate::db::Database;
use crate::models::{CopySettings, SlaveSettings};
use sankey_copier_zmq::TradeFilters;

pub(crate) async fn create_test_db() -> Database {
    Database::new("sqlite::memory:").await.unwrap()
}

pub(crate) fn create_test_settings() -> CopySettings {
    CopySettings {
        id: 0,
        status: 2, // STATUS_CONNECTED
        master_account: "MASTER_001".to_string(),
        slave_account: "SLAVE_001".to_string(),
        lot_multiplier: Some(1.5),
        reverse_trade: false,
        symbol_prefix: None,
        symbol_suffix: None,
        symbol_mappings: vec![],
        filters: TradeFilters {
            allowed_symbols: None,
            blocked_symbols: None,
            allowed_magic_numbers: None,
            blocked_magic_numbers: None,
        },
    }
}

pub(crate) fn create_test_slave_settings() -> SlaveSettings {
    SlaveSettings {
        config_version: 1,
        symbol_prefix: None,
        symbol_suffix: None,
        lot_multiplier: Some(1.5),
        reverse_trade: false,
        symbol_mappings: vec![],
        filters: TradeFilters {
            allowed_symbols: None,
            blocked_symbols: None,
            allowed_magic_numbers: None,
            blocked_magic_numbers: None,
        },
    }
}

// Test submodules
mod config_distribution_tests;
