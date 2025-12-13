// e2e-tests/src/base.rs
//
// EaSimulatorBase - Common EA Simulator logic.
//
// Note: ZMQ management is now handled by `EaContext` via FFI.
// This struct now mainly holds shared state and configuration.

use anyhow::Result;
use chrono::Utc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::types::{EaType, HeartbeatParams};

/// Base structure for EA Simulator state.
pub struct EaSimulatorBase {
    account_id: String,
    pub(crate) ea_type: EaType,
    /// Auto-trading state (simulates TERMINAL_TRADE_ALLOWED in MQL5)
    /// MQL5: g_last_trade_allowed initialized to false
    is_trade_allowed: Arc<AtomicBool>,
    /// Flag to signal background thread to stop
    pub(crate) shutdown_flag: Arc<AtomicBool>,
    /// Heartbeat parameters (balance, equity, version, etc.)
    pub(crate) heartbeat_params: HeartbeatParams,
}

impl EaSimulatorBase {
    pub fn new_without_zmq(account_id: &str, ea_type: EaType) -> Result<Self> {
        let heartbeat_params = match ea_type {
            EaType::Master => HeartbeatParams::master_default(),
            EaType::Slave => HeartbeatParams::slave_default(),
        };

        Ok(Self {
            account_id: account_id.to_string(),
            ea_type,
            is_trade_allowed: Arc::new(AtomicBool::new(false)),
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            heartbeat_params,
        })
    }

    /// Get account ID
    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    /// Get is_trade_allowed Arc for sharing with OnTimer thread
    pub(crate) fn is_trade_allowed_arc(&self) -> Arc<AtomicBool> {
        self.is_trade_allowed.clone()
    }

    /// Set is_trade_allowed state (simulates MT4/MT5 auto-trading toggle)
    ///
    /// In MQL5, this corresponds to TerminalInfoInteger(TERMINAL_TRADE_ALLOWED).
    /// Changing this value triggers a heartbeat on the next OnTimer cycle.
    pub fn set_trade_allowed(&self, allowed: bool) {
        self.is_trade_allowed.store(allowed, Ordering::SeqCst);
    }

    /// Get is_trade_allowed state
    pub fn is_trade_allowed(&self) -> bool {
        self.is_trade_allowed.load(Ordering::SeqCst)
    }
}

impl Drop for EaSimulatorBase {
    fn drop(&mut self) {
        // Signal any background threads to stop
        self.shutdown_flag.store(true, Ordering::SeqCst);
    }
}
