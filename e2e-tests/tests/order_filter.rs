// e2e-tests/tests/order_filter.rs
//
// E2E integration tests for trade signal filtering.
// Tests partial close, full close, symbol/magic number filters, lot limits, and pending orders.
//
// Migrated from relay-server/tests/e2e_trade_signal_test.rs

use e2e_tests::helpers::{default_test_slave_settings, setup_test_scenario};
use e2e_tests::types::{OrderType, TradeAction};
use e2e_tests::TestSandbox;
use e2e_tests::TradeFilters;
use sankey_copier_relay_server::adapters::outbound::persistence::Database;
use tokio::time::{sleep, Duration};

// =============================================================================
// Partial Close Tests
// =============================================================================

/// Test partial close signal with close_ratio
/// Verifies:
/// 1. close_ratio is preserved through the relay
/// 2. Lots are passed through unchanged (Slave EA handles lot calculation)
#[tokio::test]
async fn test_partial_close_signal() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_PARTIAL_001";
    let slave_account = "SLAVE_PARTIAL_001";

    // Set up slave with 2x lot multiplier (handled by Slave EA, not Relay Server)
    setup_test_scenario(&db, master_account, &[slave_account], |_| {
        let mut settings = default_test_slave_settings();
        settings.lot_multiplier = Some(2.0); // 2x multiplier (Slave EA handles this)
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

    // Wait for connection
    slave
        .wait_for_status(2, 5000)
        .expect("Failed to reach CONNECTED status");
    sleep(Duration::from_millis(1000)).await;

    // Step 1: Open a position (lots passed through unchanged, Slave EA applies multiplier)
    let open_signal =
        master.create_open_signal(12345, "EURUSD", OrderType::Buy, 1.0, 1.0850, None, None, 0);
    master
        .send_trade_signal(&open_signal)
        .expect("Failed to send signal");

    // Wait for Open signal
    let received_open = slave
        .wait_for_trade_action(TradeAction::Open, 3000)
        .expect("Failed to receive Open signal");
    assert!(received_open.is_some(), "Should receive Open signal");
    let open_sig = received_open.unwrap();
    assert_eq!(
        open_sig.lots,
        Some(2.0),
        "Lots should be calculated by mt-bridge (1.0 * 2.0 = 2.0)"
    );

    // Step 2: Partial close with 50% close_ratio
    // Note: Master's lots=1.0 (original), close_ratio=0.5 means 50% closed
    let partial_close_signal = master.create_partial_close_signal(12345, "EURUSD", 1.0, 0.5); // 50% partial close
    master
        .send_trade_signal(&partial_close_signal)
        .expect("Failed to send signal");

    // Wait for Close signal
    // Note: wait_for_trade_action calls try_receive_trade_signal internally which consumes the queue
    let received_close = slave
        .wait_for_trade_action(TradeAction::Close, 3000)
        .expect("Failed to receive Close signal");
    assert!(received_close.is_some(), "Should receive Close signal");

    let received_signal = received_close.unwrap();
    assert_eq!(received_signal.action, TradeAction::Close);
    assert_eq!(
        received_signal.close_ratio,
        Some(0.5),
        "close_ratio should be preserved: 0.5"
    );
    // Note: Close signal lots might also be transformed if we implemented it for Close.
    // In `ea_context.rs`, we currently only implemented `transform_lot_size` for `Open`.
    // For `Close`, MQL traditionally uses `Ticket` to close volume.
    // If partial close, `lots` usually means "volume to close".
    // Does Master send "Volume to close" or "Total Volume"?
    // Master sends "Original Volume" and "Close Ratio".
    // If we transformed Open Volume (2.0), we should probably transform Close Volume too?
    // Current `ea_context.rs` `process_incoming_trade`:
    // It applies `transform_lot_size` for ALL actions?
    // Let's check `ea_context.rs`:
    // `let raw_lots = signal.lots.unwrap_or(0.0); let final_lots = transform_lot_size(...)`
    // Yes, it applies to all.
    // So if Master says "Close 1.0 lot" (which was 2.0 on Slave), Slave should receive "Close 2.0 lot".
    assert_eq!(
        received_signal.lots,
        Some(2.0),
        "Lots should be calculated by mt-bridge (1.0 * 2.0 = 2.0)"
    );

    println!("✅ test_partial_close_signal passed");
}

/// Test full close signal (close_ratio = None)
/// Verifies backward compatibility - close without close_ratio works as full close
#[tokio::test]
async fn test_full_close_signal_no_ratio() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_FULLCLOSE_001";
    let slave_account = "SLAVE_FULLCLOSE_001";

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

    master.set_trade_allowed(true);
    master.start().expect("Failed to start master");
    slave.set_trade_allowed(true);
    slave.start().expect("Failed to start slave");
    sleep(Duration::from_millis(2000)).await;

    // Send full close signal (no close_ratio)
    let close_signal = master.create_close_signal(12346, "GBPUSD", 0.5);
    master
        .send_trade_signal(&close_signal)
        .expect("Failed to send signal");

    let received = slave
        .wait_for_trade_action(TradeAction::Close, 3000)
        .expect("Failed to receive signal");
    assert!(received.is_some(), "Should receive Close signal");

    let received_signal = received.unwrap();
    assert_eq!(received_signal.action, TradeAction::Close);
    assert_eq!(
        received_signal.close_ratio, None,
        "close_ratio should be None for full close"
    );

    println!("✅ test_full_close_signal_no_ratio passed");
}

// =============================================================================
// Symbol Filter Tests
// =============================================================================

/// Test allowed symbols filter - only specified symbols are copied
#[tokio::test]
async fn test_allowed_symbols_filter() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_ALLOWED_001";
    let slave_account = "SLAVE_ALLOWED_001";

    setup_test_scenario(&db, master_account, &[slave_account], |_| {
        let mut settings = default_test_slave_settings();
        settings.filters = TradeFilters {
            allowed_symbols: Some(vec!["EURUSD".to_string(), "GBPUSD".to_string()]),
            blocked_symbols: None,
            allowed_magic_numbers: None,
            blocked_magic_numbers: None,
        };
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

    // Wait for connection to be established (STATUS_CONNECTED = 2)
    slave
        .wait_for_status(2, 5000)
        .expect("Failed to reach CONNECTED status");

    // Allow time for Trade Topic subscription to propagate to Relay Server (ZMQ Slow Joiner)
    sleep(Duration::from_millis(1000)).await;

    // Send allowed symbol - should be received
    let signal =
        master.create_open_signal(12345, "EURUSD", OrderType::Buy, 0.1, 1.0850, None, None, 0);
    master
        .send_trade_signal(&signal)
        .expect("Failed to send signal");

    let received1 = slave
        .wait_for_trade_action(TradeAction::Open, 3000)
        .expect("Failed to receive signal");
    assert!(received1.is_some(), "EURUSD should be received (allowed)");

    // Send non-allowed symbol - should NOT be received
    let open_signal =
        master.create_open_signal(12345, "USDJPY", OrderType::Buy, 1.0, 150.00, None, None, 0);
    master
        .send_trade_signal(&open_signal)
        .expect("Failed to send signal");

    sleep(Duration::from_millis(500)).await;
    let received2 = slave
        .try_receive_trade_signal(500)
        .expect("Failed to receive signal");
    assert!(
        received2.is_none(),
        "USDJPY should NOT be received (not in allowed list)"
    );

    println!("✅ test_allowed_symbols_filter passed");
}

/// Test blocked symbols filter - specified symbols are excluded
#[tokio::test]
async fn test_blocked_symbols_filter() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_BLOCKED_001";
    let slave_account = "SLAVE_BLOCKED_001";

    setup_test_scenario(&db, master_account, &[slave_account], |_| {
        let mut settings = default_test_slave_settings();
        settings.filters = TradeFilters {
            allowed_symbols: None,
            blocked_symbols: Some(vec!["XAUUSD".to_string(), "XAGUSD".to_string()]),
            allowed_magic_numbers: None,
            blocked_magic_numbers: None,
        };
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

    // Send non-blocked symbol - should be received
    let signal1 =
        master.create_open_signal(12345, "EURUSD", OrderType::Buy, 0.1, 1.0850, None, None, 0);
    master
        .send_trade_signal(&signal1)
        .expect("Failed to send signal");

    let received1 = slave
        .wait_for_trade_action(TradeAction::Open, 3000)
        .expect("Failed to receive signal");
    assert!(
        received1.is_some(),
        "EURUSD should be received (not blocked)"
    );

    // Send blocked symbol - should NOT be received
    let open_signal =
        master.create_open_signal(12345, "XAUUSD", OrderType::Buy, 1.0, 2000.00, None, None, 0);
    master
        .send_trade_signal(&open_signal)
        .expect("Failed to send signal");

    sleep(Duration::from_millis(500)).await;
    let received2 = slave
        .try_receive_trade_signal(500)
        .expect("Failed to receive signal");
    assert!(
        received2.is_none(),
        "XAUUSD should NOT be received (blocked)"
    );

    println!("✅ test_blocked_symbols_filter passed");
}

// =============================================================================
// Magic Number Filter Tests
// =============================================================================

/// Test allowed magic numbers filter
#[tokio::test]
async fn test_allowed_magic_numbers_filter() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_MAGIC_ALLOW_001";
    let slave_account = "SLAVE_MAGIC_ALLOW_001";

    setup_test_scenario(&db, master_account, &[slave_account], |_| {
        let mut settings = default_test_slave_settings();
        settings.filters = TradeFilters {
            allowed_symbols: None,
            blocked_symbols: None,
            allowed_magic_numbers: Some(vec![12345, 67890]),
            blocked_magic_numbers: None,
        };
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

    // Send allowed magic number - should be received
    let signal1 = master.create_open_signal(
        1001,
        "EURUSD",
        OrderType::Buy,
        0.1,
        1.0850,
        None,
        None,
        12345,
    );
    master
        .send_trade_signal(&signal1)
        .expect("Failed to send signal");

    let received1 = slave
        .wait_for_trade_action(TradeAction::Open, 3000)
        .expect("Failed to receive signal");
    assert!(
        received1.is_some(),
        "Magic 12345 should be received (allowed)"
    );

    // Send non-allowed magic number - should NOT be received
    let signal2 = master.create_open_signal(
        1002,
        "EURUSD",
        OrderType::Buy,
        0.1,
        1.0850,
        None,
        None,
        99999,
    );
    master
        .send_trade_signal(&signal2)
        .expect("Failed to send signal");

    sleep(Duration::from_millis(500)).await;
    let received2 = slave
        .try_receive_trade_signal(500)
        .expect("Failed to receive signal");
    assert!(
        received2.is_none(),
        "Magic 99999 should NOT be received (not allowed)"
    );

    println!("✅ test_allowed_magic_numbers_filter passed");
}

/// Test blocked magic numbers filter
#[tokio::test]
async fn test_blocked_magic_numbers_filter() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_MAGIC_BLOCK_001";
    let slave_account = "SLAVE_MAGIC_BLOCK_001";

    setup_test_scenario(&db, master_account, &[slave_account], |_| {
        let mut settings = default_test_slave_settings();
        settings.filters = TradeFilters {
            allowed_symbols: None,
            blocked_symbols: None,
            allowed_magic_numbers: None,
            blocked_magic_numbers: Some(vec![11111, 22222]),
        };
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

    // Send non-blocked magic number - should be received
    let signal1 = master.create_open_signal(
        1001,
        "EURUSD",
        OrderType::Buy,
        0.1,
        1.0850,
        None,
        None,
        33333,
    );
    master
        .send_trade_signal(&signal1)
        .expect("Failed to send signal");

    let received1 = slave
        .wait_for_trade_action(TradeAction::Open, 3000)
        .expect("Failed to receive signal");
    assert!(
        received1.is_some(),
        "Magic 33333 should be received (not blocked)"
    );

    // Send blocked magic number - should NOT be received
    let signal2 = master.create_open_signal(
        1002,
        "EURUSD",
        OrderType::Buy,
        0.1,
        1.0850,
        None,
        None,
        11111,
    );
    master
        .send_trade_signal(&signal2)
        .expect("Failed to send signal");

    sleep(Duration::from_millis(500)).await;
    let received2 = slave
        .try_receive_trade_signal(500)
        .expect("Failed to receive signal");
    assert!(
        received2.is_none(),
        "Magic 11111 should NOT be received (blocked)"
    );

    println!("✅ test_blocked_magic_numbers_filter passed");
}

// =============================================================================
// Source Lot Limits Tests
// =============================================================================

/// Test source_lot_min filter - signals below minimum are excluded
#[tokio::test]
async fn test_source_lot_min_filter() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_LOTMIN_001";
    let slave_account = "SLAVE_LOTMIN_001";

    setup_test_scenario(&db, master_account, &[slave_account], |_| {
        let mut settings = default_test_slave_settings();
        settings.source_lot_min = Some(0.5); // Minimum 0.5 lots
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

    // Send signal above minimum - should be received
    let open_signal =
        master.create_open_signal(12345, "EURUSD", OrderType::Buy, 1.0, 1.0800, None, None, 0);
    master
        .send_trade_signal(&open_signal)
        .expect("Failed to send signal");

    let received1 = slave
        .wait_for_trade_action(TradeAction::Open, 3000)
        .expect("Failed to receive signal");
    assert!(
        received1.is_some(),
        "1.0 lots should be received (>= 0.5 min)"
    );

    // Send signal below minimum - should NOT be received
    let signal2 =
        master.create_open_signal(12346, "EURUSD", OrderType::Buy, 0.1, 1.0850, None, None, 0);
    master
        .send_trade_signal(&signal2)
        .expect("Failed to send signal");

    sleep(Duration::from_millis(500)).await;
    let received2 = slave
        .try_receive_trade_signal(500)
        .expect("Failed to receive signal");
    assert!(
        received2.is_none(),
        "0.1 lots should NOT be received (< 0.5 min)"
    );

    println!("✅ test_source_lot_min_filter passed");
}

/// Test source_lot_max filter - signals above maximum are excluded
#[tokio::test]
async fn test_source_lot_max_filter() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_LOTMAX_001";
    let slave_account = "SLAVE_LOTMAX_001";

    setup_test_scenario(&db, master_account, &[slave_account], |_| {
        let mut settings = default_test_slave_settings();
        settings.source_lot_max = Some(1.0); // Maximum 1.0 lots
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

    // Send signal below maximum - should be received
    let open_signal = master.create_open_signal(
        12345,
        "EURUSD",
        OrderType::Buy,
        0.5,
        1.0850,
        None,
        None,
        1001,
    );
    master
        .send_trade_signal(&open_signal)
        .expect("Failed to send signal");

    let received1 = slave
        .wait_for_trade_action(TradeAction::Open, 3000)
        .expect("Failed to receive signal");
    assert!(
        received1.is_some(),
        "0.5 lots should be received (<= 1.0 max)"
    );

    // Send signal above maximum - should NOT be received
    let signal2 =
        master.create_open_signal(12346, "EURUSD", OrderType::Buy, 5.0, 1.0850, None, None, 0);
    master
        .send_trade_signal(&signal2)
        .expect("Failed to send signal");

    sleep(Duration::from_millis(500)).await;
    let received2 = slave
        .try_receive_trade_signal(500)
        .expect("Failed to receive signal");
    assert!(
        received2.is_none(),
        "5.0 lots should NOT be received (> 1.0 max)"
    );

    println!("✅ test_source_lot_max_filter passed");
}

// =============================================================================
// Multiple Partial Close Tests
// =============================================================================

/// Test multiple sequential partial closes
/// 1.0 lot -> 50% close -> 0.5 lot -> 50% close -> 0.25 lot
#[tokio::test]
async fn test_multiple_sequential_partial_closes() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_MULTI_PARTIAL_001";
    let slave_account = "SLAVE_MULTI_PARTIAL_001";

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

    master.set_trade_allowed(true);
    master.start().expect("Failed to start master");
    slave.set_trade_allowed(true);
    slave.start().expect("Failed to start slave");
    sleep(Duration::from_millis(2000)).await;

    // Open position with 1.0 lots
    let open_signal =
        master.create_open_signal(12345, "EURUSD", OrderType::Buy, 1.0, 1.0850, None, None, 0);
    master
        .send_trade_signal(&open_signal)
        .expect("Failed to send signal");

    let received_open = slave
        .wait_for_trade_action(TradeAction::Open, 3000)
        .expect("Failed to receive signal");
    assert!(received_open.is_some(), "Should receive Open signal");

    // First partial close: 50% (1.0 -> 0.5)
    let partial1 = master.create_partial_close_signal(12345, "EURUSD", 1.0, 0.5);
    master
        .send_trade_signal(&partial1)
        .expect("Failed to send signal");

    let received_partial1 = slave
        .wait_for_trade_action(TradeAction::Close, 3000)
        .expect("Failed to receive signal");
    assert!(
        received_partial1.is_some(),
        "Should receive first partial close"
    );
    assert_eq!(
        received_partial1.unwrap().close_ratio,
        Some(0.5),
        "First close_ratio should be 0.5"
    );

    // Second partial close: 50% of remaining (0.5 -> 0.25)
    let partial2 = master.create_partial_close_signal(12345, "EURUSD", 0.5, 0.5);
    master
        .send_trade_signal(&partial2)
        .expect("Failed to send signal");

    let received_partial2 = slave
        .wait_for_trade_action(TradeAction::Close, 3000)
        .expect("Failed to receive signal");
    assert!(
        received_partial2.is_some(),
        "Should receive second partial close"
    );
    assert_eq!(
        received_partial2.unwrap().close_ratio,
        Some(0.5),
        "Second close_ratio should be 0.5"
    );

    println!("✅ test_multiple_sequential_partial_closes passed");
}

// =============================================================================
// Pending Order Tests
// =============================================================================

/// Test pending order types (BuyLimit, SellLimit, BuyStop, SellStop)
#[tokio::test]
async fn test_pending_order_types() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_PENDING_001";
    let slave_account = "SLAVE_PENDING_001";

    setup_test_scenario(&db, master_account, &[slave_account], |_| {
        let mut settings = default_test_slave_settings();
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

    // Test BuyLimit
    let buy_limit = master.create_open_signal(
        1001,
        "EURUSD",
        OrderType::BuyLimit,
        0.1,
        1.0800,
        Some(1.0750),
        Some(1.0900),
        0,
    );
    master
        .send_trade_signal(&buy_limit)
        .expect("Failed to send signal");

    let received1 = slave
        .wait_for_trade_action(TradeAction::Open, 3000)
        .expect("Failed to receive signal");
    assert!(received1.is_some(), "BuyLimit should be received");
    assert_eq!(received1.unwrap().order_type, Some(OrderType::BuyLimit));

    // Test SellLimit
    let sell_limit = master.create_open_signal(
        1002,
        "EURUSD",
        OrderType::SellLimit,
        0.1,
        1.0900,
        Some(1.0950),
        Some(1.0800),
        0,
    );
    master
        .send_trade_signal(&sell_limit)
        .expect("Failed to send signal");

    let received2 = slave
        .wait_for_trade_action(TradeAction::Open, 3000)
        .expect("Failed to receive signal");
    assert!(received2.is_some(), "SellLimit should be received");
    assert_eq!(received2.unwrap().order_type, Some(OrderType::SellLimit));

    // Test BuyStop
    let buy_stop = master.create_open_signal(
        1003,
        "EURUSD",
        OrderType::BuyStop,
        0.1,
        1.0900,
        Some(1.0850),
        Some(1.1000),
        0,
    );
    master
        .send_trade_signal(&buy_stop)
        .expect("Failed to send signal");

    let received3 = slave
        .wait_for_trade_action(TradeAction::Open, 3000)
        .expect("Failed to receive signal");
    assert!(received3.is_some(), "BuyStop should be received");
    assert_eq!(received3.unwrap().order_type, Some(OrderType::BuyStop));

    // Test SellStop
    let sell_stop = master.create_open_signal(
        1004,
        "EURUSD",
        OrderType::SellStop,
        0.1,
        1.0800,
        Some(1.0850),
        Some(1.0700),
        0,
    );
    master
        .send_trade_signal(&sell_stop)
        .expect("Failed to send signal");

    let received4 = slave
        .wait_for_trade_action(TradeAction::Open, 3000)
        .expect("Failed to receive signal");
    assert!(received4.is_some(), "SellStop should be received");
    assert_eq!(received4.unwrap().order_type, Some(OrderType::SellStop));

    println!("✅ test_pending_order_types passed");
}

/// Test copy_pending_orders = false - pending orders should not be copied
#[tokio::test]
async fn test_pending_orders_disabled() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_NO_PENDING_001";
    let slave_account = "SLAVE_NO_PENDING_001";

    setup_test_scenario(&db, master_account, &[slave_account], |_| {
        let mut settings = default_test_slave_settings();
        settings.copy_pending_orders = false; // Disabled
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

    // Wait for connection to be established
    slave
        .wait_for_status(2, 5000)
        .expect("Failed to reach CONNECTED status");
    sleep(Duration::from_millis(1000)).await;

    // Market order should be received
    let market_order =
        master.create_open_signal(1001, "EURUSD", OrderType::Buy, 0.1, 1.0850, None, None, 0);
    master
        .send_trade_signal(&market_order)
        .expect("Failed to send signal");

    let received1 = slave
        .wait_for_trade_action(TradeAction::Open, 3000)
        .expect("Failed to receive signal");
    assert!(received1.is_some(), "Market order (Buy) should be received");

    // Pending order should NOT be received
    let pending_order = master.create_open_signal(
        1002,
        "EURUSD",
        OrderType::BuyLimit,
        0.1,
        1.0800,
        None,
        None,
        0,
    );
    master
        .send_trade_signal(&pending_order)
        .expect("Failed to send signal");

    sleep(Duration::from_millis(500)).await;
    let received2 = slave
        .try_receive_trade_signal(500)
        .expect("Failed to receive signal");
    assert!(
        received2.is_none(),
        "Pending order (BuyLimit) should NOT be received when copy_pending_orders=false"
    );

    println!("✅ test_pending_orders_disabled passed");
}
