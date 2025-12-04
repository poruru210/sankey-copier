// e2e-tests/src/helpers.rs
//
// Shared helper functions for E2E tests.
// These functions handle common test setup patterns like DB seeding,
// EA registration, and status management.
//
// DESIGN PRINCIPLE: All status changes MUST go through the Status Engine.
// The Status Engine calculates status from:
// - Intent: User's web UI toggle (enabled/disabled)
// - ConnectionSnapshot: EA's heartbeat status (online/offline)
//
// DO NOT directly manipulate runtime_status in the database.

use anyhow::Result;
use sankey_copier_relay_server::db::Database;
use sankey_copier_relay_server::models::{
    LotCalculationMode, MasterSettings, SlaveSettings, SyncMode, TradeFilters,
};
use tokio::time::{sleep, Duration};

use crate::{MasterEaSimulator, SlaveEaSimulator};

// =============================================================================
// Constants
// =============================================================================

/// Status: No config received yet (initial state before any server response)
pub const STATUS_NO_CONFIG: i32 = -1;

/// Status: Member is disabled (Web UI toggle OFF or EA offline)
pub const STATUS_DISABLED: i32 = 0;

/// Status: Member is enabled but not fully connected (waiting for Master)
pub const STATUS_ENABLED: i32 = 1;

/// Status: Member is fully connected (Slave enabled + online + Master CONNECTED)
pub const STATUS_CONNECTED: i32 = 2;

// =============================================================================
// Helper Functions - Settings
// =============================================================================

/// Create default slave settings for testing
pub fn default_test_slave_settings() -> SlaveSettings {
    SlaveSettings {
        lot_calculation_mode: LotCalculationMode::Multiplier,
        lot_multiplier: Some(1.0),
        reverse_trade: false,
        symbol_prefix: None,
        symbol_suffix: None,
        symbol_mappings: vec![],
        filters: TradeFilters::default(),
        config_version: 0,
        source_lot_min: None,
        source_lot_max: None,
        sync_mode: SyncMode::Skip,
        limit_order_expiry_min: None,
        market_sync_max_pips: None,
        max_slippage: None,
        copy_pending_orders: false,
        max_retries: 3,
        max_signal_delay_ms: 5000,
        use_pending_order_for_delayed: false,
    }
}

// =============================================================================
// Helper Functions - DB Setup (Intent Only)
// =============================================================================

/// Enable a member's Web UI toggle (Intent only, no runtime status change)
/// 
/// This sets the "enabled" flag in the database, which is the Intent.
/// The actual runtime status is calculated by the Status Engine based on
/// Intent + ConnectionSnapshot (heartbeat).
pub async fn enable_member_intent(
    db: &Database,
    master_account: &str,
    slave_account: &str,
) -> Result<()> {
    db.update_member_enabled_flag(master_account, slave_account, true)
        .await?;
    Ok(())
}

/// Disable a member's Web UI toggle (Intent only)
pub async fn disable_member_intent(
    db: &Database,
    master_account: &str,
    slave_account: &str,
) -> Result<()> {
    db.update_member_enabled_flag(master_account, slave_account, false)
        .await?;
    Ok(())
}

/// Setup basic test scenario with master and slaves in DB
/// 
/// Creates trade group, enables master, and adds all slaves with given settings.
/// Slaves are added with DISABLED status and disabled intent.
/// 
/// To achieve CONNECTED status:
/// 1. Call `enable_member_intent()` to enable slave's Web UI toggle
/// 2. Start Master EA with `master.start()` to send heartbeats
/// 3. Start Slave EA with `slave.start()` to send heartbeats
/// 4. Wait for Status Engine to calculate CONNECTED
pub async fn setup_test_db(
    db: &Database,
    master_account: &str,
    slave_accounts: &[&str],
    slave_settings_fn: impl Fn(usize) -> SlaveSettings,
) -> Result<()> {
    // Create trade group for master
    db.create_trade_group(master_account).await?;

    // Enable Master (web_ui switch ON)
    let master_settings = MasterSettings {
        enabled: true,
        ..Default::default()
    };
    db.update_master_settings(master_account, master_settings)
        .await?;

    // Add slaves with settings (initially DISABLED, intent OFF)
    for (i, slave_account) in slave_accounts.iter().enumerate() {
        let settings = slave_settings_fn(i);
        db.add_member(master_account, slave_account, settings, STATUS_DISABLED)
            .await?;
    }

    Ok(())
}

/// Setup test scenario and enable all slaves' intent
/// 
/// This is a convenience function that sets up the DB and enables
/// all slaves' Web UI toggles. The EAs still need to be started
/// to achieve CONNECTED status.
pub async fn setup_test_scenario_with_enabled_slaves(
    db: &Database,
    master_account: &str,
    slave_accounts: &[&str],
    slave_settings_fn: impl Fn(usize) -> SlaveSettings,
) -> Result<()> {
    // Setup basic DB
    setup_test_db(db, master_account, slave_accounts, slave_settings_fn).await?;

    // Enable all slaves' intent
    for slave_account in slave_accounts {
        enable_member_intent(db, master_account, slave_account).await?;
    }

    Ok(())
}

/// Setup test scenario with slaves ready for trade copying
/// 
/// This is a complete setup that:
/// 1. Creates trade group and master in DB
/// 2. Adds all slaves with settings and ENABLED intent
/// 
/// After calling this, start EAs with `start_eas_and_wait_for_ready()` to
/// achieve CONNECTED status.
pub async fn setup_test_scenario(
    db: &Database,
    master_account: &str,
    slave_accounts: &[&str],
    slave_settings_fn: impl Fn(usize) -> SlaveSettings,
) -> Result<()> {
    setup_test_scenario_with_enabled_slaves(db, master_account, slave_accounts, slave_settings_fn)
        .await
}

// =============================================================================
// Helper Functions - EA Lifecycle Management
// =============================================================================

/// Start Master EA and wait for it to become ready
/// 
/// This function:
/// 1. Starts the Master EA (sends initial heartbeat + background thread)
/// 2. Waits for the specified duration for the connection to establish
/// 
/// Returns Ok(()) when the Master EA is online and ready.
pub async fn start_master_and_wait(
    master: &mut MasterEaSimulator,
    wait_ms: u64,
) -> Result<()> {
    master.start()?;
    sleep(Duration::from_millis(wait_ms)).await;
    Ok(())
}

/// Start Slave EA and wait for Config reception (if DB has connection info)
/// 
/// This function:
/// 1. Starts the Slave EA (OnTimer loop starts automatically)
/// 2. Waits for the Slave to receive a SlaveConfigMessage from the server
/// 3. Returns the received config, or None if timeout
/// 
/// The Slave is considered "ready" when it receives its first config.
/// This ensures the Status Engine has processed the heartbeat and
/// calculated the runtime status.
/// 
/// Note: With MQL5-conformant implementation, config reception happens
/// automatically via the OnTimer loop. This function uses wait_for_status
/// to detect when config has been received.
pub async fn start_slave_and_wait_for_config(
    slave: &mut SlaveEaSimulator,
    timeout_ms: i32,
) -> Result<Option<crate::SlaveConfig>> {
    slave.start()?;
    
    // Wait for any status (config reception)
    // Use STATUS_DISABLED as minimum - any status >= -1 means config received
    let config = slave.wait_for_status(STATUS_DISABLED, timeout_ms)?;
    Ok(config)
}

/// Start Slave EA and wait for a specific status
/// 
/// This function:
/// 1. Starts the Slave EA (sends initial heartbeat + background thread)
/// 2. Waits to receive a SlaveConfigMessage with the expected status
/// 3. Returns the config if status matches, or None if timeout
pub async fn start_slave_and_wait_for_status(
    slave: &mut SlaveEaSimulator,
    expected_status: i32,
    timeout_ms: i32,
) -> Result<Option<crate::SlaveConfig>> {
    slave.start()?;
    
    // Wait for config with expected status
    let config = slave.wait_for_status(expected_status, timeout_ms)?;
    Ok(config)
}

/// Start all EAs and wait for Slaves to receive their configs
/// 
/// This function:
/// 1. Starts the Master EA first (so it's online when Slaves connect)
/// 2. Starts all Slave EAs (OnTimer loops start automatically)
/// 3. Waits for each Slave to receive a config message
/// 
/// This provides a reliable way to know when all EAs are ready:
/// - Master is online and registered with the server
/// - Each Slave has received its config (Status Engine has processed)
/// 
/// Returns a Vec of received configs for each slave.
/// 
/// Note: With MQL5-conformant implementation, config reception happens
/// automatically via the OnTimer loop.
pub async fn start_eas_and_wait_for_ready(
    master: &mut MasterEaSimulator,
    slaves: &mut [&mut SlaveEaSimulator],
    timeout_ms: i32,
) -> Result<Vec<Option<crate::SlaveConfig>>> {
    // Start Master first (so it's online when Slaves connect)
    master.start()?;

    // Give Master time to register with server
    sleep(Duration::from_millis(200)).await;

    // Start all Slaves and wait for config reception
    let mut configs = Vec::with_capacity(slaves.len());
    for slave in slaves.iter_mut() {
        slave.start()?;
        // Wait for any status (config reception)
        let config = slave.wait_for_status(STATUS_DISABLED, timeout_ms)?;
        configs.push(config);
    }

    Ok(configs)
}

/// Start all EAs and wait for all Slaves to reach CONNECTED status
/// 
/// This is the most reliable way to ensure all EAs are fully connected
/// and ready for trade copying.
/// 
/// Returns a Vec of received configs (all with CONNECTED status).
/// Returns error if any Slave fails to reach CONNECTED within timeout.
pub async fn start_eas_and_wait_for_connected(
    master: &mut MasterEaSimulator,
    slaves: &mut [&mut SlaveEaSimulator],
    timeout_ms: i32,
) -> Result<Vec<crate::SlaveConfig>> {
    // Start Master first
    master.start()?;
    sleep(Duration::from_millis(200)).await;

    // Start all Slaves and wait for CONNECTED status
    let mut configs = Vec::with_capacity(slaves.len());
    for slave in slaves.iter_mut() {
        slave.start()?;
        match slave.wait_for_status(STATUS_CONNECTED, timeout_ms)? {
            Some(config) => configs.push(config),
            None => anyhow::bail!(
                "Slave {} failed to reach CONNECTED status within {}ms",
                slave.account_id(),
                timeout_ms
            ),
        }
    }

    Ok(configs)
}

/// Start all EAs with default wait time (legacy compatibility)
/// 
/// This is kept for backwards compatibility. New tests should use
/// `start_eas_and_wait_for_ready()` or `start_eas_and_wait_for_connected()`.
pub async fn start_and_wait_for_connection(
    master: &mut MasterEaSimulator,
    slaves: &mut [&mut SlaveEaSimulator],
    wait_ms: u64,
) -> Result<()> {
    // Start Master first (so it's online when Slaves connect)
    master.start()?;

    // Give Master time to register with server
    sleep(Duration::from_millis(200)).await;

    // Start all Slaves
    for slave in slaves.iter_mut() {
        slave.start()?;
    }

    // Wait for Status Engine to process and calculate status
    sleep(Duration::from_millis(wait_ms)).await;

    Ok(())
}

/// Start all EAs with default wait time (500ms)
pub async fn start_and_wait_for_connection_default(
    master: &mut MasterEaSimulator,
    slaves: &mut [&mut SlaveEaSimulator],
) -> Result<()> {
    start_and_wait_for_connection(master, slaves, 500).await
}
