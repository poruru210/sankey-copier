// e2e-tests/src/sandbox.rs
//
// TestSandbox: The unified environment manager for E2E testing.
//
// This module provides a "Sandbox" abstraction that manages the complete lifecycle
// of a test environment, including:
// 1. Isolated Relay Server (temp dir, DB, dynamic ports)
// 2. Factory methods for Master/Slave EA simulators
// 3. Automatic cleanup of all resources on Drop
//
// DESIGN GOALS:
// - Parallel Execution: Every Sandbox instance is completely isolated.
// - Ease of Use: Simple API to spawn EAs without worrying about ports/config.
// - Safety: Robust cleanup to prevent zombie processes and resource leaks.

use anyhow::{Context, Result};

use crate::adapters::infrastructure::process::RelayServerProcess;
use crate::application::simulators::master::MasterEaSimulator;
use crate::application::simulators::slave::SlaveEaSimulator;

/// The TestSandbox represents a complete, isolated testing environment.
pub struct TestSandbox {
    /// The underlying relay server process.
    /// Manages the temp directory, DB, and actual binary process.
    server: RelayServerProcess,
}

impl TestSandbox {
    /// Create a new Sandbox environment.
    ///
    /// This will:
    /// 1. Create a temporary directory.
    /// 2. Initialize a unique database.
    /// 3. Start the relay-server on random available ports.
    /// 4. Wait for the server to become ready.
    pub fn new() -> Result<Self> {
        let server =
            RelayServerProcess::start().context("Failed to start relay server in sandbox")?;
        Ok(Self { server })
    }

    /// Create a new Master EA Simulator in this sandbox.
    ///
    /// # Arguments
    /// * `account_id` - Unique identifier for this Master EA (e.g., "master-01").
    /// * `is_trade_allowed` - Initial auto-trading state (TERMINAL_TRADE_ALLOWED).
    pub fn create_master(
        &self,
        account_id: &str,
        is_trade_allowed: bool,
    ) -> Result<MasterEaSimulator> {
        // Master connects to PULL (for commands) and PUB (for config/sync)
        let ini_path = self.server.ini_path();

        let master = MasterEaSimulator::new(&ini_path, account_id, is_trade_allowed)
            .context("Failed to create Master EA simulator")?;

        // Note: We don't automatically call master.start() here to give the caller
        // a chance to configure it before the loop starts.

        Ok(master)
    }

    /// Create a new Slave EA Simulator in this sandbox.
    ///
    /// # Arguments
    /// * `account_id` - Unique identifier for this Slave EA (e.g., "slave-01").
    /// * `master_account_id` - The Master Account ID to subscribe to.
    /// * `is_trade_allowed` - Initial auto-trading state (TERMINAL_TRADE_ALLOWED).
    pub fn create_slave(
        &self,
        account_id: &str,
        master_account_id: &str,
        is_trade_allowed: bool,
    ) -> Result<SlaveEaSimulator> {
        let ini_path = self.server.ini_path();

        let slave =
            SlaveEaSimulator::new(&ini_path, account_id, master_account_id, is_trade_allowed)
                .context("Failed to create Slave EA simulator")?;

        Ok(slave)
    }

    /// Explicitly create a TradeGroup (Master Resource) via API.
    /// Required because auto-creation is disabled.
    pub async fn create_trade_group(&self, master_account_id: &str) -> anyhow::Result<()> {
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .context("Failed to build HTTP client")?;

        let url = format!("{}/api/trade-groups", self.server.http_base_url());

        let response = client
            .post(&url)
            .json(&serde_json::json!({
                "id": master_account_id,
                "master_settings": {
                    "enabled": true,
                    "config_version": 1
                },
                "members": []
            }))
            .send()
            .await
            .context("Failed to send create_trade_group request")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to create TradeGroup (Status: {}): {}", status, text);
        }

        Ok(())
    }

    /// Access the underlying server process if needed (e.g. to inspect DB path)
    pub fn server(&self) -> &RelayServerProcess {
        &self.server
    }
}

// Drop is handled automatically:
// - server (RelayServerProcess) implements Drop, which kills the child process.
// - Created EAs are owned by the caller, so they drop when the caller's scope ends.
