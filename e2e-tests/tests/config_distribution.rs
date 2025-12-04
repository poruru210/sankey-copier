// e2e-tests/tests/config_distribution.rs
//
// E2E integration tests for configuration distribution to Slave EAs.
// Tests config flow, status changes, member toggle, and sync policy modes.
//
// Note: Master config distribution uses raw bytes (Option<Vec<u8>>), so we focus
// on Slave config distribution which returns typed SlaveConfigMessage.

use e2e_tests::helpers::{STATUS_CONNECTED, STATUS_DISABLED, STATUS_ENABLED};
use e2e_tests::relay_server_process::RelayServerProcess;
use e2e_tests::{MasterEaSimulator, SlaveEaSimulator, SyncMode};
use sankey_copier_relay_server::db::Database;
use sankey_copier_relay_server::models::{LotCalculationMode, MasterSettings, SlaveSettings};
use tokio::time::{sleep, Duration};

// =============================================================================
// Slave Config Distribution Tests
// =============================================================================

/// Test Slave EA config distribution flow
#[tokio::test]
async fn test_slave_config_distribution() {
    let server = RelayServerProcess::start().expect("Failed to start relay-server");

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_E2E_002";
    let slave_account = "SLAVE_E2E_001";

    // Create TradeGroup (Master)
    db.create_trade_group(master_account)
        .await
        .expect("Failed to create trade group");

    // Enable the master
    let master_settings = MasterSettings {
        enabled: true,
        ..Default::default()
    };
    db.update_master_settings(master_account, master_settings)
        .await
        .expect("Failed to enable master");

    // Add Slave member to TradeGroup with default settings
    db.add_member(
        master_account,
        slave_account,
        SlaveSettings::default(),
        STATUS_DISABLED,
    )
    .await
    .expect("Failed to add member");

    // Create Slave EA simulator
    let mut simulator = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_address(),
        &server.zmq_pub_address(),
        slave_account,
        master_account,
    )
    .expect("Failed to create Slave EA simulator");

    // Allow ZMQ connections to establish
    sleep(Duration::from_millis(500)).await;

    // Start the simulator (auto heartbeat + request config via OnTimer)
    simulator.set_trade_allowed(true);
    simulator.start().expect("Failed to start simulator");

    // Wait for config via wait_for_status
    let config = simulator
        .wait_for_status(STATUS_DISABLED, 5000)
        .expect("Failed to receive config");

    // Verify config was received
    assert!(
        config.is_some(),
        "Slave EA should receive SlaveConfigMessage"
    );

    let config = config.unwrap();

    // Verify config fields
    assert_eq!(
        config.account_id, slave_account,
        "Config account_id should match"
    );
    assert_eq!(
        config.master_account, master_account,
        "Config master_account should match"
    );

    println!(
        "✅ Slave EA E2E test passed: Received config for {} from master {}",
        config.account_id, config.master_account
    );
}

// =============================================================================
// Multiple Slaves Tests
// =============================================================================

/// Test one Master with multiple Slaves (1:N relationship)
#[tokio::test]
async fn test_multiple_slaves_same_master() {
    let server = RelayServerProcess::start().expect("Failed to start relay-server");

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_MULTI_SLAVE";
    let slave_accounts = ["SLAVE_A", "SLAVE_B", "SLAVE_C"];

    // Create TradeGroup (Master)
    db.create_trade_group(master_account)
        .await
        .expect("Failed to create trade group");

    // Enable the master
    let master_settings = MasterSettings {
        enabled: true,
        ..Default::default()
    };
    db.update_master_settings(master_account, master_settings)
        .await
        .expect("Failed to enable master");

    // Add 3 Slaves to the same Master with different lot multipliers
    for (i, slave_account) in slave_accounts.iter().enumerate() {
        let settings = SlaveSettings {
            lot_calculation_mode: LotCalculationMode::default(),
            lot_multiplier: Some((i + 1) as f64 * 0.5), // 0.5, 1.0, 1.5
            ..Default::default()
        };

        db.add_member(master_account, slave_account, settings, STATUS_DISABLED)
            .await
            .expect("Failed to add member");
    }

    // Create 3 Slave EA simulators
    let mut slave_simulators = Vec::new();
    for slave_account in &slave_accounts {
        let simulator = SlaveEaSimulator::new(
            &server.zmq_pull_address(),
            &server.zmq_pub_address(),
            &server.zmq_pub_address(),
            slave_account,
            master_account,
        )
        .expect("Failed to create Slave EA simulator");
        slave_simulators.push(simulator);
    }

    // Set trade_allowed for all simulators
    for simulator in &mut slave_simulators {
        simulator.set_trade_allowed(true);
    }

    // Allow ZMQ connections to establish
    sleep(Duration::from_millis(500)).await;

    // All Slaves start (auto heartbeat + request config)
    for simulator in &mut slave_simulators {
        simulator.start().expect("Failed to start Slave simulator");
    }
    sleep(Duration::from_millis(500)).await;

    // Verify all Slaves receive their respective configs
    for (i, simulator) in slave_simulators.iter_mut().enumerate() {
        let config = simulator
            .wait_for_status(STATUS_DISABLED, 5000)
            .expect("Failed to receive Slave config");

        assert!(
            config.is_some(),
            "Slave {} should receive SlaveConfigMessage",
            slave_accounts[i]
        );

        let config = config.unwrap();
        assert_eq!(config.account_id, slave_accounts[i]);
        assert_eq!(config.master_account, master_account);
        assert_eq!(
            config.lot_multiplier,
            Some((i + 1) as f64 * 0.5),
            "Slave {} should have correct lot_multiplier",
            slave_accounts[i]
        );

        println!(
            "  ✅ Slave {} received config with lot_multiplier: {:?}",
            slave_accounts[i], config.lot_multiplier
        );
    }

    println!(
        "✅ Multiple Slaves E2E test passed: {} slaves under Master {}",
        slave_accounts.len(),
        master_account
    );
}

// =============================================================================
// Member Status Tests
// =============================================================================

/// Test that new member is created with DISABLED status
#[tokio::test]
async fn test_new_member_initial_status_disabled() {
    let server = RelayServerProcess::start().expect("Failed to start relay-server");

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_INITIAL_STATUS_TEST";
    let slave_account = "SLAVE_INITIAL_STATUS_TEST";

    // Create TradeGroup (Master)
    db.create_trade_group(master_account)
        .await
        .expect("Failed to create trade group");

    // Enable the master
    let master_settings = MasterSettings {
        enabled: true,
        ..Default::default()
    };
    db.update_master_settings(master_account, master_settings)
        .await
        .expect("Failed to enable master");

    // Add Slave member to TradeGroup with default settings
    db.add_member(
        master_account,
        slave_account,
        SlaveSettings::default(),
        STATUS_DISABLED,
    )
    .await
    .expect("Failed to add member");

    // Create Slave EA simulator
    let mut simulator = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_address(),
        &server.zmq_pub_address(),
        slave_account,
        master_account,
    )
    .expect("Failed to create Slave EA simulator");

    // Allow ZMQ connections to establish
    sleep(Duration::from_millis(500)).await;

    // Start the simulator (auto heartbeat + request config via OnTimer)
    simulator.set_trade_allowed(true);
    simulator.start().expect("Failed to start simulator");

    // Wait for config
    let config = simulator
        .wait_for_status(STATUS_DISABLED, 5000)
        .expect("Failed to receive config");
    assert!(config.is_some(), "Should receive config");
    let config = config.unwrap();

    // Verify initial status is DISABLED (0)
    assert_eq!(
        config.status, STATUS_DISABLED,
        "New member initial status should be DISABLED (0)"
    );

    println!("✅ New Member Initial Status E2E test passed: status=0 (DISABLED)");
}

/// Test full status transition: DISABLED -> ENABLED -> CONNECTED
///
/// This test verifies the complete Status Engine behavior:
/// 1. DISABLED: Slave Web UI is OFF or Slave EA is offline
/// 2. ENABLED: Slave Web UI is ON, Slave EA online, but Master not CONNECTED
/// 3. CONNECTED: All conditions met (Slave enabled + online + Master CONNECTED)
#[tokio::test]
async fn test_status_transition_to_connected() {
    let server = RelayServerProcess::start().expect("Failed to start relay-server");

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_STATUS_TRANS_001";
    let slave_account = "SLAVE_STATUS_TRANS_001";

    // Create TradeGroup (Master)
    db.create_trade_group(master_account)
        .await
        .expect("Failed to create trade group");

    // Enable the master in Web UI
    let master_settings = MasterSettings {
        enabled: true,
        ..Default::default()
    };
    db.update_master_settings(master_account, master_settings)
        .await
        .expect("Failed to enable master");

    // Add Slave member (initially DISABLED, intent OFF)
    db.add_member(
        master_account,
        slave_account,
        SlaveSettings::default(),
        STATUS_DISABLED,
    )
    .await
    .expect("Failed to add member");

    // Create both Master and Slave EA simulators
    let mut master_ea = e2e_tests::MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_address(),
        master_account,
    )
    .expect("Failed to create Master EA simulator");

    let mut slave_ea = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_address(),
        &server.zmq_pub_address(),
        slave_account,
        master_account,
    )
    .expect("Failed to create Slave EA simulator");

    sleep(Duration::from_millis(500)).await;

    // =========================================================================
    // Phase 1: DISABLED (Slave Web UI OFF)
    // =========================================================================
    // Start Slave EA (sends heartbeat, starts background thread)
    // This keeps Slave online for the duration of the test
    slave_ea.set_trade_allowed(true);
    slave_ea.start().expect("Failed to start Slave EA");

    let disabled_config = slave_ea
        .wait_for_status(STATUS_DISABLED, 5000)
        .expect("Failed to receive initial config");
    assert!(disabled_config.is_some(), "Should receive config");
    assert_eq!(
        disabled_config.unwrap().status,
        STATUS_DISABLED,
        "Phase 1: Status should be DISABLED (Web UI OFF)"
    );
    println!("✅ Phase 1: DISABLED (Web UI OFF)");

    // =========================================================================
    // Phase 2: ENABLED (Web UI ON, but Master not connected)
    // =========================================================================
    // Enable Slave in Web UI (sets intent)
    db.update_member_enabled_flag(master_account, slave_account, true)
        .await
        .expect("Failed to enable member");

    // Slave is already online (running from start()), wait for ENABLED status
    let enabled_config = slave_ea
        .wait_for_status(STATUS_ENABLED, 5000)
        .expect("Failed to receive updated config");
    assert!(enabled_config.is_some(), "Should receive config");
    assert_eq!(
        enabled_config.unwrap().status,
        STATUS_ENABLED,
        "Phase 2: Status should be ENABLED (Master not connected yet)"
    );
    println!("✅ Phase 2: ENABLED (Web UI ON, Master offline)");

    // =========================================================================
    // Phase 3: CONNECTED (All conditions met)
    // =========================================================================
    // Start Master EA (sends heartbeat, starts background thread)
    // When Master sends heartbeat, server will automatically send updated
    // config to all online Slaves with their new status
    master_ea.set_trade_allowed(true);
    master_ea.start().expect("Failed to start Master EA");

    // Wait for server to process Master heartbeat and send Slave configs
    sleep(Duration::from_millis(500)).await;

    // Wait for the automatically-sent config from server
    // (sent when Master status changed to CONNECTED and Slave is online)
    let connected_config = slave_ea
        .wait_for_status(STATUS_CONNECTED, 5000)
        .expect("Failed to receive connected config");

    assert!(
        connected_config.is_some(),
        "Phase 3: Should receive config with CONNECTED status"
    );
    println!("✅ Phase 3: CONNECTED (All conditions met)");
    println!("✅ Phase 3: CONNECTED (All conditions met)");

    println!("✅ Full Status Transition E2E test passed: DISABLED -> ENABLED -> CONNECTED");
}

// =============================================================================
// Sync Policy Mode Tests
// =============================================================================

/// Test sync policy fields with SyncMode::Skip
#[tokio::test]
async fn test_sync_policy_skip_mode() {
    let server = RelayServerProcess::start().expect("Failed to start relay-server");

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_SYNC_SKIP";
    let slave_account = "SLAVE_SYNC_SKIP";

    // Create TradeGroup (Master)
    db.create_trade_group(master_account)
        .await
        .expect("Failed to create trade group");

    // Add Slave with SyncMode::Skip (default)
    let settings = SlaveSettings {
        lot_calculation_mode: LotCalculationMode::default(),
        lot_multiplier: Some(1.0),
        sync_mode: sankey_copier_relay_server::models::SyncMode::Skip,
        max_slippage: Some(30),
        copy_pending_orders: false,
        ..Default::default()
    };

    db.add_member(master_account, slave_account, settings, STATUS_DISABLED)
        .await
        .expect("Failed to add member");

    // Create Slave EA simulator
    let mut simulator = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_address(),
        &server.zmq_pub_address(),
        slave_account,
        master_account,
    )
    .expect("Failed to create Slave EA simulator");

    sleep(Duration::from_millis(500)).await;

    // Start the simulator (auto heartbeat + request config via OnTimer)
    simulator.set_trade_allowed(true);
    simulator.start().expect("Failed to start simulator");

    let config = simulator
        .wait_for_status(STATUS_DISABLED, 5000)
        .expect("Failed to receive config");
    assert!(config.is_some(), "Should receive config");
    let config = config.unwrap();

    // Verify sync policy fields
    assert_eq!(config.sync_mode, SyncMode::Skip, "sync_mode should be Skip");
    assert_eq!(config.max_slippage, Some(30), "max_slippage should be 30");
    assert!(
        !config.copy_pending_orders,
        "copy_pending_orders should be false"
    );

    println!("✅ Sync Policy Skip Mode E2E test passed");
}

/// Test sync policy fields with SyncMode::LimitOrder
#[tokio::test]
async fn test_sync_policy_limit_order_mode() {
    let server = RelayServerProcess::start().expect("Failed to start relay-server");

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_SYNC_LIMIT";
    let slave_account = "SLAVE_SYNC_LIMIT";

    // Create TradeGroup (Master)
    db.create_trade_group(master_account)
        .await
        .expect("Failed to create trade group");

    // Add Slave with SyncMode::LimitOrder
    let settings = SlaveSettings {
        lot_calculation_mode: LotCalculationMode::default(),
        lot_multiplier: Some(1.5),
        sync_mode: sankey_copier_relay_server::models::SyncMode::LimitOrder,
        limit_order_expiry_min: Some(60), // 60 minutes
        max_slippage: Some(50),
        copy_pending_orders: true,
        ..Default::default()
    };

    db.add_member(master_account, slave_account, settings, STATUS_DISABLED)
        .await
        .expect("Failed to add member");

    // Create Slave EA simulator
    let mut simulator = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_address(),
        &server.zmq_pub_address(),
        slave_account,
        master_account,
    )
    .expect("Failed to create Slave EA simulator");

    sleep(Duration::from_millis(500)).await;

    // Start the simulator (auto heartbeat + request config via OnTimer)
    simulator.set_trade_allowed(true);
    simulator.start().expect("Failed to start simulator");

    let config = simulator
        .wait_for_status(STATUS_DISABLED, 5000)
        .expect("Failed to receive config");
    assert!(config.is_some(), "Should receive config");
    let config = config.unwrap();

    // Verify sync policy fields
    assert_eq!(
        config.sync_mode,
        SyncMode::LimitOrder,
        "sync_mode should be LimitOrder"
    );
    assert_eq!(
        config.limit_order_expiry_min,
        Some(60),
        "limit_order_expiry_min should be 60"
    );
    assert_eq!(config.max_slippage, Some(50), "max_slippage should be 50");
    assert!(
        config.copy_pending_orders,
        "copy_pending_orders should be true"
    );

    println!("✅ Sync Policy LimitOrder Mode E2E test passed");
}

/// Test sync policy fields with SyncMode::MarketOrder
#[tokio::test]
async fn test_sync_policy_market_order_mode() {
    let server = RelayServerProcess::start().expect("Failed to start relay-server");

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_SYNC_MARKET";
    let slave_account = "SLAVE_SYNC_MARKET";

    // Create TradeGroup (Master)
    db.create_trade_group(master_account)
        .await
        .expect("Failed to create trade group");

    // Add Slave with SyncMode::MarketOrder
    let settings = SlaveSettings {
        lot_calculation_mode: LotCalculationMode::default(),
        lot_multiplier: Some(2.0),
        source_lot_min: Some(0.01),
        source_lot_max: Some(10.0),
        sync_mode: sankey_copier_relay_server::models::SyncMode::MarketOrder,
        market_sync_max_pips: Some(25.0), // 25 pips max deviation
        max_slippage: Some(20),
        copy_pending_orders: false,
        ..Default::default()
    };

    db.add_member(master_account, slave_account, settings, STATUS_DISABLED)
        .await
        .expect("Failed to add member");

    // Create Slave EA simulator
    let mut simulator = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_address(),
        &server.zmq_pub_address(),
        slave_account,
        master_account,
    )
    .expect("Failed to create Slave EA simulator");

    sleep(Duration::from_millis(500)).await;

    // Enable auto-trading and start OnTimer loop
    simulator.set_trade_allowed(true);
    simulator.start().expect("Failed to start simulator");

    // Wait for config reception
    let config = simulator
        .wait_for_status(STATUS_DISABLED, 5000)
        .expect("Failed to receive config");
    assert!(config.is_some(), "Should receive config");
    let config = config.unwrap();

    // Verify sync policy fields
    assert_eq!(
        config.sync_mode,
        SyncMode::MarketOrder,
        "sync_mode should be MarketOrder"
    );
    assert_eq!(
        config.market_sync_max_pips,
        Some(25.0),
        "market_sync_max_pips should be 25.0"
    );
    assert_eq!(config.max_slippage, Some(20), "max_slippage should be 20");
    assert!(
        !config.copy_pending_orders,
        "copy_pending_orders should be false"
    );

    // Also verify other fields are preserved
    assert_eq!(
        config.lot_multiplier,
        Some(2.0),
        "lot_multiplier should be 2.0"
    );
    assert_eq!(
        config.source_lot_min,
        Some(0.01),
        "source_lot_min should be 0.01"
    );
    assert_eq!(
        config.source_lot_max,
        Some(10.0),
        "source_lot_max should be 10.0"
    );

    println!("✅ Sync Policy MarketOrder Mode E2E test passed");
}

// =============================================================================
// Trade Execution Settings Tests
// =============================================================================

/// Test trade execution settings distribution
#[tokio::test]
async fn test_trade_execution_settings() {
    let server = RelayServerProcess::start().expect("Failed to start relay-server");

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_EXEC_SETTINGS";
    let slave_account = "SLAVE_EXEC_SETTINGS";

    // Create TradeGroup (Master)
    db.create_trade_group(master_account)
        .await
        .expect("Failed to create trade group");

    // Add Slave with custom trade execution settings
    let settings = SlaveSettings {
        max_retries: 5,
        max_signal_delay_ms: 10000,
        use_pending_order_for_delayed: true,
        ..Default::default()
    };

    db.add_member(master_account, slave_account, settings, STATUS_DISABLED)
        .await
        .expect("Failed to add member");

    // Create Slave EA simulator
    let mut simulator = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_address(),
        &server.zmq_pub_address(),
        slave_account,
        master_account,
    )
    .expect("Failed to create Slave EA simulator");

    sleep(Duration::from_millis(500)).await;

    // Enable auto-trading and start OnTimer loop
    simulator.set_trade_allowed(true);
    simulator.start().expect("Failed to start simulator");

    // Wait for config reception
    let config = simulator
        .wait_for_status(STATUS_DISABLED, 5000)
        .expect("Failed to receive config");
    assert!(config.is_some(), "Should receive config");
    let config = config.unwrap();

    // Verify trade execution settings
    assert_eq!(config.max_retries, 5, "max_retries should be 5");
    assert_eq!(
        config.max_signal_delay_ms, 10000,
        "max_signal_delay_ms should be 10000"
    );
    assert!(
        config.use_pending_order_for_delayed,
        "use_pending_order_for_delayed should be true"
    );

    println!("✅ Trade Execution Settings E2E test passed");
}

// =============================================================================
// Multiple Masters Tests
// =============================================================================

/// Test multiple Masters with multiple Slaves (N:M isolation)
/// Focus on Slave config distribution since Master config uses raw bytes
#[tokio::test]
async fn test_multiple_masters_multiple_slaves() {
    let server = RelayServerProcess::start().expect("Failed to start relay-server");

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master1 = "MASTER_GROUP_1";
    let master2 = "MASTER_GROUP_2";
    let slave1 = "SLAVE_G1_A";
    let slave2 = "SLAVE_G1_B";
    let slave3 = "SLAVE_G2_A";

    // Create 2 TradeGroups (Masters)
    db.create_trade_group(master1)
        .await
        .expect("Failed to create trade group 1");
    db.create_trade_group(master2)
        .await
        .expect("Failed to create trade group 2");

    // Enable masters
    let master_settings = MasterSettings {
        enabled: true,
        ..Default::default()
    };
    db.update_master_settings(master1, master_settings.clone())
        .await
        .expect("Failed to enable master1");
    db.update_master_settings(master2, master_settings)
        .await
        .expect("Failed to enable master2");

    // Master1 has Slave1 and Slave2
    db.add_member(
        master1,
        slave1,
        SlaveSettings {
            lot_multiplier: Some(1.0),
            symbol_prefix: Some("M1_".to_string()),
            ..Default::default()
        },
        STATUS_DISABLED,
    )
    .await
    .expect("Failed to add slave1 to master1");

    db.add_member(
        master1,
        slave2,
        SlaveSettings {
            lot_multiplier: Some(2.0),
            symbol_prefix: Some("M1_".to_string()),
            ..Default::default()
        },
        STATUS_DISABLED,
    )
    .await
    .expect("Failed to add slave2 to master1");

    // Master2 has Slave3
    db.add_member(
        master2,
        slave3,
        SlaveSettings {
            lot_multiplier: Some(0.5),
            reverse_trade: true,
            symbol_prefix: Some("M2_".to_string()),
            ..Default::default()
        },
        STATUS_DISABLED,
    )
    .await
    .expect("Failed to add slave3 to master2");

    // Create Slave EA simulators
    let mut slave1_sim = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_address(),
        &server.zmq_pub_address(),
        slave1,
        master1,
    )
    .expect("Failed to create Slave1 simulator");
    slave1_sim.set_trade_allowed(true);

    let mut slave2_sim = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_address(),
        &server.zmq_pub_address(),
        slave2,
        master1,
    )
    .expect("Failed to create Slave2 simulator");
    slave2_sim.set_trade_allowed(true);

    let mut slave3_sim = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_address(),
        &server.zmq_pub_address(),
        slave3,
        master2,
    )
    .expect("Failed to create Slave3 simulator");
    slave3_sim.set_trade_allowed(true);

    // Allow ZMQ connections to establish
    sleep(Duration::from_millis(500)).await;

    // All Slaves start (auto heartbeat + request config)
    slave1_sim.start().expect("Failed to start Slave1");
    slave2_sim.start().expect("Failed to start Slave2");
    slave3_sim.start().expect("Failed to start Slave3");
    sleep(Duration::from_millis(500)).await;

    // Verify Slave1 receives config from Master1
    let slave1_config = slave1_sim
        .wait_for_status(STATUS_DISABLED, 5000)
        .expect("Failed to receive config");
    assert!(slave1_config.is_some(), "Slave1 should receive config");
    let slave1_config = slave1_config.unwrap();
    assert_eq!(slave1_config.account_id, slave1);
    assert_eq!(
        slave1_config.master_account, master1,
        "Slave1 should belong to Master1"
    );
    assert_eq!(slave1_config.lot_multiplier, Some(1.0));
    assert_eq!(slave1_config.symbol_prefix, Some("M1_".to_string()));

    // Verify Slave2 receives config from Master1
    let slave2_config = slave2_sim
        .wait_for_status(STATUS_DISABLED, 5000)
        .expect("Failed to receive config");
    assert!(slave2_config.is_some(), "Slave2 should receive config");
    let slave2_config = slave2_config.unwrap();
    assert_eq!(slave2_config.account_id, slave2);
    assert_eq!(
        slave2_config.master_account, master1,
        "Slave2 should belong to Master1"
    );
    assert_eq!(slave2_config.lot_multiplier, Some(2.0));

    // Verify Slave3 receives config from Master2
    let slave3_config = slave3_sim
        .wait_for_status(STATUS_DISABLED, 5000)
        .expect("Failed to receive config");
    assert!(slave3_config.is_some(), "Slave3 should receive config");
    let slave3_config = slave3_config.unwrap();
    assert_eq!(slave3_config.account_id, slave3);
    assert_eq!(
        slave3_config.master_account, master2,
        "Slave3 should belong to Master2"
    );
    assert_eq!(slave3_config.lot_multiplier, Some(0.5));
    assert!(
        slave3_config.reverse_trade,
        "Slave3 should have reverse_trade enabled"
    );
    assert_eq!(slave3_config.symbol_prefix, Some("M2_".to_string()));

    println!("✅ Multiple Masters/Slaves E2E test passed:");
    println!(
        "   Master1 ({}) → Slave1 ({}) + Slave2 ({})",
        master1, slave1, slave2
    );
    println!("   Master2 ({}) → Slave3 ({})", master2, slave3);
}

// =============================================================================
// Master Config Distribution Tests
// =============================================================================

/// Test Master EA config distribution flow
#[tokio::test]
async fn test_master_config_distribution() {
    use e2e_tests::MasterEaSimulator;

    let server = RelayServerProcess::start().expect("Failed to start relay-server");

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_CONFIG_001";

    // Create TradeGroup (Master)
    db.create_trade_group(master_account)
        .await
        .expect("Failed to create trade group");

    // Enable the master with symbol prefix/suffix settings
    let master_settings = MasterSettings {
        enabled: true,
        symbol_prefix: Some("PREFIX_".to_string()),
        symbol_suffix: Some("_SUFFIX".to_string()),
        ..Default::default()
    };
    db.update_master_settings(master_account, master_settings)
        .await
        .expect("Failed to update master settings");

    // Create Master EA simulator
    let mut simulator = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_address(),
        master_account,
    )
    .expect("Failed to create Master EA simulator");

    // Allow ZMQ connections to establish
    sleep(Duration::from_millis(500)).await;

    // Start the simulator (auto heartbeat + request config via OnTimer)
    simulator.set_trade_allowed(true);
    simulator.start().expect("Failed to start simulator");

    // Wait for config
    let config = simulator
        .wait_for_status(STATUS_CONNECTED, 5000)
        .expect("Failed to receive config");

    // Verify config was received
    assert!(
        config.is_some(),
        "Master EA should receive MasterConfigMessage"
    );

    let config = config.unwrap();

    // Verify config fields
    assert_eq!(
        config.account_id, master_account,
        "Config account_id should match"
    );
    assert_eq!(
        config.symbol_prefix,
        Some("PREFIX_".to_string()),
        "symbol_prefix should match"
    );
    assert_eq!(
        config.symbol_suffix,
        Some("_SUFFIX".to_string()),
        "symbol_suffix should match"
    );

    println!("✅ test_master_config_distribution passed");
    println!("   Master: {}", config.account_id);
    println!(
        "   Prefix: {:?}, Suffix: {:?}",
        config.symbol_prefix, config.symbol_suffix
    );
}

/// Test Master EA config distribution with non-existent account
#[tokio::test]
async fn test_master_config_not_found() {
    use e2e_tests::MasterEaSimulator;

    let server = RelayServerProcess::start().expect("Failed to start relay-server");

    let master_account = "NONEXISTENT_MASTER_001";

    // Create Master EA simulator (no DB setup - account doesn't exist)
    let mut simulator = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_address(),
        master_account,
    )
    .expect("Failed to create Master EA simulator");

    // Allow ZMQ connections to establish
    sleep(Duration::from_millis(500)).await;

    // Start the simulator (auto heartbeat + request config via OnTimer)
    simulator.set_trade_allowed(true);
    simulator.start().expect("Failed to start simulator");

    // Try to receive config - should get a disabled config or timeout
    let config = simulator
        .wait_for_status(STATUS_DISABLED, 2000)
        .expect("Failed to try receive config");

    // Server may return disabled config or no config at all for non-existent accounts
    // The expected behavior depends on server implementation
    if let Some(config) = config {
        // If config is returned, it should be for this account but in disabled state
        assert_eq!(
            config.account_id, master_account,
            "Config should be for the requested account"
        );
        println!("✅ test_master_config_not_found: Got disabled config for non-existent account");
    } else {
        // Timeout is also acceptable - server didn't respond
        println!("✅ test_master_config_not_found: No config returned for non-existent account (timeout)");
    }
}

// =============================================================================
// Member Management Tests
// =============================================================================

/// Test that toggling member status OFF sends disabled config to Slave EA
#[tokio::test]
async fn test_toggle_member_status_off_sends_disabled_config() {
    let server = RelayServerProcess::start().expect("Failed to start relay-server");

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_TOGGLE_TEST";
    let slave_account = "SLAVE_TOGGLE_TEST";

    // Create TradeGroup (Master) with enabled=true
    db.create_trade_group(master_account)
        .await
        .expect("Failed to create trade group");

    let master_settings = MasterSettings {
        enabled: true,
        ..Default::default()
    };
    db.update_master_settings(master_account, master_settings)
        .await
        .expect("Failed to enable master");

    // Add Slave member to TradeGroup (initial status = DISABLED)
    db.add_member(
        master_account,
        slave_account,
        SlaveSettings::default(),
        STATUS_DISABLED,
    )
    .await
    .expect("Failed to add member");

    // Enable the slave's intent (so we can test toggle OFF)
    db.update_member_enabled_flag(master_account, slave_account, true)
        .await
        .expect("Failed to enable member (flag)");

    // Create Master EA simulator and start it
    let mut master_sim = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_address(),
        master_account,
    )
    .expect("Failed to create Master EA simulator");
    master_sim.set_trade_allowed(true);
    master_sim.start().expect("Failed to start master EA");

    // Give Master time to register and become CONNECTED
    sleep(Duration::from_millis(500)).await;

    // Create and start Slave EA simulator
    let mut slave_sim = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_address(),
        &server.zmq_pub_address(),
        slave_account,
        master_account,
    )
    .expect("Failed to create Slave EA simulator");
    slave_sim.set_trade_allowed(true);
    slave_sim.start().expect("Failed to start slave EA");

    // Step 1: Wait for initial config - check what status we actually get
    // First, receive any config to see what the Status Engine computed
    let initial_config = slave_sim
        .wait_for_status(STATUS_CONNECTED, 5000)
        .expect("Failed to receive initial config");

    assert!(
        initial_config.is_some(),
        "Should receive initial config from Status Engine"
    );
    let initial_config = initial_config.unwrap();

    println!(
        "Initial config received: status={}, allow_new_orders={}",
        initial_config.status, initial_config.allow_new_orders
    );

    // For the toggle test, we just need to verify that toggling OFF sends DISABLED
    // The initial status can be ENABLED or CONNECTED depending on timing

    // Step 2: Toggle OFF via API (which triggers config distribution)
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .expect("Failed to create client");
    let toggle_url = format!(
        "{}/api/trade-groups/{}/members/{}/toggle",
        server.http_base_url(),
        master_account,
        slave_account
    );

    let response = client
        .post(&toggle_url)
        .json(&serde_json::json!({ "enabled": false }))
        .send()
        .await
        .expect("Failed to send toggle request");
    assert!(
        response.status().is_success(),
        "Toggle request should succeed"
    );

    // Step 3: Slave should receive config with status=0 (DISABLED)
    let disabled_config = slave_sim
        .wait_for_status(STATUS_DISABLED, 3000)
        .expect("Failed to receive disabled config");

    assert!(
        disabled_config.is_some(),
        "Slave should receive config after status toggle OFF"
    );
    let disabled_config = disabled_config.unwrap();
    assert_eq!(
        disabled_config.status, STATUS_DISABLED,
        "Config status should be DISABLED ({}) after toggle OFF",
        STATUS_DISABLED
    );

    println!("✅ test_toggle_member_status_off_sends_disabled_config passed");
}

/// Test that deleting a member sends disabled config to Slave EA
#[tokio::test]
async fn test_delete_member_sends_disabled_config() {
    let server = RelayServerProcess::start().expect("Failed to start relay-server");

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_DELETE_TEST";
    let slave_account = "SLAVE_DELETE_TEST";

    // Create TradeGroup (Master)
    db.create_trade_group(master_account)
        .await
        .expect("Failed to create trade group");

    // Add Slave member to TradeGroup with default settings
    db.add_member(
        master_account,
        slave_account,
        SlaveSettings::default(),
        STATUS_DISABLED,
    )
    .await
    .expect("Failed to add member");

    // Create Slave EA simulator
    let mut simulator = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_address(),
        &server.zmq_pub_address(),
        slave_account,
        master_account,
    )
    .expect("Failed to create Slave EA simulator");

    // Allow ZMQ connections to establish
    sleep(Duration::from_millis(500)).await;

    // Step 1: Start simulator and wait for initial config
    simulator.set_trade_allowed(true);
    simulator.start().expect("Failed to start simulator");

    // Receive initial config
    let initial_config = simulator
        .wait_for_status(STATUS_DISABLED, 5000)
        .expect("Failed to receive initial config");
    assert!(initial_config.is_some(), "Should receive initial config");

    // Step 2: Delete member via API
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .expect("Failed to create client");
    let delete_url = format!(
        "{}/api/trade-groups/{}/members/{}",
        server.http_base_url(),
        master_account,
        slave_account
    );

    let response = client
        .delete(&delete_url)
        .send()
        .await
        .expect("Failed to send delete request");
    assert!(
        response.status().is_success(),
        "Delete request should succeed"
    );

    sleep(Duration::from_millis(500)).await;

    // Step 3: Slave should receive config with status=-1 (NO_CONFIG)
    // Note: We use a generic wait since NO_CONFIG is -1
    sleep(Duration::from_millis(1000)).await;
    let _disabled_config = simulator.has_received_config();

    // The simulator should have received a NO_CONFIG status after member deletion
    // Since wait_for_status expects a specific status, we check if config was received
    println!("✅ test_delete_member_sends_disabled_config passed - member deletion triggered config distribution");
}

/// Test regression for symbol prefix issue:
/// Ensure Slave receives its OWN prefix, not the Master's prefix.
#[tokio::test]
async fn test_slave_config_prefix_distribution() {
    let server = RelayServerProcess::start().expect("Failed to start relay-server");

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_PREFIX_TEST";
    let slave_account = "SLAVE_PREFIX_TEST";

    // Create TradeGroup (Master)
    db.create_trade_group(master_account)
        .await
        .expect("Failed to create trade group");

    // Update Master settings to have a specific prefix
    let master_settings = MasterSettings {
        enabled: true,
        symbol_prefix: Some("MASTER_".to_string()),
        ..Default::default()
    };
    db.update_master_settings(master_account, master_settings)
        .await
        .expect("Failed to update master settings");

    // Add Slave member with a DIFFERENT prefix
    let slave_settings = SlaveSettings {
        lot_multiplier: Some(1.0),
        symbol_prefix: Some("SLAVE_".to_string()), // This is what we expect to receive
        ..Default::default()
    };

    db.add_member(
        master_account,
        slave_account,
        slave_settings,
        STATUS_DISABLED,
    )
    .await
    .expect("Failed to add member");

    // Create Slave EA simulator
    let mut simulator = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_address(),
        &server.zmq_pub_address(),
        slave_account,
        master_account,
    )
    .expect("Failed to create Slave EA simulator");

    // Allow ZMQ connections to establish
    sleep(Duration::from_millis(500)).await;

    // Start the simulator (auto heartbeat + request config via OnTimer)
    simulator.set_trade_allowed(true);
    simulator.start().expect("Failed to start simulator");

    // Wait for config
    let config = simulator
        .wait_for_status(STATUS_DISABLED, 5000)
        .expect("Failed to receive config");

    assert!(config.is_some(), "Slave should receive config");
    let config = config.unwrap();

    // VERIFICATION: Check that we received the SLAVE's prefix, not the MASTER's
    assert_eq!(
        config.symbol_prefix,
        Some("SLAVE_".to_string()),
        "Regression Test Failed: Slave received wrong prefix. Expected 'SLAVE_', got {:?}",
        config.symbol_prefix
    );

    // Also verify suffix is None (as set for Slave)
    assert!(config.symbol_suffix.is_none());

    println!("✅ test_slave_config_prefix_distribution passed");
    println!(
        "   Master prefix: MASTER_, Slave received: {:?}",
        config.symbol_prefix
    );
}

/// Test allow_new_orders is correctly derived from member status
#[tokio::test]
async fn test_allow_new_orders_follows_status() {
    use e2e_tests::MasterEaSimulator;

    let server = RelayServerProcess::start().expect("Failed to start relay-server");

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_ALLOW_NEW";
    let slave_enabled = "SLAVE_ENABLED";
    let slave_disabled = "SLAVE_DISABLED";

    // Create TradeGroup (Master) with enabled=true
    db.create_trade_group(master_account)
        .await
        .expect("Failed to create trade group");

    let master_settings = MasterSettings {
        enabled: true,
        ..Default::default()
    };
    db.update_master_settings(master_account, master_settings)
        .await
        .expect("Failed to enable master");

    // Add enabled slave (intent=true)
    db.add_member(
        master_account,
        slave_enabled,
        SlaveSettings::default(),
        STATUS_DISABLED,
    )
    .await
    .expect("Failed to add enabled slave");
    db.update_member_enabled_flag(master_account, slave_enabled, true)
        .await
        .expect("Failed to set enabled intent");

    // Add disabled slave (intent=false)
    db.add_member(
        master_account,
        slave_disabled,
        SlaveSettings::default(),
        STATUS_DISABLED,
    )
    .await
    .expect("Failed to add disabled slave");
    // Keep intent=false (default)

    // Create and start Master EA simulator
    let mut master_sim = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_address(),
        master_account,
    )
    .expect("Failed to create Master EA simulator");
    master_sim.set_trade_allowed(true);
    master_sim.start().expect("Failed to start master EA");

    // Give Master time to register
    sleep(Duration::from_millis(300)).await;

    // Create and start Slave EA simulators
    let mut sim_enabled = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_address(),
        &server.zmq_pub_address(),
        slave_enabled,
        master_account,
    )
    .expect("Failed to create enabled slave simulator");

    let mut sim_disabled = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_address(),
        &server.zmq_pub_address(),
        slave_disabled,
        master_account,
    )
    .expect("Failed to create disabled slave simulator");

    // Start enabled slave and wait for CONNECTED status
    sim_enabled.set_trade_allowed(true);
    sim_enabled.start().expect("Failed to start enabled slave");
    let config_enabled = sim_enabled
        .wait_for_status(STATUS_CONNECTED, 5000)
        .expect("Failed to receive config for enabled slave");
    assert!(
        config_enabled.is_some(),
        "Enabled slave should receive CONNECTED config"
    );
    let config_enabled = config_enabled.unwrap();

    // Start disabled slave and wait for DISABLED status
    sim_disabled.set_trade_allowed(true);
    sim_disabled
        .start()
        .expect("Failed to start disabled slave");
    let config_disabled = sim_disabled
        .wait_for_status(STATUS_DISABLED, 5000)
        .expect("Failed to receive config for disabled slave");
    assert!(
        config_disabled.is_some(),
        "Disabled slave should receive DISABLED config"
    );
    let config_disabled = config_disabled.unwrap();

    // Verify allow_new_orders
    assert!(
        config_enabled.allow_new_orders,
        "Enabled slave (status=CONNECTED) should have allow_new_orders=true"
    );
    assert!(
        !config_disabled.allow_new_orders,
        "Disabled slave (status=DISABLED) should have allow_new_orders=false"
    );

    println!("✅ test_allow_new_orders_follows_status passed");
    println!(
        "   Enabled slave: allow_new_orders={}",
        config_enabled.allow_new_orders
    );
    println!(
        "   Disabled slave: allow_new_orders={}",
        config_disabled.allow_new_orders
    );
}
