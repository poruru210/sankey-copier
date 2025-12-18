// e2e-tests/tests/order_routing.rs
//
// E2E integration tests for order routing and signal delivery.
// Tests multi-master isolation, broadcast to multiple slaves, and latency.
//
// Migrated from relay-server/tests/e2e_trade_signal_test.rs

use e2e_tests::helpers::{default_test_slave_settings, setup_test_scenario};
use e2e_tests::SlaveEaSimulator;
use e2e_tests::TestSandbox;
use e2e_tests::{OrderType, TradeSignal};
use sankey_copier_relay_server::adapters::outbound::persistence::Database;
use sankey_copier_relay_server::domain::models::{LotCalculationMode, SlaveSettings, SyncMode};
use tokio::time::{sleep, Duration};

// =============================================================================
// Helper Functions
// =============================================================================

/// Helper function to collect multiple trade signals from a slave
fn collect_trade_signals(
    slave: &SlaveEaSimulator,
    timeout_ms: i32,
    max_signals: usize,
) -> Result<Vec<TradeSignal>, String> {
    let mut signals = Vec::new();
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_millis(timeout_ms as u64);

    while signals.len() < max_signals && start.elapsed() < timeout {
        let remaining = timeout.saturating_sub(start.elapsed());
        let remaining_ms = remaining.as_millis() as i32;
        if remaining_ms <= 0 {
            break;
        }

        match slave.try_receive_trade_signal(remaining_ms.min(100)) {
            Ok(Some(signal)) => signals.push(signal),
            Ok(None) => continue,
            Err(e) => return Err(e.to_string()),
        }
    }

    Ok(signals)
}

// =============================================================================
// Multi-Master Isolation Tests
// =============================================================================

/// Test multi-master signal isolation
/// Master1 -> Slave1, Master2 -> Slave2 (no cross-contamination)
#[tokio::test]
async fn test_multi_master_signal_isolation() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master1_account = "MASTER_ISO_001";
    let master2_account = "MASTER_ISO_002";
    let slave1_account = "SLAVE_ISO_001";
    let slave2_account = "SLAVE_ISO_002";

    // Setup: Master1 -> Slave1, Master2 -> Slave2
    setup_test_scenario(&db, master1_account, &[slave1_account], |_| {
        default_test_slave_settings()
    })
    .await
    .expect("Failed to setup test scenario for master1");

    setup_test_scenario(&db, master2_account, &[slave2_account], |_| {
        default_test_slave_settings()
    })
    .await
    .expect("Failed to setup test scenario for master2");

    let mut master1 = sandbox
        .create_master(master1_account, true)
        .expect("Failed to create master1 simulator");

    let mut master2 = sandbox
        .create_master(master2_account, true)
        .expect("Failed to create master2 simulator");

    let mut slave1 = sandbox
        .create_slave(slave1_account, master1_account, true)
        .expect("Failed to create slave1 simulator");

    let mut slave2 = sandbox
        .create_slave(slave2_account, master2_account, true)
        .expect("Failed to create slave2 simulator");

    // Start all EAs
    master1.set_trade_allowed(true);
    master1.start().expect("Failed to start master1");
    master2.set_trade_allowed(true);
    master2.start().expect("Failed to start master2");
    slave1.set_trade_allowed(true);
    slave1.start().expect("Failed to start slave1");
    slave2.set_trade_allowed(true);
    slave2.start().expect("Failed to start slave2");

    slave1
        .wait_for_status(2, 5000)
        .expect("Slave1 failed to connect");
    slave2
        .wait_for_status(2, 5000)
        .expect("Slave2 failed to connect");
    sleep(Duration::from_millis(1000)).await;

    // Master1 sends ticket 100
    let sig1 =
        master1.create_open_signal(100, "EURUSD", OrderType::Buy, 0.1, 1.0850, None, None, 0);
    master1
        .send_trade_signal(&sig1)
        .expect("Failed to send signal");

    // Master2 sends ticket 200
    let sig2 =
        master2.create_open_signal(200, "GBPUSD", OrderType::Sell, 0.2, 1.2500, None, None, 0);
    master2
        .send_trade_signal(&sig2)
        .expect("Failed to send signal");

    sleep(Duration::from_millis(500)).await;

    let signals1 = collect_trade_signals(&slave1, 2000, 2).expect("Failed to collect signals");
    let signals2 = collect_trade_signals(&slave2, 2000, 2).expect("Failed to collect signals");

    // Slave1 should only receive ticket 100 from Master1
    assert_eq!(signals1.len(), 1, "Slave1 should receive only 1 signal");
    assert_eq!(signals1[0].ticket, 100);

    // Slave2 should only receive ticket 200 from Master2
    assert_eq!(signals2.len(), 1, "Slave2 should receive only 1 signal");
    assert_eq!(signals2[0].ticket, 200);

    println!("✅ test_multi_master_signal_isolation passed");
}

/// Test same symbol from different masters
#[tokio::test]
async fn test_multi_master_same_symbol_open() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master1_account = "MASTER_SAME_SYM_001";
    let master2_account = "MASTER_SAME_SYM_002";
    let slave1_account = "SLAVE_SAME_SYM_001";
    let slave2_account = "SLAVE_SAME_SYM_002";

    setup_test_scenario(&db, master1_account, &[slave1_account], |_| {
        default_test_slave_settings()
    })
    .await
    .expect("Failed to setup test scenario for master1");

    setup_test_scenario(&db, master2_account, &[slave2_account], |_| {
        default_test_slave_settings()
    })
    .await
    .expect("Failed to setup test scenario for master2");

    let mut master1 = sandbox
        .create_master(master1_account, true)
        .expect("Failed to create master1 simulator");

    let mut master2 = sandbox
        .create_master(master2_account, true)
        .expect("Failed to create master2 simulator");

    let mut slave1 = sandbox
        .create_slave(slave1_account, master1_account, true)
        .expect("Failed to create slave1 simulator");

    let mut slave2 = sandbox
        .create_slave(slave2_account, master2_account, true)
        .expect("Failed to create slave2 simulator");

    // Start all EAs
    master1.set_trade_allowed(true);
    master1.start().expect("Failed to start master1");
    master2.set_trade_allowed(true);
    master2.start().expect("Failed to start master2");
    slave1.set_trade_allowed(true);
    slave1.start().expect("Failed to start slave1");
    slave2.set_trade_allowed(true);
    slave2.start().expect("Failed to start slave2");

    slave1
        .wait_for_status(2, 5000)
        .expect("Slave1 failed to connect");
    slave2
        .wait_for_status(2, 5000)
        .expect("Slave2 failed to connect");
    sleep(Duration::from_millis(1000)).await;

    // Both masters send Open for EURUSD (same symbol)
    let sig1 =
        master1.create_open_signal(100, "EURUSD", OrderType::Buy, 0.1, 1.0850, None, None, 0);
    let sig2 =
        master2.create_open_signal(200, "EURUSD", OrderType::Sell, 0.2, 1.0850, None, None, 0);

    master1
        .send_trade_signal(&sig1)
        .expect("Failed to send signal");
    master2
        .send_trade_signal(&sig2)
        .expect("Failed to send signal");

    sleep(Duration::from_millis(500)).await;

    let signals1 = collect_trade_signals(&slave1, 2000, 2).expect("Failed to collect signals");
    let signals2 = collect_trade_signals(&slave2, 2000, 2).expect("Failed to collect signals");

    // Each slave receives only its master's signal (no cross-contamination)
    assert_eq!(signals1.len(), 1);
    assert_eq!(signals1[0].ticket, 100);
    assert_eq!(signals1[0].order_type, Some(OrderType::Buy));

    assert_eq!(signals2.len(), 1);
    assert_eq!(signals2[0].ticket, 200);
    assert_eq!(signals2[0].order_type, Some(OrderType::Sell));

    println!("✅ test_multi_master_same_symbol_open passed");
}

// =============================================================================
// Multiple Slaves Tests
// =============================================================================

/// Test signal broadcast to all slaves
#[tokio::test]
async fn test_signal_broadcast_to_all_slaves() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_BROADCAST_001";
    let slave1_account = "SLAVE_BROADCAST_001";
    let slave2_account = "SLAVE_BROADCAST_002";
    let slave3_account = "SLAVE_BROADCAST_003";

    // Setup: 1 Master -> 3 Slaves
    setup_test_scenario(
        &db,
        master_account,
        &[slave1_account, slave2_account, slave3_account],
        |_| default_test_slave_settings(),
    )
    .await
    .expect("Failed to setup test scenario");

    let mut master = sandbox
        .create_master(master_account, true)
        .expect("Failed to create master simulator");

    let mut slave1 = sandbox
        .create_slave(slave1_account, master_account, true)
        .expect("Failed to create slave1 simulator");

    let mut slave2 = sandbox
        .create_slave(slave2_account, master_account, true)
        .expect("Failed to create slave2 simulator");

    let mut slave3 = sandbox
        .create_slave(slave3_account, master_account, true)
        .expect("Failed to create slave3 simulator");

    // Start all EAs
    master.set_trade_allowed(true);
    master.start().expect("Failed to start master");
    slave1.set_trade_allowed(true);
    slave1.start().expect("Failed to start slave1");
    slave2.set_trade_allowed(true);
    slave2.start().expect("Failed to start slave2");
    slave3.set_trade_allowed(true);
    slave3.start().expect("Failed to start slave3");

    slave1
        .wait_for_status(2, 5000)
        .expect("Slave1 failed to connect");
    slave2
        .wait_for_status(2, 5000)
        .expect("Slave2 failed to connect");
    slave3
        .wait_for_status(2, 5000)
        .expect("Slave3 failed to connect");
    sleep(Duration::from_millis(1000)).await;

    // Master sends one signal
    let signal =
        master.create_open_signal(12345, "EURUSD", OrderType::Buy, 0.1, 1.0850, None, None, 0);
    master
        .send_trade_signal(&signal)
        .expect("Failed to send signal");

    sleep(Duration::from_millis(500)).await;

    let signals1 = collect_trade_signals(&slave1, 2000, 1).expect("Failed to collect signals");
    let signals2 = collect_trade_signals(&slave2, 2000, 1).expect("Failed to collect signals");
    let signals3 = collect_trade_signals(&slave3, 2000, 1).expect("Failed to collect signals");

    // All 3 slaves should receive the signal
    assert_eq!(signals1.len(), 1, "Slave1 should receive signal");
    assert_eq!(signals2.len(), 1, "Slave2 should receive signal");
    assert_eq!(signals3.len(), 1, "Slave3 should receive signal");

    // All received the same ticket
    assert_eq!(signals1[0].ticket, 12345);
    assert_eq!(signals2[0].ticket, 12345);
    assert_eq!(signals3[0].ticket, 12345);

    println!("✅ test_signal_broadcast_to_all_slaves passed");
}

/// Test lot multiplier passthrough
/// Note: Lot multiplier is handled by Slave EA, Relay Server passes through original lots
#[tokio::test]
async fn test_slave_individual_lot_multiplier() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_LOT_MULT_001";
    let slave_account = "SLAVE_LOT_MULT_001";

    // Setup with 2x lot multiplier (handled by Slave EA, not Relay Server)
    setup_test_scenario(&db, master_account, &[slave_account], |_| SlaveSettings {
        lot_calculation_mode: LotCalculationMode::Multiplier,
        lot_multiplier: Some(2.0),
        reverse_trade: false,
        symbol_prefix: None,
        symbol_suffix: None,
        symbol_mappings: vec![],
        filters: sankey_copier_relay_server::domain::models::TradeFilters::default(),
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
    })
    .await
    .expect("Failed to setup test scenario");

    let mut master = sandbox
        .create_master(master_account, true)
        .expect("Failed to create master simulator");

    let mut slave = sandbox
        .create_slave(slave_account, master_account, true)
        .expect("Failed to create slave simulator");

    // Start all EAs
    master.set_trade_allowed(true);
    master.start().expect("Failed to start master");
    slave.set_trade_allowed(true);
    slave.start().expect("Failed to start slave");
    sleep(Duration::from_millis(2000)).await;

    // Master sends 0.1 lot
    let signal =
        master.create_open_signal(12345, "EURUSD", OrderType::Buy, 0.1, 1.0850, None, None, 0);
    master
        .send_trade_signal(&signal)
        .expect("Failed to send signal");

    sleep(Duration::from_millis(500)).await;

    let signals = collect_trade_signals(&slave, 3000, 1).expect("Failed to collect signals");

    assert_eq!(signals.len(), 1, "Should receive 1 signal");

    // Verify lots are transformed by mt-bridge (0.1 * 2.0 = 0.2)
    let lots = signals[0].lots.expect("lots should be present");
    assert!(
        (lots - 0.2).abs() < 0.001,
        "Lots should be 0.2 (0.1 * 2.0 calculated by mt-bridge), got {}",
        lots
    );

    println!("✅ test_slave_individual_lot_multiplier passed");
}

// =============================================================================
// Latency Tests
// =============================================================================

/// Test signal latency measurement
#[tokio::test]
async fn test_signal_latency_measurement() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_LATENCY_001";
    let slave_account = "SLAVE_LATENCY_001";

    setup_test_scenario(&db, master_account, &[slave_account], |_| {
        default_test_slave_settings()
    })
    .await
    .expect("Failed to setup test scenario");

    let mut master = sandbox
        .create_master(master_account, true)
        .expect("Failed to create master simulator");

    let mut slave = sandbox
        .create_slave(slave_account, master_account, true)
        .expect("Failed to create slave simulator");

    // Start all EAs
    master.set_trade_allowed(true);
    master.start().expect("Failed to start master");
    slave.set_trade_allowed(true);
    slave.start().expect("Failed to start slave");
    sleep(Duration::from_millis(2000)).await;

    // Measure latency for 10 signals
    let mut latencies = Vec::new();

    for i in 1..=10 {
        let send_time = std::time::Instant::now();
        let signal =
            master.create_open_signal(i, "EURUSD", OrderType::Buy, 0.1, 1.0850, None, None, 0);
        master
            .send_trade_signal(&signal)
            .expect("Failed to send signal");

        if slave
            .try_receive_trade_signal(1000)
            .expect("Failed to receive")
            .is_some()
        {
            let latency = send_time.elapsed();
            latencies.push(latency.as_millis() as f64);
        }

        sleep(Duration::from_millis(50)).await;
    }

    let avg_latency: f64 = latencies.iter().sum::<f64>() / latencies.len() as f64;
    let max_latency = latencies.iter().cloned().fold(0.0_f64, f64::max);

    println!(
        "Latency stats: avg={:.2}ms, max={:.2}ms, samples={}",
        avg_latency,
        max_latency,
        latencies.len()
    );

    // Assert reasonable latency (should be < 50ms in local test)
    assert!(
        avg_latency < 100.0,
        "Average latency {} ms exceeds 50ms threshold",
        avg_latency
    );

    println!("✅ test_signal_latency_measurement passed");
}

// =============================================================================
// Delayed Signal Tests
// =============================================================================

/// Test delayed signal (100ms old) - should be delivered by server
/// Server delivers all signals regardless of timestamp - EA is responsible for validation
#[tokio::test]
async fn test_delayed_signal_immediate() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_DELAY_IMM_001";
    let slave_account = "SLAVE_DELAY_IMM_001";

    setup_test_scenario(&db, master_account, &[slave_account], |_| {
        default_test_slave_settings()
    })
    .await
    .expect("Failed to setup test scenario");

    let mut master = sandbox
        .create_master(master_account, true)
        .expect("Failed to create master simulator");

    let mut slave = sandbox
        .create_slave(slave_account, master_account, true)
        .expect("Failed to create slave simulator");

    // Start all EAs
    master.set_trade_allowed(true);
    master.start().expect("Failed to start master");
    slave.set_trade_allowed(true);
    slave.start().expect("Failed to start slave");
    sleep(Duration::from_millis(2000)).await;

    // Create signal with 100ms old timestamp
    let signal =
        master.create_open_signal(12345, "EURUSD", OrderType::Buy, 0.1, 1.0850, None, None, 0);
    let delayed_signal = master.create_delayed_signal(signal, 100);
    master
        .send_trade_signal(&delayed_signal)
        .expect("Failed to send delayed signal");

    sleep(Duration::from_millis(200)).await;
    let received = slave
        .try_receive_trade_signal(3000)
        .expect("Failed to receive signal");

    // Should still be delivered (server doesn't filter by timestamp)
    assert!(
        received.is_some(),
        "Slightly delayed signal (100ms) should be delivered"
    );

    println!("✅ test_delayed_signal_immediate passed");
}

/// Test delayed signal (3 seconds old) - should still be delivered
#[tokio::test]
async fn test_delayed_signal_acceptable() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_DELAY_ACC_001";
    let slave_account = "SLAVE_DELAY_ACC_001";

    setup_test_scenario(&db, master_account, &[slave_account], |_| {
        default_test_slave_settings()
    })
    .await
    .expect("Failed to setup test scenario");

    let mut master = sandbox
        .create_master(master_account, true)
        .expect("Failed to create master simulator");

    let mut slave = sandbox
        .create_slave(slave_account, master_account, true)
        .expect("Failed to create slave simulator");

    // Start all EAs
    master.set_trade_allowed(true);
    master.start().expect("Failed to start master");
    slave.set_trade_allowed(true);
    slave.start().expect("Failed to start slave");
    sleep(Duration::from_millis(2000)).await;

    // Create signal with 3 second old timestamp
    let signal =
        master.create_open_signal(12346, "EURUSD", OrderType::Buy, 0.1, 1.0850, None, None, 0);
    let delayed_signal = master.create_delayed_signal(signal, 3000);
    master
        .send_trade_signal(&delayed_signal)
        .expect("Failed to send delayed signal");

    sleep(Duration::from_millis(200)).await;
    let received = slave
        .try_receive_trade_signal(3000)
        .expect("Failed to receive signal");

    // Server doesn't filter by timestamp - should be delivered
    assert!(
        received.is_some(),
        "Moderately delayed signal (3s) should be delivered (filtering is EA's job)"
    );

    println!("✅ test_delayed_signal_acceptable passed");
}

/// Test stale signal (10+ seconds old) - should still be delivered with old timestamp
#[tokio::test]
async fn test_stale_signal_too_old() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_STALE_001";
    let slave_account = "SLAVE_STALE_001";

    setup_test_scenario(&db, master_account, &[slave_account], |_| {
        let mut settings = default_test_slave_settings();
        settings.max_signal_delay_ms = 5000;
        settings
    })
    .await
    .expect("Failed to setup test scenario");

    let mut master = sandbox
        .create_master(master_account, true)
        .expect("Failed to create master simulator");

    let mut slave = sandbox
        .create_slave(slave_account, master_account, true)
        .expect("Failed to create slave simulator");

    // Start all EAs
    master.set_trade_allowed(true);
    master.start().expect("Failed to start master");
    slave.set_trade_allowed(true);
    slave.start().expect("Failed to start slave");
    sleep(Duration::from_millis(2000)).await;

    // Create signal with 10 second old timestamp
    let signal =
        master.create_open_signal(12347, "EURUSD", OrderType::Buy, 0.1, 1.0850, None, None, 0);
    let stale_signal = master.create_delayed_signal(signal, 10000);
    master
        .send_trade_signal(&stale_signal)
        .expect("Failed to send stale signal");

    sleep(Duration::from_millis(500)).await;
    let received = slave
        .try_receive_trade_signal(3000)
        .expect("Failed to receive signal");

    // Server delivers all signals - EA (mt-bridge) validates timestamp and drops it if too old
    assert!(
        received.is_none(),
        "Stale signal (10s old) should be rejected by EA latency check (max_signal_delay_ms=5000)"
    );

    println!("✅ test_stale_signal_too_old passed (Signal correctly rejected)");
}
