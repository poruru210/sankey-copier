// e2e-tests/tests/order_lifecycle.rs
//
// E2E integration tests for basic order lifecycle.
// Tests Open/Close/Modify cycles and multiple order handling.
//
// Migrated from relay-server/tests/e2e_trade_signal_test.rs

use e2e_tests::helpers::{default_test_slave_settings, setup_test_scenario};
use e2e_tests::relay_server_process::RelayServerProcess;
use e2e_tests::{MasterEaSimulator, SlaveEaSimulator};
use sankey_copier_relay_server::db::Database;
use sankey_copier_relay_server::models::OrderType;
use tokio::time::{sleep, Duration};

// =============================================================================
// Helper Functions
// =============================================================================

fn order_type_to_string(order_type: OrderType) -> &'static str {
    match order_type {
        OrderType::Buy => "Buy",
        OrderType::Sell => "Sell",
        OrderType::BuyLimit => "BuyLimit",
        OrderType::SellLimit => "SellLimit",
        OrderType::BuyStop => "BuyStop",
        OrderType::SellStop => "SellStop",
    }
}

// =============================================================================
// Basic Order Lifecycle Tests
// =============================================================================

/// Test basic Open -> Close cycle
#[tokio::test]
async fn test_open_close_cycle() {
    let server = RelayServerProcess::start().expect("Failed to start relay-server");

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_OPEN_CLOSE_001";
    let slave_account = "SLAVE_OPEN_CLOSE_001";

    setup_test_scenario(&db, master_account, &[slave_account], |_| {
        default_test_slave_settings()
    })
    .await
    .expect("Failed to setup test scenario");

    let mut master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_address(),
        master_account,
    )
    .expect("Failed to create master simulator");

    let mut slave = SlaveEaSimulator::new(&server.zmq_pull_address(), &server.zmq_pub_address(), &server.zmq_pub_address(), slave_account,
        master_account,
    )
    .expect("Failed to create slave simulator");

    master.set_trade_allowed(true);
    master.start().expect("Failed to start master");
    slave.set_trade_allowed(true);
    slave.start().expect("Failed to start slave");
    sleep(Duration::from_millis(500)).await;

    // Step 1: Master sends Open signal
    let open_signal = master.create_open_signal(
        12345,
        "EURUSD",
        order_type_to_string(OrderType::Buy),
        0.1,
        1.0850,
        Some(1.0800),
        Some(1.0900),
        0,
    );
    master
        .send_trade_signal(&open_signal)
        .expect("Failed to send Open signal");

    sleep(Duration::from_millis(500)).await;

    // Step 2: Slave receives the Open signal
    let received_open = slave
        .try_receive_trade_signal(2000)
        .expect("Failed to receive trade signal");

    assert!(received_open.is_some(), "Slave should receive Open signal");
    let open_sig = received_open.unwrap();
    assert_eq!(open_sig.action, "Open");
    assert_eq!(open_sig.ticket, 12345);
    assert_eq!(open_sig.symbol.as_deref(), Some("EURUSD"));
    assert_eq!(open_sig.lots, Some(0.1));

    // Step 3: Master sends Close signal
    let close_signal = master.create_close_signal(12345, "EURUSD", 0.1);
    master
        .send_trade_signal(&close_signal)
        .expect("Failed to send Close signal");

    sleep(Duration::from_millis(500)).await;

    // Step 4: Slave receives the Close signal
    let received_close = slave
        .try_receive_trade_signal(2000)
        .expect("Failed to receive close signal");

    assert!(
        received_close.is_some(),
        "Slave should receive Close signal"
    );
    let close_sig = received_close.unwrap();
    assert_eq!(close_sig.action, "Close");
    assert_eq!(close_sig.ticket, 12345);

    println!("✅ test_open_close_cycle passed");
}

/// Test Open -> Modify -> Close cycle
#[tokio::test]
async fn test_open_modify_close_cycle() {
    let server = RelayServerProcess::start().expect("Failed to start relay-server");

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_MODIFY_001";
    let slave_account = "SLAVE_MODIFY_001";

    setup_test_scenario(&db, master_account, &[slave_account], |_| {
        default_test_slave_settings()
    })
    .await
    .expect("Failed to setup test scenario");

    let mut master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_address(),
        master_account,
    )
    .expect("Failed to create master simulator");

    let mut slave = SlaveEaSimulator::new(&server.zmq_pull_address(), &server.zmq_pub_address(), &server.zmq_pub_address(), slave_account,
        master_account,
    )
    .expect("Failed to create slave simulator");

    master.set_trade_allowed(true);
    master.start().expect("Failed to start master");
    slave.set_trade_allowed(true);
    slave.start().expect("Failed to start slave");
    sleep(Duration::from_millis(500)).await;

    // Open
    let open_signal = master.create_open_signal(
        22222,
        "GBPUSD",
        order_type_to_string(OrderType::Sell),
        0.2,
        1.2500,
        Some(1.2600),
        Some(1.2400),
        100,
    );
    master.send_trade_signal(&open_signal).expect("Open failed");
    sleep(Duration::from_millis(300)).await;

    let received = slave
        .try_receive_trade_signal(2000)
        .expect("Receive failed");
    assert!(received.is_some(), "Should receive Open");
    assert_eq!(received.unwrap().action, "Open");

    // Modify
    let modify_signal = master.create_modify_signal(22222, "GBPUSD", Some(1.2550), Some(1.2350));
    master
        .send_trade_signal(&modify_signal)
        .expect("Modify failed");
    sleep(Duration::from_millis(300)).await;

    let received = slave
        .try_receive_trade_signal(2000)
        .expect("Receive failed");
    assert!(received.is_some(), "Should receive Modify");
    let mod_sig = received.unwrap();
    assert_eq!(mod_sig.action, "Modify");
    assert_eq!(mod_sig.stop_loss, Some(1.2550));
    assert_eq!(mod_sig.take_profit, Some(1.2350));

    // Close
    let close_signal = master.create_close_signal(22222, "GBPUSD", 0.2);
    master
        .send_trade_signal(&close_signal)
        .expect("Close failed");
    sleep(Duration::from_millis(300)).await;

    let received = slave
        .try_receive_trade_signal(2000)
        .expect("Receive failed");
    assert!(received.is_some(), "Should receive Close");
    assert_eq!(received.unwrap().action, "Close");

    println!("✅ test_open_modify_close_cycle passed");
}

/// Test modify SL only
#[tokio::test]
async fn test_modify_sl_only() {
    let server = RelayServerProcess::start().expect("Failed to start relay-server");

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_MODIFY_SL_001";
    let slave_account = "SLAVE_MODIFY_SL_001";

    setup_test_scenario(&db, master_account, &[slave_account], |_| {
        default_test_slave_settings()
    })
    .await
    .expect("Failed to setup test scenario");

    let mut master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_address(),
        master_account,
    )
    .expect("Failed to create master simulator");

    let mut slave = SlaveEaSimulator::new(&server.zmq_pull_address(), &server.zmq_pub_address(), &server.zmq_pub_address(), slave_account,
        master_account,
    )
    .expect("Failed to create slave simulator");

    master.set_trade_allowed(true);
    master.start().expect("Failed to start master");
    slave.set_trade_allowed(true);
    slave.start().expect("Failed to start slave");
    sleep(Duration::from_millis(500)).await;

    // Open position first
    let open_signal = master.create_open_signal(
        33333,
        "USDJPY",
        order_type_to_string(OrderType::Buy),
        0.5,
        150.00,
        None,
        None,
        0,
    );
    master.send_trade_signal(&open_signal).unwrap();
    sleep(Duration::from_millis(300)).await;
    let _ = slave.try_receive_trade_signal(2000).unwrap();

    // Modify SL only
    let modify_signal = master.create_modify_signal(33333, "USDJPY", Some(149.50), None);
    master.send_trade_signal(&modify_signal).unwrap();
    sleep(Duration::from_millis(300)).await;

    let received = slave.try_receive_trade_signal(2000).unwrap();
    assert!(received.is_some(), "Should receive Modify");
    let mod_sig = received.unwrap();
    assert_eq!(mod_sig.action, "Modify");
    assert_eq!(mod_sig.stop_loss, Some(149.50));
    assert_eq!(mod_sig.take_profit, None);

    println!("✅ test_modify_sl_only passed");
}

/// Test modify TP only
#[tokio::test]
async fn test_modify_tp_only() {
    let server = RelayServerProcess::start().expect("Failed to start relay-server");

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_MODIFY_TP_001";
    let slave_account = "SLAVE_MODIFY_TP_001";

    setup_test_scenario(&db, master_account, &[slave_account], |_| {
        default_test_slave_settings()
    })
    .await
    .expect("Failed to setup test scenario");

    let mut master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_address(),
        master_account,
    )
    .expect("Failed to create master simulator");

    let mut slave = SlaveEaSimulator::new(&server.zmq_pull_address(), &server.zmq_pub_address(), &server.zmq_pub_address(), slave_account,
        master_account,
    )
    .expect("Failed to create slave simulator");

    master.set_trade_allowed(true);
    master.start().expect("Failed to start master");
    slave.set_trade_allowed(true);
    slave.start().expect("Failed to start slave");
    sleep(Duration::from_millis(500)).await;

    // Open position first
    let open_signal = master.create_open_signal(
        44444,
        "EURUSD",
        order_type_to_string(OrderType::Sell),
        0.3,
        1.0900,
        None,
        None,
        0,
    );
    master.send_trade_signal(&open_signal).unwrap();
    sleep(Duration::from_millis(300)).await;
    let _ = slave.try_receive_trade_signal(2000).unwrap();

    // Modify TP only
    let modify_signal = master.create_modify_signal(44444, "EURUSD", None, Some(1.0800));
    master.send_trade_signal(&modify_signal).unwrap();
    sleep(Duration::from_millis(300)).await;

    let received = slave.try_receive_trade_signal(2000).unwrap();
    assert!(received.is_some(), "Should receive Modify");
    let mod_sig = received.unwrap();
    assert_eq!(mod_sig.action, "Modify");
    assert_eq!(mod_sig.stop_loss, None);
    assert_eq!(mod_sig.take_profit, Some(1.0800));

    println!("✅ test_modify_tp_only passed");
}

/// Test modify both SL and TP
#[tokio::test]
async fn test_modify_both_sl_tp() {
    let server = RelayServerProcess::start().expect("Failed to start relay-server");

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_MODIFY_BOTH_001";
    let slave_account = "SLAVE_MODIFY_BOTH_001";

    setup_test_scenario(&db, master_account, &[slave_account], |_| {
        default_test_slave_settings()
    })
    .await
    .expect("Failed to setup test scenario");

    let mut master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_address(),
        master_account,
    )
    .expect("Failed to create master simulator");

    let mut slave = SlaveEaSimulator::new(&server.zmq_pull_address(), &server.zmq_pub_address(), &server.zmq_pub_address(), slave_account,
        master_account,
    )
    .expect("Failed to create slave simulator");

    master.set_trade_allowed(true);
    master.start().expect("Failed to start master");
    slave.set_trade_allowed(true);
    slave.start().expect("Failed to start slave");
    sleep(Duration::from_millis(500)).await;

    // Open position first
    let open_signal = master.create_open_signal(
        55555,
        "GBPJPY",
        order_type_to_string(OrderType::Buy),
        0.2,
        188.00,
        None,
        None,
        0,
    );
    master.send_trade_signal(&open_signal).unwrap();
    sleep(Duration::from_millis(300)).await;
    let _ = slave.try_receive_trade_signal(2000).unwrap();

    // Modify both SL and TP
    let modify_signal = master.create_modify_signal(55555, "GBPJPY", Some(187.00), Some(190.00));
    master.send_trade_signal(&modify_signal).unwrap();
    sleep(Duration::from_millis(300)).await;

    let received = slave.try_receive_trade_signal(2000).unwrap();
    assert!(received.is_some(), "Should receive Modify");
    let mod_sig = received.unwrap();
    assert_eq!(mod_sig.action, "Modify");
    assert_eq!(mod_sig.stop_loss, Some(187.00));
    assert_eq!(mod_sig.take_profit, Some(190.00));

    println!("✅ test_modify_both_sl_tp passed");
}

/// Test multiple sequential opens
#[tokio::test]
async fn test_multiple_open_sequential() {
    let server = RelayServerProcess::start().expect("Failed to start relay-server");

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_MULTI_OPEN_001";
    let slave_account = "SLAVE_MULTI_OPEN_001";

    setup_test_scenario(&db, master_account, &[slave_account], |_| {
        default_test_slave_settings()
    })
    .await
    .expect("Failed to setup test scenario");

    let mut master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_address(),
        master_account,
    )
    .expect("Failed to create master simulator");

    let mut slave = SlaveEaSimulator::new(&server.zmq_pull_address(), &server.zmq_pub_address(), &server.zmq_pub_address(), slave_account,
        master_account,
    )
    .expect("Failed to create slave simulator");

    master.set_trade_allowed(true);
    master.start().expect("Failed to start master");
    slave.set_trade_allowed(true);
    slave.start().expect("Failed to start slave");
    sleep(Duration::from_millis(500)).await;

    // Open 3 positions sequentially
    for i in 0..3 {
        let ticket = 60000 + i;
        let open_signal = master.create_open_signal(
            ticket,
            "EURUSD",
            order_type_to_string(OrderType::Buy),
            0.1,
            1.0850 + (i as f64 * 0.001),
            None,
            None,
            0,
        );
        master.send_trade_signal(&open_signal).unwrap();
        sleep(Duration::from_millis(200)).await;

        let received = slave.try_receive_trade_signal(2000).unwrap();
        assert!(
            received.is_some(),
            "Should receive Open signal for ticket {}",
            ticket
        );
        assert_eq!(received.unwrap().ticket, ticket);
    }

    println!("✅ test_multiple_open_sequential passed");
}

/// Test rapid fire signals (high throughput)
#[tokio::test]
async fn test_rapid_fire_signals() {
    let server = RelayServerProcess::start().expect("Failed to start relay-server");

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_RAPID_001";
    let slave_account = "SLAVE_RAPID_001";

    setup_test_scenario(&db, master_account, &[slave_account], |_| {
        default_test_slave_settings()
    })
    .await
    .expect("Failed to setup test scenario");

    let mut master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_address(),
        master_account,
    )
    .expect("Failed to create master simulator");

    let mut slave = SlaveEaSimulator::new(&server.zmq_pull_address(), &server.zmq_pub_address(), &server.zmq_pub_address(), slave_account,
        master_account,
    )
    .expect("Failed to create slave simulator");

    master.set_trade_allowed(true);
    master.start().expect("Failed to start master");
    slave.set_trade_allowed(true);
    slave.start().expect("Failed to start slave");
    sleep(Duration::from_millis(500)).await;

    // Send 10 signals rapidly
    let signal_count = 10;
    for i in 0..signal_count {
        let ticket = 70000 + i;
        let open_signal = master.create_open_signal(
            ticket,
            "EURUSD",
            order_type_to_string(OrderType::Buy),
            0.1,
            1.0850,
            None,
            None,
            0,
        );
        master.send_trade_signal(&open_signal).unwrap();
        // No sleep between sends - rapid fire
    }

    // Give time for all signals to propagate
    sleep(Duration::from_millis(1000)).await;

    // Receive all signals
    let mut received_count = 0;
    for _ in 0..signal_count {
        if slave.try_receive_trade_signal(500).unwrap().is_some() {
            received_count += 1;
        }
    }

    assert!(
        received_count >= signal_count - 1,
        "Should receive most signals ({}/{})",
        received_count,
        signal_count
    );

    println!(
        "✅ test_rapid_fire_signals passed ({}/{} signals received)",
        received_count, signal_count
    );
}

// =============================================================================
// Edge Case Tests
// =============================================================================

/// Test Close signal for non-existent position (should still be relayed)
/// Server doesn't track position state - all signals are relayed
#[tokio::test]
async fn test_close_nonexistent_position() {
    let server = RelayServerProcess::start().expect("Failed to start relay-server");

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_NONEXIST_001";
    let slave_account = "SLAVE_NONEXIST_001";

    setup_test_scenario(&db, master_account, &[slave_account], |_| {
        default_test_slave_settings()
    })
    .await
    .expect("Failed to setup test scenario");

    let mut master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_address(),
        master_account,
    )
    .expect("Failed to create master simulator");

    let mut slave = SlaveEaSimulator::new(&server.zmq_pull_address(), &server.zmq_pub_address(), &server.zmq_pub_address(), slave_account,
        master_account,
    )
    .expect("Failed to create slave simulator");

    master.set_trade_allowed(true);
    master.start().expect("Failed to start master");
    slave.set_trade_allowed(true);
    slave.start().expect("Failed to start slave");
    sleep(Duration::from_millis(500)).await;

    // Send Close for ticket that was never opened
    let close_signal = master.create_close_signal(99999, "EURUSD", 0.1);
    master.send_trade_signal(&close_signal).expect("Failed to send signal");

    sleep(Duration::from_millis(200)).await;
    let received = slave.try_receive_trade_signal(3000).expect("Failed to receive");

    // Server should still relay the signal (doesn't track position state)
    assert!(
        received.is_some(),
        "Close signal for non-existent position should be relayed"
    );
    let signal = received.unwrap();
    assert_eq!(signal.action, "Close");
    assert_eq!(signal.ticket, 99999);

    println!("✅ test_close_nonexistent_position passed");
}

/// Test duplicate Close signals (double close)
/// Server doesn't deduplicate - all signals are relayed
#[tokio::test]
async fn test_close_already_closed() {
    let server = RelayServerProcess::start().expect("Failed to start relay-server");

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_DOUBLE_CLOSE_001";
    let slave_account = "SLAVE_DOUBLE_CLOSE_001";

    setup_test_scenario(&db, master_account, &[slave_account], |_| {
        default_test_slave_settings()
    })
    .await
    .expect("Failed to setup test scenario");

    let mut master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_address(),
        master_account,
    )
    .expect("Failed to create master simulator");

    let mut slave = SlaveEaSimulator::new(&server.zmq_pull_address(), &server.zmq_pub_address(), &server.zmq_pub_address(), slave_account,
        master_account,
    )
    .expect("Failed to create slave simulator");

    master.set_trade_allowed(true);
    master.start().expect("Failed to start master");
    slave.set_trade_allowed(true);
    slave.start().expect("Failed to start slave");
    sleep(Duration::from_millis(500)).await;

    // Open
    let open_signal = master.create_open_signal(
        12347,
        "EURUSD",
        order_type_to_string(OrderType::Buy),
        0.1,
        1.0850,
        None,
        None,
        0,
    );
    master.send_trade_signal(&open_signal).expect("Failed to send signal");
    sleep(Duration::from_millis(100)).await;

    // First Close
    let close1 = master.create_close_signal(12347, "EURUSD", 0.1);
    master.send_trade_signal(&close1).expect("Failed to send signal");
    sleep(Duration::from_millis(100)).await;

    // Second Close (duplicate)
    let close2 = master.create_close_signal(12347, "EURUSD", 0.1);
    master.send_trade_signal(&close2).expect("Failed to send signal");

    sleep(Duration::from_millis(200)).await;

    // Collect all signals
    let mut signals = Vec::new();
    for _ in 0..3 {
        if let Some(signal) = slave.try_receive_trade_signal(1000).expect("Failed to receive") {
            signals.push(signal);
        }
    }

    // Server doesn't deduplicate - all 3 signals should be delivered
    assert_eq!(
        signals.len(),
        3,
        "All signals should be delivered (dedup is EA's job)"
    );

    println!("✅ test_close_already_closed passed");
}

/// Test multiple Modify signals in sequence
#[tokio::test]
async fn test_modify_multiple_times() {
    let server = RelayServerProcess::start().expect("Failed to start relay-server");

    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "MASTER_MULTI_MODIFY_001";
    let slave_account = "SLAVE_MULTI_MODIFY_001";

    setup_test_scenario(&db, master_account, &[slave_account], |_| {
        default_test_slave_settings()
    })
    .await
    .expect("Failed to setup test scenario");

    let mut master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_address(),
        master_account,
    )
    .expect("Failed to create master simulator");

    let mut slave = SlaveEaSimulator::new(&server.zmq_pull_address(), &server.zmq_pub_address(), &server.zmq_pub_address(), slave_account,
        master_account,
    )
    .expect("Failed to create slave simulator");

    master.set_trade_allowed(true);
    master.start().expect("Failed to start master");
    slave.set_trade_allowed(true);
    slave.start().expect("Failed to start slave");
    sleep(Duration::from_millis(500)).await;

    // Send 3 Modify signals with different SL/TP values
    let modify1 = master.create_modify_signal(12351, "EURUSD", Some(1.0800), Some(1.0900));
    master.send_trade_signal(&modify1).expect("Failed to send signal");
    sleep(Duration::from_millis(100)).await;

    let modify2 = master.create_modify_signal(12351, "EURUSD", Some(1.0750), Some(1.0950));
    master.send_trade_signal(&modify2).expect("Failed to send signal");
    sleep(Duration::from_millis(100)).await;

    let modify3 = master.create_modify_signal(12351, "EURUSD", Some(1.0700), Some(1.1000));
    master.send_trade_signal(&modify3).expect("Failed to send signal");

    sleep(Duration::from_millis(200)).await;

    // Collect all signals
    let mut signals = Vec::new();
    for _ in 0..3 {
        if let Some(signal) = slave.try_receive_trade_signal(1000).expect("Failed to receive") {
            signals.push(signal);
        }
    }

    assert_eq!(signals.len(), 3, "Should receive all 3 Modify signals");

    // Verify values in order
    assert_eq!(signals[0].stop_loss, Some(1.0800));
    assert_eq!(signals[1].stop_loss, Some(1.0750));
    assert_eq!(signals[2].stop_loss, Some(1.0700));

    println!("✅ test_modify_multiple_times passed");
}

