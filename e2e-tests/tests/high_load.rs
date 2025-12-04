use chrono::Utc;
use e2e_tests::helpers::{
    default_test_slave_settings, setup_test_scenario, start_eas_and_wait_for_connected,
};
use e2e_tests::relay_server_process::RelayServerProcess;
use e2e_tests::{MasterEaSimulator, SlaveEaSimulator};
use sankey_copier_relay_server::db::Database;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_high_load_trade_routing() {
    // Ensure the relay-server binary is up-to-date (build workspace release)
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let status = std::process::Command::new("cargo")
        .args(["build", "--release", "-p", "sankey-copier-relay-server"])
        .current_dir(&workspace_root)
        .status()
        .expect("failed to run cargo build");
    assert!(status.success(), "Failed to build relay-server binary");

    // Start relay-server instance
    let server = RelayServerProcess::start().expect("Failed to start server");

    // Connect to server DB and setup a master with many slaves
    let db = Database::new(&server.db_url())
        .await
        .expect("DB connect failed");

    let master_account = "hload-master";
    let slave_accounts: Vec<String> = (0..10).map(|i| format!("hload-slave-{}", i)).collect();
    let slave_refs: Vec<&str> = slave_accounts.iter().map(|s| s.as_str()).collect();

    // Setup DB scenario and enable slaves
    setup_test_scenario(&db, master_account, &slave_refs, |_| {
        default_test_slave_settings()
    })
    .await
    .expect("Failed to setup DB scenario");

    // Create EA simulators
    let mut master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_address(),
        master_account,
    )
    .expect("failed to create master");

    let mut slaves: Vec<SlaveEaSimulator> = slave_accounts
        .iter()
        .map(|s| {
            SlaveEaSimulator::new(
                &server.zmq_pull_address(),
                &server.zmq_pub_address(),
                &server.zmq_pub_address(),
                s,
                master_account,
            )
            .expect("failed to create slave")
        })
        .collect();

    // Enable trading and Start all EAs and wait until CONNECTED
    master.set_trade_allowed(true);
    for s in slaves.iter_mut() {
        s.set_trade_allowed(true);
    }
    let mut slave_refs_mut: Vec<&mut SlaveEaSimulator> = slaves.iter_mut().collect();
    start_eas_and_wait_for_connected(&mut master, &mut slave_refs_mut[..], 5000)
        .await
        .expect("EAs failed to connect");

    // Flood the system with trade signals from master
    let total_signals = 200;
    let mut successes = 0usize;
    for i in 0..total_signals {
        let signal =
            master.create_open_signal(i as i64, "EURUSD", "BUY", 0.1, 1.2345, None, None, i as i64);
        if master.send_trade_signal(&signal).is_ok() {
            successes += 1;
        }
    }

    // Assert we could send most of the signals (i.e., server accepted them)
    assert!(
        successes > (total_signals / 2),
        "Too many send failures under load: {} of {}",
        successes,
        total_signals
    );

    // Give system some time to process
    sleep(Duration::from_millis(2000)).await;

    // Check database for any recorded failed ZMQ sends (should be none under normal operation)
    let pending = db
        .fetch_pending_failed_sends(100)
        .await
        .expect("db fetch failed");
    assert!(
        pending.is_empty(),
        "Found unexpected pending failed sends after high load: {}",
        pending.len()
    );
}
