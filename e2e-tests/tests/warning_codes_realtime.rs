//! E2E tests for warning_codes real-time WebSocket broadcast
//!
//! These tests verify that:
//! 1. When MT EA's auto-trading is disabled (is_trade_allowed=false), warning_codes are updated immediately
//! 2. WebSocket broadcasts settings_updated with warning_codes within expected timeframe
//! 3. Both Master and Slave scenarios work correctly

use e2e_tests::helpers::default_test_slave_settings;
use e2e_tests::TestSandbox;
use e2e_tests::STATUS_DISABLED;
use futures_util::StreamExt;
use sankey_copier_relay_server::db::Database;
use sankey_copier_relay_server::models::MasterSettings;
use serde_json::Value;
use tokio::time::{sleep, timeout, Duration};
use tokio_tungstenite::tungstenite::Message;

const SETTLE_WAIT_MS: u64 = 500;
const BROADCAST_TIMEOUT_SECS: u64 = 5;

/// Create a WebSocket connector that accepts self-signed certificates
async fn create_ws_connector(
    url: &str,
) -> Result<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    Box<dyn std::error::Error>,
> {
    use native_tls::TlsConnector;
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;

    let connector = TlsConnector::builder()
        .danger_accept_invalid_certs(true)
        .build()?;
    let connector = tokio_tungstenite::Connector::NativeTls(connector);

    let request = url.into_client_request()?;
    let (ws_stream, _response) =
        tokio_tungstenite::connect_async_tls_with_config(request, None, false, Some(connector))
            .await?;

    Ok(ws_stream)
}

/// Test that Master auto-trading disabled triggers immediate warning_codes broadcast
#[tokio::test]
async fn test_master_auto_trading_disabled_warning_broadcast() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();
    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to database");

    let master_account = "WARNING_MASTER_001";
    let slave_account = "WARNING_SLAVE_001";

    // Seed trade group
    seed_trade_group(&db, master_account, slave_account)
        .await
        .expect("failed to seed trade group");

    // Wait for relay-server to fully start
    sleep(Duration::from_secs(2)).await;

    // Connect WebSocket client FIRST (before Master starts)
    let ws_url = format!("wss://127.0.0.1:{}/ws", server.http_port);
    let ws_stream = create_ws_connector(&ws_url)
        .await
        .expect("Failed to connect to WebSocket");
    let (_write, mut read) = ws_stream.split();
    sleep(Duration::from_millis(1000)).await;

    // Start Master with auto-trading DISABLED AFTER WebSocket connection
    let mut master = sandbox
        .create_master(master_account)
        .expect("Failed to create master simulator");

    master.set_trade_allowed(false); // Auto-trading OFF
    master.start().expect("master start should succeed");

    // Wait for WebSocket broadcast with master_auto_trading_disabled
    let broadcast_result = timeout(
        Duration::from_secs(BROADCAST_TIMEOUT_SECS),
        wait_for_settings_updated(&mut read, slave_account),
    )
    .await;

    assert!(
        broadcast_result.is_ok(),
        "Expected WebSocket broadcast within {} seconds",
        BROADCAST_TIMEOUT_SECS
    );

    let settings_json = broadcast_result.unwrap();
    let warning_codes: Vec<String> = settings_json["warning_codes"]
        .as_array()
        .expect("warning_codes should be an array")
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();

    // Slave is offline, Master has auto-trading disabled
    // Expected warnings: slave_offline, master_auto_trading_disabled
    assert!(
        warning_codes.contains(&"slave_offline".to_string()),
        "Expected slave_offline in warning_codes (Slave is not connected), got: {:?}",
        warning_codes
    );
    assert!(
        warning_codes.contains(&"master_auto_trading_disabled".to_string()),
        "Expected master_auto_trading_disabled in warning_codes, got: {:?}",
        warning_codes
    );
}

/// Test that Slave auto-trading disabled triggers immediate warning_codes broadcast
#[tokio::test]
async fn test_slave_auto_trading_disabled_warning_broadcast() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();
    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to database");

    let master_account = "WARNING_MASTER_002";
    let slave_account = "WARNING_SLAVE_002";

    // Seed trade group
    seed_trade_group(&db, master_account, slave_account)
        .await
        .expect("failed to seed trade group");

    // Connect WebSocket client
    let ws_url = format!("wss://127.0.0.1:{}/ws", server.http_port);
    let ws_stream = create_ws_connector(&ws_url)
        .await
        .expect("Failed to connect to WebSocket");
    let (_write, mut read) = ws_stream.split();
    sleep(Duration::from_millis(1000)).await;

    // Start Slave with auto-trading DISABLED
    let mut slave = sandbox
        .create_slave(slave_account, master_account)
        .expect("Failed to create slave simulator");

    slave.set_trade_allowed(false); // Auto-trading OFF
    slave.start().expect("slave start should succeed");

    // Wait for WebSocket broadcast
    let broadcast_result = timeout(
        Duration::from_secs(BROADCAST_TIMEOUT_SECS),
        wait_for_settings_updated(&mut read, slave_account),
    )
    .await;

    assert!(
        broadcast_result.is_ok(),
        "WebSocket broadcast timeout (no settings_updated received within {} seconds)",
        BROADCAST_TIMEOUT_SECS
    );

    let settings_json = broadcast_result.unwrap();

    // Verify warning_codes contains slave_auto_trading_disabled
    let warning_codes: Vec<String> = settings_json["warning_codes"]
        .as_array()
        .expect("warning_codes should be an array")
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();

    assert!(
        warning_codes.contains(&"slave_auto_trading_disabled".to_string()),
        "Expected slave_auto_trading_disabled in warning_codes, got {:?}",
        warning_codes
    );
}

/// Test broadcast timing: should receive within 2 seconds of heartbeat
#[tokio::test]
async fn test_warning_broadcast_timing() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();
    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to database");

    let master_account = "WARNING_MASTER_003";
    let slave_account = "WARNING_SLAVE_003";

    // Seed trade group
    seed_trade_group(&db, master_account, slave_account)
        .await
        .expect("failed to seed trade group");

    // Connect WebSocket client
    let ws_url = format!("wss://127.0.0.1:{}/ws", server.http_port);
    let ws_stream = create_ws_connector(&ws_url)
        .await
        .expect("Failed to connect to WebSocket");

    let (_write, mut read) = ws_stream.split();
    sleep(Duration::from_millis(1000)).await;

    // Start Master with auto-trading DISABLED
    let mut master = sandbox
        .create_master(master_account)
        .expect("Failed to create master simulator");

    master.set_trade_allowed(false); // Auto-trading OFF

    master.start().expect("master start should succeed");

    // Wait for WebSocket broadcast (should be fast)
    let broadcast_result = timeout(
        Duration::from_secs(2), // Strict 2-second timeout
        wait_for_settings_updated(&mut read, slave_account),
    )
    .await;

    assert!(
        broadcast_result.is_ok(),
        "WebSocket broadcast took more than 2 seconds"
    );

    let elapsed = std::time::Instant::now().elapsed();

    assert!(
        elapsed.as_millis() < 2000,
        "Broadcast should be received within 2 seconds, took {:?}",
        elapsed
    );
}

/// Test that Master warning clears when auto-trading is re-enabled
///
/// This test verifies:
/// 1. Master starts with auto-trading OFF → Slave receives master_auto_trading_disabled
/// 2. Master switches auto-trading ON (runtime) → Slave warning clears
/// 3. Demonstrates EA simulator's ability to dynamically toggle auto-trading state
#[tokio::test]
async fn test_master_warning_clears_on_auto_trading_enabled() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();
    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to database");

    let master_account = "WARNING_MASTER_004";
    let slave_account = "WARNING_SLAVE_004";

    // Seed trade group
    seed_trade_group(&db, master_account, slave_account)
        .await
        .expect("failed to seed trade group");

    // Connect WebSocket client
    let ws_url = format!("wss://127.0.0.1:{}/ws", server.http_port);
    let ws_stream = create_ws_connector(&ws_url)
        .await
        .expect("Failed to connect to WebSocket");
    let (_write, mut read) = ws_stream.split();
    sleep(Duration::from_millis(1000)).await;

    // Start Master with auto-trading DISABLED
    let mut master = sandbox
        .create_master(master_account)
        .expect("Failed to create master simulator");

    master.set_trade_allowed(false); // Auto-trading OFF
    master.start().expect("master start should succeed");

    // Wait for Master heartbeat to be processed
    sleep(Duration::from_millis(SETTLE_WAIT_MS * 4)).await;

    // Start Slave with auto-trading ENABLED
    let mut slave = sandbox
        .create_slave(slave_account, master_account)
        .expect("Failed to create slave simulator");

    slave.set_trade_allowed(true); // Slave auto-trading ON
    slave.start().expect("slave start should succeed");

    // Wait for broadcast with master_auto_trading_disabled warning
    // (Skip any initial broadcasts that don't have this warning yet)
    let _first_broadcast = timeout(
        Duration::from_secs(BROADCAST_TIMEOUT_SECS),
        wait_for_slave_warning(&mut read, slave_account, "master_auto_trading_disabled"),
    )
    .await
    .expect("Timeout waiting for master_auto_trading_disabled warning");

    // ==================================================================
    // RUNTIME AUTO-TRADING TOGGLE: Master OFF → ON
    // This demonstrates the EA simulator can change auto-trading state
    // dynamically while running (like user clicking MT5 AutoTrading button)
    // ==================================================================
    master.set_trade_allowed(true); // Auto-trading ON

    // Wait for second broadcast (warning should clear)
    let second_broadcast = timeout(
        Duration::from_secs(BROADCAST_TIMEOUT_SECS),
        wait_for_settings_updated(&mut read, slave_account),
    )
    .await
    .expect("Second broadcast timeout");

    let warning_codes_enabled: Vec<String> = second_broadcast["warning_codes"]
        .as_array()
        .expect("warning_codes should be an array")
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();

    assert!(
        !warning_codes_enabled.contains(&"master_auto_trading_disabled".to_string()),
        "Expected master_auto_trading_disabled to clear when Master auto-trading is re-enabled, got {:?}",
        warning_codes_enabled
    );
}

/// Test that Slave config updates when Master comes online later (was offline)
#[tokio::test]
async fn test_slave_update_when_master_connects_later() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();
    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to database");

    let master_account = "REPRO_MASTER_LATE";
    let slave_account = "REPRO_SLAVE_EARLY";

    seed_trade_group(&db, master_account, slave_account)
        .await
        .expect("failed to seed trade group");

    // Connect WebSocket client
    let ws_url = format!("wss://127.0.0.1:{}/ws", server.http_port);
    let ws_stream = create_ws_connector(&ws_url)
        .await
        .expect("Failed to connect to WebSocket");
    let (_write, mut read) = ws_stream.split();
    sleep(Duration::from_millis(1000)).await;

    // 1. Start Slave FIRST (Master is offline)
    println!("[TEST] Starting Slave...");
    let mut slave = sandbox
        .create_slave(slave_account, master_account)
        .expect("Failed to create slave simulator");

    slave.set_trade_allowed(true);
    slave.start().expect("slave start should succeed");

    // 2. Wait for initial status broadcast (should be Master Offline)
    println!("[TEST] Waiting for initial Slave status...");
    let initial_update = timeout(
        Duration::from_secs(BROADCAST_TIMEOUT_SECS),
        wait_for_settings_updated(&mut read, slave_account),
    )
    .await
    .expect("Timeout waiting for initial slave status");

    let warning_codes: Vec<String> = initial_update["warning_codes"]
        .as_array()
        .expect("warning_codes should be an array")
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();

    // Should have master_offline or similar
    println!("[TEST] Initial warnings: {:?}", warning_codes);

    // 3. Start Master LATER
    println!("[TEST] Starting Master...");
    let mut master = sandbox
        .create_master(master_account)
        .expect("Failed to create master simulator");

    master.set_trade_allowed(true);
    master.start().expect("master start should succeed");

    // 4. Expect a NEW update for Slave (Transition to CONNECTED / Warnings cleared)
    println!("[TEST] Waiting for Helper Update (Slave connected)...");
    let update_result = timeout(
        Duration::from_secs(BROADCAST_TIMEOUT_SECS),
        wait_for_settings_updated(&mut read, slave_account),
    )
    .await;

    // This is where we expect failure if the bug exists
    assert!(
        update_result.is_ok(),
        "Slave did NOT receive config update when Master came online!"
    );

    let final_update = update_result.unwrap();
    let final_warnings: Vec<String> = final_update["warning_codes"]
        .as_array()
        .expect("warning_codes should be an array")
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();

    println!("[TEST] Final warnings: {:?}", final_warnings);

    // Ensure master_offline IS GONE
    assert!(
        !final_warnings.contains(&"master_offline".to_string()),
        "master_offline warning persists!"
    );
    assert!(
        !final_warnings.contains(&"slave_offline".to_string()),
        "slave_offline warning persists!"
    );
}

/// Test reconnection after explicit deletion (Unregister)
/// Verifies that the server correctly handles the "Unknown" -> "Connected" transition
/// (or "Offline" -> "Connected") even after an explicit Unregister event (Soft Delete).
#[tokio::test]
async fn test_reconnection_after_deletion() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();
    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to database");

    let master_account = "DELETE_TEST_MASTER";
    let slave_account = "DELETE_TEST_SLAVE";

    seed_trade_group(&db, master_account, slave_account)
        .await
        .expect("failed to seed trade group");

    // Connect WebSocket client
    let ws_url = format!("wss://127.0.0.1:{}/ws", server.http_port);
    let ws_stream = create_ws_connector(&ws_url)
        .await
        .expect("Failed to connect to WebSocket");
    let (_write, mut read) = ws_stream.split();
    sleep(Duration::from_millis(1000)).await;

    // 1. Start Master
    let mut master = sandbox
        .create_master(master_account)
        .expect("Failed to create master");
    master.set_trade_allowed(true);
    master.start().expect("Failed to start master");

    // 2. Start Slave
    let mut slave = sandbox
        .create_slave(slave_account, master_account)
        .expect("Failed to create slave");
    slave.set_trade_allowed(true);
    slave.start().expect("Failed to start slave");

    // 3. Wait for initial connection (Slave Connected)
    // Retry loop because we might receive interim updates (status 0 or 1) before settling on 2
    let mut initial_status_ok = false;
    let start = std::time::Instant::now();

    while start.elapsed() < Duration::from_secs(BROADCAST_TIMEOUT_SECS) {
        if let Ok(config) = timeout(
            Duration::from_secs(1),
            wait_for_settings_updated(&mut read, slave_account),
        )
        .await
        {
            if config["status"] == 2 {
                initial_status_ok = true;
                break;
            }
        }
    }
    assert!(
        initial_status_ok,
        "Slave should be CONNECTED (status=2) initially"
    );

    // 4. Perform Deletion (Unregister)
    println!("[TEST] Stopping and unregistering slave...");
    slave.stop().expect("Failed to stop slave");
    // slave.send_unregister() is handled inside stop()'s thread shutdown now
    drop(slave); // Close sockets

    // Wait a bit for server to process Unregister
    sleep(Duration::from_millis(1000)).await;

    // 5. Reconnect (New Simulator instance)
    println!("[TEST] Reconnecting slave...");
    let mut slave_new = sandbox
        .create_slave(slave_account, master_account)
        .expect("Failed to create new slave");
    slave_new.set_trade_allowed(true);
    slave_new.start().expect("Failed to start new slave");

    // 6. Verify Config Broadcast (Should receive update due to Unknown/Offline -> Connected transition)
    let mut reconnect_status_ok = false;
    let mut final_warnings = Vec::new();
    let start_reconnect = std::time::Instant::now();

    while start_reconnect.elapsed() < Duration::from_secs(BROADCAST_TIMEOUT_SECS) {
        if let Ok(config) = timeout(
            Duration::from_secs(1),
            wait_for_settings_updated(&mut read, slave_account),
        )
        .await
        {
            if config["status"] == 2 {
                reconnect_status_ok = true;
                final_warnings = config["warning_codes"]
                    .as_array()
                    .expect("warning_codes array")
                    .iter()
                    .map(|v| v.as_str().unwrap().to_string())
                    .collect();
                break;
            }
        }
    }

    assert!(
        reconnect_status_ok,
        "Slave should be CONNECTED after reconnection"
    );

    assert!(
        final_warnings.is_empty(),
        "Slave should have no warnings after reconnection (Master is online)"
    );
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Seed trade group with master and slave (both enabled, DISABLED status initially)
async fn seed_trade_group(
    db: &Database,
    master_account: &str,
    slave_account: &str,
) -> anyhow::Result<()> {
    // Create trade group for master
    db.create_trade_group(master_account).await?;

    // Enable master
    let master_settings = MasterSettings {
        enabled: true,
        config_version: 1,
        ..Default::default()
    };
    db.update_master_settings(master_account, master_settings)
        .await?;

    // Add slave member with DISABLED status
    let slave_settings = default_test_slave_settings();
    db.add_member(
        master_account,
        slave_account,
        slave_settings,
        STATUS_DISABLED,
    )
    .await?;

    // Enable the member (enabled_flag) but keep status as DISABLED
    db.update_member_enabled_flag(master_account, slave_account, true)
        .await?;
    db.update_member_runtime_status(master_account, slave_account, STATUS_DISABLED)
        .await?;

    Ok(())
}

/// Wait for settings_updated WebSocket message for the specified slave account
/// Returns the parsed JSON payload
async fn wait_for_settings_updated(
    read: &mut futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
    expected_slave_account: &str,
) -> Value {
    println!(
        "[TEST] Waiting for settings_updated for slave: {}",
        expected_slave_account
    );
    while let Some(msg_result) = read.next().await {
        let msg = msg_result.expect("WebSocket read error");
        if let Message::Text(text) = msg {
            println!("[TEST] Received WebSocket message: {}", text);
            if let Some(json_str) = text.strip_prefix("settings_updated:") {
                let settings: Value =
                    serde_json::from_str(json_str).expect("Failed to parse settings_updated JSON");

                println!("[TEST] Parsed settings_updated: slave_account={}, status={}, warning_codes={:?}",
                    settings["slave_account"].as_str().unwrap_or("N/A"),
                    settings["status"],
                    settings["warning_codes"]
                );

                // Check if this is the expected slave account
                if settings["slave_account"].as_str() == Some(expected_slave_account) {
                    println!("[TEST] ✅ Found expected slave account!");
                    return settings;
                } else {
                    println!("[TEST] ⏭️  Skipping (different slave account)");
                }
            }
        }
    }
    panic!("WebSocket stream closed without receiving expected settings_updated message");
}

/// Wait for settings_updated broadcast for specific slave WITH expected warning
async fn wait_for_slave_warning(
    read: &mut futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
    slave_account: &str,
    expected_warning: &str,
) -> Value {
    use futures_util::StreamExt;

    while let Some(msg_result) = read.next().await {
        if let Ok(Message::Text(text)) = msg_result {
            if let Some(json_str) = text.strip_prefix("settings_updated:") {
                if let Ok(settings) = serde_json::from_str::<Value>(json_str) {
                    if settings["slave_account"].as_str() == Some(slave_account) {
                        let warning_codes = settings["warning_codes"]
                            .as_array()
                            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
                            .unwrap_or_default();

                        if warning_codes.contains(&expected_warning) {
                            return settings;
                        }
                    }
                }
            }
        }
    }
    panic!(
        "WebSocket stream closed without receiving {} warning for {}",
        expected_warning, slave_account
    );
}
