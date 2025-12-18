// e2e-tests/tests/multi_master_concurrency.rs
//
// E2E integration tests for Multi-Master concurrency.
// Verifies that multiple Master-Slave pairs can operate simultaneously
// on the same Relay Server without cross-talk or data loss.

use e2e_tests::helpers::{default_test_slave_settings, enable_member_intent, setup_test_db};
use e2e_tests::TestSandbox;
use sankey_copier_relay_server::adapters::outbound::persistence::Database;
use sankey_copier_relay_server::domain::models::OrderType;
use tokio::time::{sleep, Duration};

/// Test that two independent Master-Slave pairs can operate concurrently.
/// - Master A -> Slave A
/// - Master B -> Slave B
/// Verifies correct routing (no cross-talk) and no signal loss.
#[tokio::test]
async fn test_dual_master_routing() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();
    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to DB");

    // Setup Pair A
    let master_a = "MASTER_A";
    let slave_a = "SLAVE_A";
    setup_test_db(&db, master_a, &[slave_a], |_| default_test_slave_settings())
        .await
        .expect("Failed to setup Pair A");
    enable_member_intent(&db, master_a, slave_a)
        .await
        .expect("Failed to enable Slave A");

    // Setup Pair B
    let master_b = "MASTER_B";
    let slave_b = "SLAVE_B";
    setup_test_db(&db, master_b, &[slave_b], |_| default_test_slave_settings())
        .await
        .expect("Failed to setup Pair B");
    enable_member_intent(&db, master_b, slave_b)
        .await
        .expect("Failed to enable Slave B");

    // Create Simulators
    let mut sim_master_a = sandbox
        .create_master(master_a, true)
        .expect("Failed Master A");
    let mut sim_slave_a = sandbox
        .create_slave(slave_a, master_a, true)
        .expect("Failed Slave A");

    let mut sim_master_b = sandbox
        .create_master(master_b, true)
        .expect("Failed Master B");
    let mut sim_slave_b = sandbox
        .create_slave(slave_b, master_b, true)
        .expect("Failed Slave B");

    // Start Masters
    sim_master_a.start().expect("Start Master A");
    sim_master_b.start().expect("Start Master B");
    // Wait for masters to be online and PUSH sockets connected
    sleep(Duration::from_millis(1000)).await;

    // Start Slaves
    sim_slave_a.start().expect("Start Slave A");
    sim_slave_b.start().expect("Start Slave B");

    // Wait for Slaves to reach CONNECTED status (2)
    // This ensures full end-to-end connectivity before testing
    let config_a = sim_slave_a.wait_for_status(2, 5000).expect("Wait Status A");
    assert!(config_a.is_some(), "Slave A failed to connect");

    let config_b = sim_slave_b.wait_for_status(2, 5000).expect("Wait Status B");
    assert!(config_b.is_some(), "Slave B failed to connect");

    println!("All systems connected. Starting concurrency test...");

    // --- Scenario: Concurrent Open ---
    println!("--- Testing Concurrent Open ---");
    let ticket_a = 1000;
    let ticket_b = 2000;

    let signal_a = sim_master_a.create_open_signal(
        ticket_a,
        "EURUSD",
        OrderType::Buy,
        0.1,
        1.1000,
        None,
        None,
        1,
    );
    let signal_b = sim_master_b.create_open_signal(
        ticket_b,
        "GBPUSD",
        OrderType::Sell,
        0.2,
        1.2500,
        None,
        None,
        2,
    );

    // Send "simultaneously" (sequential but fast)
    sim_master_a.send_trade_signal(&signal_a).expect("Send A");
    sim_master_b.send_trade_signal(&signal_b).expect("Send B");

    // Verify Slave A received ONLY Signal A
    let received_a = sim_slave_a.try_receive_trade_signal(5000).expect("Recv A");
    assert!(received_a.is_some(), "Slave A missed signal");
    let sig_a = received_a.unwrap();
    assert_eq!(sig_a.ticket, ticket_a, "Slave A got wrong ticket");
    assert_eq!(sig_a.symbol.unwrap(), "EURUSD");

    // Verify Slave B received ONLY Signal B
    let received_b = sim_slave_b.try_receive_trade_signal(5000).expect("Recv B");
    assert!(received_b.is_some(), "Slave B missed signal");
    let sig_b = received_b.unwrap();
    assert_eq!(sig_b.ticket, ticket_b, "Slave B got wrong ticket");
    assert_eq!(sig_b.symbol.unwrap(), "GBPUSD");

    // Ensure no cross-talk (check buffer empty)
    // Small sleep to ensure any stray messages arrived
    sleep(Duration::from_millis(500)).await;
    assert!(
        sim_slave_a
            .try_receive_trade_signal(100)
            .expect("Check A empty")
            .is_none(),
        "Slave A got extra signal"
    );
    assert!(
        sim_slave_b
            .try_receive_trade_signal(100)
            .expect("Check B empty")
            .is_none(),
        "Slave B got extra signal"
    );

    println!("✅ Concurrent Open Passed");

    // --- Scenario: Concurrent Modify ---
    println!("--- Testing Concurrent Modify ---");
    let mod_a = sim_master_a.create_modify_signal(ticket_a, "EURUSD", Some(1.0900), Some(1.1100));
    let mod_b = sim_master_b.create_modify_signal(ticket_b, "GBPUSD", Some(1.2600), Some(1.2400));

    sim_master_b.send_trade_signal(&mod_b).expect("Send Mod B"); // Reverse order for variety
    sim_master_a.send_trade_signal(&mod_a).expect("Send Mod A");

    let rec_mod_a = sim_slave_a
        .try_receive_trade_signal(5000)
        .expect("Recv Mod A");
    assert!(rec_mod_a.is_some(), "Slave A missed modify");
    let mod_sig_a = rec_mod_a.unwrap();
    assert_eq!(mod_sig_a.action, e2e_tests::TradeAction::Modify);
    assert_eq!(mod_sig_a.stop_loss, Some(1.0900));

    let rec_mod_b = sim_slave_b
        .try_receive_trade_signal(5000)
        .expect("Recv Mod B");
    assert!(rec_mod_b.is_some(), "Slave B missed modify");
    let mod_sig_b = rec_mod_b.unwrap();
    assert_eq!(mod_sig_b.action, e2e_tests::TradeAction::Modify);
    assert_eq!(mod_sig_b.stop_loss, Some(1.2600));

    println!("✅ Concurrent Modify Passed");

    // --- Scenario: Concurrent Close ---
    println!("--- Testing Concurrent Close ---");
    let close_a = sim_master_a.create_close_signal(ticket_a, "EURUSD", 0.1);
    let close_b = sim_master_b.create_close_signal(ticket_b, "GBPUSD", 0.2);

    sim_master_a
        .send_trade_signal(&close_a)
        .expect("Send Close A");
    sim_master_b
        .send_trade_signal(&close_b)
        .expect("Send Close B");

    let rec_close_a = sim_slave_a
        .try_receive_trade_signal(5000)
        .expect("Recv Close A");
    assert!(rec_close_a.is_some(), "Slave A missed close");
    assert_eq!(rec_close_a.unwrap().action, e2e_tests::TradeAction::Close);

    let rec_close_b = sim_slave_b
        .try_receive_trade_signal(5000)
        .expect("Recv Close B");
    assert!(rec_close_b.is_some(), "Slave B missed close");
    assert_eq!(rec_close_b.unwrap().action, e2e_tests::TradeAction::Close);

    println!("✅ Concurrent Close Passed");
}

/// Stress test with higher volume and 3 pairs
#[tokio::test]
async fn test_multi_master_stress_test() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();
    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to DB");

    const NUM_PAIRS: usize = 3;
    const NUM_CYCLES: usize = 20; // 20 cycles * 3 pairs = 60 orders total

    let mut masters = Vec::new();
    let mut slaves = Vec::new();

    // Setup 3 Pairs
    for i in 0..NUM_PAIRS {
        let master_acc = format!("MASTER_S_{}", i);
        let slave_acc = format!("SLAVE_S_{}", i);

        setup_test_db(&db, &master_acc, &[&slave_acc], |_| {
            default_test_slave_settings()
        })
        .await
        .expect("Setup DB");
        enable_member_intent(&db, &master_acc, &slave_acc)
            .await
            .expect("Enable Intent");

        let mut m = sandbox
            .create_master(&master_acc, true)
            .expect("Create Master");
        let mut s = sandbox
            .create_slave(&slave_acc, &master_acc, true)
            .expect("Create Slave");

        m.start().expect("Start Master");
        s.start().expect("Start Slave");

        masters.push(m);
        slaves.push(s);
    }

    sleep(Duration::from_millis(2000)).await; // Wait for initial connections

    // Wait for all to be ready
    for s in slaves.iter() {
        assert!(
            s.wait_for_status(2, 5000).expect("Wait").is_some(),
            "Slave failed ready"
        );
    }

    println!(
        "--- Starting Stress Test ({} Pairs, {} Cycles) ---",
        NUM_PAIRS, NUM_CYCLES
    );

    for cycle in 0..NUM_CYCLES {
        // 1. OPEN ALL
        for (i, master) in masters.iter().enumerate() {
            let ticket = (cycle * 1000 + i) as i64;
            let sig = master.create_open_signal(
                ticket,
                "EURUSD",
                OrderType::Buy,
                0.1,
                1.0,
                None,
                None,
                i as i64,
            );
            master.send_trade_signal(&sig).expect("Send Open");
        }

        // Verify Open
        for (i, slave) in slaves.iter().enumerate() {
            let received = slave.try_receive_trade_signal(5000).expect("Recv Open");
            assert!(received.is_some(), "Cycle {} Pair {} missed Open", cycle, i);
            assert_eq!(received.unwrap().ticket, (cycle * 1000 + i) as i64);
        }

        // 2. MODIFY ALL
        for (i, master) in masters.iter().enumerate() {
            let ticket = (cycle * 1000 + i) as i64;
            let sig = master.create_modify_signal(ticket, "EURUSD", Some(1.1), None);
            master.send_trade_signal(&sig).expect("Send Modify");
        }

        // Verify Modify
        for (i, slave) in slaves.iter().enumerate() {
            let received = slave.try_receive_trade_signal(5000).expect("Recv Modify");
            assert!(
                received.is_some(),
                "Cycle {} Pair {} missed Modify",
                cycle,
                i
            );
            assert_eq!(received.unwrap().action, e2e_tests::TradeAction::Modify);
        }

        // 3. CLOSE ALL
        for (i, master) in masters.iter().enumerate() {
            let ticket = (cycle * 1000 + i) as i64;
            let sig = master.create_close_signal(ticket, "EURUSD", 0.1);
            master.send_trade_signal(&sig).expect("Send Close");
        }

        // Verify Close
        for (i, slave) in slaves.iter().enumerate() {
            let received = slave.try_receive_trade_signal(5000).expect("Recv Close");
            assert!(
                received.is_some(),
                "Cycle {} Pair {} missed Close",
                cycle,
                i
            );
            assert_eq!(received.unwrap().action, e2e_tests::TradeAction::Close);
        }

        if cycle % 5 == 0 {
            println!("Completed Cycle {}", cycle);
        }
    }

    println!("✅ Stress Test Passed");
}
