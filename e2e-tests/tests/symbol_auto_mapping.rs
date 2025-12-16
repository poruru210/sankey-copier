// e2e-tests/tests/symbol_auto_mapping.rs
//
// Integration test to verify Auto Symbol Mapping logic.
// Simulates a Slave EA reporting "GOLD" and a Master sending "XAUUSD" trades.
// Verifies that the Relay Server maps "XAUUSD" to "GOLD" using synonym groups.

use e2e_tests::{TestSandbox, STATUS_CONNECTED};

use std::time::Duration;
use tokio::time::sleep;

/// Test full auto-mapping flow:
/// 1. Configure Relay Server with synonym: [XAUUSD, GOLD]
/// 2. Register Master (Master_001) sending XAUUSD trades
/// 3. Register Slave (Slave_001) reporting detected_symbols=["GOLD"]
/// 4. Master opens trade on XAUUSD
/// 5. Verify Slave receives trade mapped to GOLD
#[tokio::test]
async fn test_auto_mapping_xauusd_to_gold() {
    use e2e_tests::helpers::{default_test_slave_settings, setup_test_scenario};
    use sankey_copier_relay_server::adapters::outbound::persistence::Database;

    // 1. Setup Sandbox directly
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();

    // 2. Setup Database fixtures
    // We must register the Master and Slave in the DB so they are "Enabled" and linked.
    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to test DB");

    let master_account = "Master_AutoMap";
    let slave_account = "Slave_AutoMap";

    setup_test_scenario(&db, master_account, &[slave_account], |_| {
        default_test_slave_settings()
    })
    .await
    .expect("Failed to setup test scenario");

    // 3. Create and Start Simulators
    let mut master = sandbox
        .create_master(master_account, true)
        .expect("Failed to create master");

    let mut slave = sandbox
        .create_slave(slave_account, master_account, true)
        .expect("Failed to create slave");

    // Inject detected symbols using our new setter
    slave.set_candidates(vec!["GOLD".to_string()]);

    master.set_trade_allowed(true);
    master.start().expect("Failed to start master");

    slave.set_trade_allowed(true);
    slave.start().expect("Failed to start slave");

    // Wait for Slave to connect (receives config from server)
    slave
        .wait_for_status(STATUS_CONNECTED, 5000)
        .expect("Slave failed to connect");

    // 4. Master sends Open Trade (XAUUSD)
    println!("Master sending Open Trade: XAUUSD");
    let ticket = 1001;
    let signal = master.create_open_signal(
        ticket,
        "XAUUSD",
        e2e_tests::OrderType::Buy,
        1.0,
        2000.0,
        None,
        None,
        0,
    );
    master
        .send_trade_signal(&signal)
        .expect("Failed to send open trade");

    // 5. Verify Slave receives trade mapped to "GOLD"
    // We wait for the trade to be received
    // Use try_receive loop or sleep
    sleep(Duration::from_millis(1000)).await;

    let trades = slave.get_received_trade_signals();
    // Debug output if empty
    if trades.is_empty() {
        println!("WARNING: No trades received by slave!");
    }

    assert!(!trades.is_empty(), "Slave should have received the trade");

    let received_trade = &trades[0];
    let symbol = received_trade.symbol.clone().unwrap_or_default();
    println!("Slave received trade: Symbol='{}'", symbol);

    assert_eq!(symbol, "GOLD", "Symbol should be mapped to GOLD");
    assert_eq!(received_trade.lots.unwrap_or(0.0), 1.0);
}
