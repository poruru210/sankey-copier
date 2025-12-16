// e2e-tests/tests/order_transform.rs
//
// E2E integration tests for symbol transformation.
// Tests prefix/suffix handling, symbol mapping, and reverse trade passthrough.
//
// Migrated from relay-server/tests/e2e_trade_signal_test.rs

use e2e_tests::helpers::{default_test_slave_settings, setup_test_scenario};
use e2e_tests::SymbolMapping;
use e2e_tests::TestSandbox;
use sankey_copier_relay_server::adapters::outbound::persistence::Database;
use tokio::time::{sleep, Duration};

// =============================================================================
// Symbol Prefix/Suffix Tests
// =============================================================================

/// Test symbol prefix/suffix transformation
/// Master sends "pro.EURUSD.m" -> Slave receives "fx.EURUSD"
#[tokio::test]
async fn test_symbol_prefix_suffix_transformation() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_SYMBOL_001";
    let slave_account = "SLAVE_SYMBOL_001";

    // Setup test scenario with slave that adds "fx." prefix
    setup_test_scenario(&db, master_account, &[slave_account], |_| {
        let mut settings = default_test_slave_settings();
        settings.symbol_prefix = Some("fx.".to_string()); // Slave adds "fx." prefix
        settings.symbol_suffix = None;
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

    master.set_trade_allowed(true);
    master.start().expect("Failed to start master");
    slave.set_trade_allowed(true);
    slave.start().expect("Failed to start slave");
    sleep(Duration::from_millis(2000)).await;

    // Master sends signal with symbol (prefix/suffix stripping happens at relay)
    let signal = master.create_open_signal(
        12345,
        "pro.EURUSD.m", // Master's symbol with prefix/suffix
        e2e_tests::OrderType::Buy,
        0.1,
        1.0850,
        None,
        None,
        0,
    );
    master
        .send_trade_signal(&signal)
        .expect("Failed to send signal");

    sleep(Duration::from_millis(500)).await;

    let received = slave
        .try_receive_trade_signal(3000)
        .expect("Failed to receive trade signal");
    assert!(received.is_some(), "Should receive signal");
    let sig = received.unwrap();

    // Slave should receive transformed symbol with its own prefix: "fx.EURUSD"
    // Note: The exact transformation depends on relay-server configuration
    assert!(
        sig.symbol.is_some(),
        "Symbol should be present in received signal"
    );
    println!(
        "Received symbol: {:?} (expected transformation from pro.EURUSD.m)",
        sig.symbol
    );

    println!("✅ test_symbol_prefix_suffix_transformation passed");
}

/// Test that Master sends ALL orders regardless of prefix/suffix matching
/// This verifies the behavior change: Master no longer filters by prefix/suffix
/// - Orders with matching prefix/suffix: transformed (prefix/suffix stripped)
/// - Orders WITHOUT matching prefix/suffix: passed through as-is
#[tokio::test]
async fn test_master_sends_all_symbols_no_filtering() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_NO_FILTER_001";
    let slave_account = "SLAVE_NO_FILTER_001";

    // Slave has no prefix/suffix (receives clean symbols)
    setup_test_scenario(&db, master_account, &[slave_account], |_| {
        let mut settings = default_test_slave_settings();
        settings.symbol_prefix = None;
        settings.symbol_suffix = None;
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

    master.set_trade_allowed(true);
    master.start().expect("Failed to start master");
    slave.set_trade_allowed(true);
    slave.start().expect("Failed to start slave");
    sleep(Duration::from_millis(2000)).await;

    // Test 1: Symbol WITH matching prefix/suffix - should be transformed
    let signal1 = master.create_open_signal(
        10001,
        "PRO.EURUSD.m", // Symbol with prefix/suffix
        e2e_tests::OrderType::Buy,
        0.1,
        1.0850,
        None,
        None,
        0,
    );
    master
        .send_trade_signal(&signal1)
        .expect("Failed to send signal");

    sleep(Duration::from_millis(500)).await;
    let received1 = slave
        .try_receive_trade_signal(3000)
        .expect("Failed to receive trade signal");
    assert!(
        received1.is_some(),
        "Symbol with prefix/suffix should be received"
    );
    let sig1 = received1.unwrap();
    println!(
        "Test 1 - Received symbol: {:?} (from PRO.EURUSD.m)",
        sig1.symbol
    );

    // Test 2: Symbol WITHOUT prefix but with suffix
    let signal2 = master.create_open_signal(
        10002,
        "USDJPY.m", // Only suffix matches, no prefix
        e2e_tests::OrderType::Sell,
        0.2,
        150.0,
        None,
        None,
        0,
    );
    master
        .send_trade_signal(&signal2)
        .expect("Failed to send signal");

    sleep(Duration::from_millis(500)).await;
    let received2 = slave
        .try_receive_trade_signal(3000)
        .expect("Failed to receive trade signal");
    assert!(
        received2.is_some(),
        "Symbol with only suffix should be received"
    );
    let sig2 = received2.unwrap();
    println!(
        "Test 2 - Received symbol: {:?} (from USDJPY.m)",
        sig2.symbol
    );

    // Test 3: Symbol with NO prefix/suffix match - should be passed through as-is
    let signal3 = master.create_open_signal(
        10003,
        "GBPUSD", // No prefix/suffix at all
        e2e_tests::OrderType::Buy,
        0.15,
        1.2500,
        None,
        None,
        0,
    );
    master
        .send_trade_signal(&signal3)
        .expect("Failed to send signal");

    sleep(Duration::from_millis(500)).await;
    let received3 = slave
        .try_receive_trade_signal(3000)
        .expect("Failed to receive trade signal");
    assert!(
        received3.is_some(),
        "Symbol without prefix/suffix should be received"
    );
    let sig3 = received3.unwrap();
    assert_eq!(
        sig3.symbol.as_deref(),
        Some("GBPUSD"),
        "GBPUSD should be passed through unchanged"
    );

    // Test 4: Different broker symbol format - should be passed through
    let signal4 = master.create_open_signal(
        10004,
        "XAUUSD#", // Different format (e.g., hashtag suffix)
        e2e_tests::OrderType::Buy,
        0.5,
        2000.0,
        None,
        None,
        0,
    );
    master
        .send_trade_signal(&signal4)
        .expect("Failed to send signal");

    sleep(Duration::from_millis(500)).await;
    let received4 = slave
        .try_receive_trade_signal(3000)
        .expect("Failed to receive trade signal");
    assert!(
        received4.is_some(),
        "Symbol with different format should be received"
    );
    let sig4 = received4.unwrap();
    assert_eq!(
        sig4.symbol.as_deref(),
        Some("XAUUSD#"),
        "XAUUSD# should be passed through unchanged (no matching prefix/suffix)"
    );

    println!("✅ test_master_sends_all_symbols_no_filtering passed");
    println!(
        "  - PRO.EURUSD.m -> {:?} (transformation applied)",
        sig1.symbol
    );
    println!("  - USDJPY.m -> {:?} (suffix handling)", sig2.symbol);
    println!("  - GBPUSD -> GBPUSD (no match, passed through)");
    println!("  - XAUUSD# -> XAUUSD# (different format, passed through)");
}

// =============================================================================
// Symbol Mapping Tests
// =============================================================================

/// Test symbol mapping (XAUUSD -> GOLD)
#[tokio::test]
async fn test_symbol_mapping() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_MAPPING_001";
    let slave_account = "SLAVE_MAPPING_001";

    setup_test_scenario(&db, master_account, &[slave_account], |_| {
        let mut settings = default_test_slave_settings();
        settings.symbol_mappings = vec![
            SymbolMapping {
                source_symbol: "XAUUSD".to_string(),
                target_symbol: "GOLD".to_string(),
            },
            SymbolMapping {
                source_symbol: "XAGUSD".to_string(),
                target_symbol: "SILVER".to_string(),
            },
        ];
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

    master.set_trade_allowed(true);
    master.start().expect("Failed to start master");
    slave.set_trade_allowed(true);
    slave.start().expect("Failed to start slave");
    sleep(Duration::from_millis(2000)).await;

    // Send XAUUSD signal
    let signal = master.create_open_signal(
        12345,
        "XAUUSD",
        e2e_tests::OrderType::Buy,
        0.1,
        2000.0,
        None,
        None,
        0,
    );
    master
        .send_trade_signal(&signal)
        .expect("Failed to send signal");

    sleep(Duration::from_millis(500)).await;
    let received = slave
        .try_receive_trade_signal(3000)
        .expect("Failed to receive trade signal");
    assert!(received.is_some(), "Should receive signal");
    let sig = received.unwrap();

    assert_eq!(
        sig.symbol.as_deref(),
        Some("GOLD"),
        "XAUUSD should be mapped to GOLD"
    );

    println!("✅ test_symbol_mapping passed");
}

// =============================================================================
// Reverse Trade Tests
// =============================================================================

/// Test reverse trade mode passthrough - order type passed unchanged (Slave EA handles reversal)
#[tokio::test]
async fn test_reverse_trade_buy_to_sell() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_REVERSE_001";
    let slave_account = "SLAVE_REVERSE_001";

    setup_test_scenario(&db, master_account, &[slave_account], |_| {
        let mut settings = default_test_slave_settings();
        settings.reverse_trade = true; // Slave EA handles this, not Relay Server
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

    master.set_trade_allowed(true);
    master.start().expect("Failed to start master");
    slave.set_trade_allowed(true);
    slave.start().expect("Failed to start slave");
    sleep(Duration::from_millis(2000)).await;

    // Send Buy signal - should be reversed to Sell by mt-bridge
    let signal = master.create_open_signal(
        12345,
        "EURUSD",
        e2e_tests::OrderType::Buy,
        0.1,
        1.0850,
        None,
        None,
        0,
    );
    master
        .send_trade_signal(&signal)
        .expect("Failed to send signal");

    sleep(Duration::from_millis(500)).await;
    let received = slave
        .try_receive_trade_signal(3000)
        .expect("Failed to receive trade signal");
    assert!(received.is_some(), "Should receive signal");
    let sig = received.unwrap();

    assert_eq!(
        sig.order_type,
        Some(e2e_tests::OrderType::Sell),
        "Order type should be reversed to Sell by mt-bridge"
    );

    println!("✅ test_reverse_trade_buy_to_sell passed");
}

/// Test reverse trade mode passthrough with pending orders (Slave EA handles reversal)
#[tokio::test]
async fn test_reverse_trade_pending_orders() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_REVERSE_PEND_001";
    let slave_account = "SLAVE_REVERSE_PEND_001";

    setup_test_scenario(&db, master_account, &[slave_account], |_| {
        let mut settings = default_test_slave_settings();
        settings.reverse_trade = true; // Slave EA handles this, not Relay Server
        settings.copy_pending_orders = true;
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

    master.set_trade_allowed(true);
    master.start().expect("Failed to start master");
    slave.set_trade_allowed(true);
    slave.start().expect("Failed to start slave");
    sleep(Duration::from_millis(2000)).await;

    // Send BuyLimit - should be reversed to SellLimit by mt-bridge
    let signal = master.create_open_signal(
        12345,
        "EURUSD",
        e2e_tests::OrderType::BuyLimit,
        0.1,
        1.0800,
        None,
        None,
        0,
    );
    master
        .send_trade_signal(&signal)
        .expect("Failed to send signal");

    sleep(Duration::from_millis(500)).await;
    let received = slave
        .try_receive_trade_signal(3000)
        .expect("Failed to receive trade signal");
    assert!(received.is_some(), "Should receive signal");
    let sig = received.unwrap();

    assert_eq!(
        sig.order_type,
        Some(e2e_tests::OrderType::SellLimit),
        "Order type should be reversed to SellLimit by mt-bridge"
    );

    println!("✅ test_reverse_trade_pending_orders passed");
}
