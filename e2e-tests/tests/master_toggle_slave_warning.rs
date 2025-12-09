//! E2E tests for Master toggle → Slave warning_codes propagation
//!
//! Verifies that when a Master is toggled ON/OFF via REST API,
//! all connected Slaves receive updated warning_codes via WebSocket broadcast.
//!
//! Test scenarios:
//! 1. Master OFF → Slave gets master_web_ui_disabled warning
//! 2. Master ON → Slave warning clears

use e2e_tests::helpers::default_test_slave_settings;
use e2e_tests::TestSandbox;
use e2e_tests::STATUS_DISABLED;
use futures_util::StreamExt;
use sankey_copier_relay_server::db::Database;
use sankey_copier_relay_server::models::MasterSettings;
use serde_json::{json, Value};
use tokio::time::{sleep, timeout, Duration};

const SETTLE_WAIT_MS: u64 = 250;
const BROADCAST_TIMEOUT_SECS: u64 = 5;

/// Create an HTTP client that accepts self-signed certificates
fn create_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .expect("Failed to create HTTP client")
}

/// Create a WebSocket connector that accepts self-signed certificates
async fn create_ws_connector(
    url: &str,
) -> Result<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    Box<dyn std::error::Error>,
> {
    use native_tls::TlsConnector;
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;

    let mut retries = 5;
    let mut last_error = None;

    while retries > 0 {
        let connector = TlsConnector::builder()
            .danger_accept_invalid_certs(true)
            .build()?;
        let connector = tokio_tungstenite::Connector::NativeTls(connector);

        let request = url.into_client_request()?;

        match tokio_tungstenite::connect_async_tls_with_config(
            request,
            None,
            false,
            Some(connector),
        )
        .await
        {
            Ok((ws_stream, _)) => return Ok(ws_stream),
            Err(e) => {
                last_error = Some(e);
                retries -= 1;
                if retries > 0 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                }
            }
        }
    }

    Err(Box::new(last_error.unwrap()))
}

/// Test Master toggle OFF → Slave receives master_web_ui_disabled warning
#[tokio::test]
async fn test_master_toggle_off_adds_slave_warning() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();
    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to database");

    let master_account = "TOGGLE_MASTER_001";
    let slave_account = "TOGGLE_SLAVE_001";

    // Seed trade group with Master ENABLED
    seed_trade_group(&db, master_account, slave_account, true)
        .await
        .expect("failed to seed trade group");

    // Start Master EA (online, auto-trading ON)
    let mut master = sandbox.create_master(master_account)
        .expect("Failed to create master simulator");
    master.set_trade_allowed(true);
    master.start().expect("master start should succeed");

    // Start Slave EA (online, auto-trading ON)
    let mut slave = sandbox.create_slave(slave_account, master_account)
        .expect("Failed to create slave simulator");
    slave.set_trade_allowed(true);
    slave.start().expect("slave start should succeed");

    // Wait for initial connection
    sleep(Duration::from_millis(SETTLE_WAIT_MS * 2)).await;

    // Connect WebSocket to monitor broadcasts
    let ws_url = format!("wss://{}:{}/ws", "127.0.0.1", server.http_port);
    let ws_stream = create_ws_connector(&ws_url)
        .await
        .expect("Failed to connect to WebSocket");
    let (_write, mut read) = ws_stream.split();

    // Toggle Master OFF via REST API
    let api_url = format!(
        "{}/api/trade-groups/{}/toggle",
        server.http_base_url(),
        master_account
    );
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .expect("Failed to create HTTP client");
    let response = client
        .post(&api_url)
        .json(&json!({ "enabled": false }))
        .send()
        .await
        .expect("Failed to toggle master");

    assert_eq!(
        response.status(),
        reqwest::StatusCode::OK,
        "Master toggle API should succeed"
    );

    // Wait for WebSocket broadcast with master_web_ui_disabled warning
    let broadcast_result = timeout(
        Duration::from_secs(BROADCAST_TIMEOUT_SECS),
        wait_for_slave_warning(&mut read, slave_account, "master_web_ui_disabled"),
    )
    .await;

    assert!(
        broadcast_result.is_ok(),
        "WebSocket broadcast timeout: Slave should receive master_web_ui_disabled warning within {} seconds",
        BROADCAST_TIMEOUT_SECS
    );

    let settings_json = broadcast_result.unwrap();

    // Verify warning_codes
    let warning_codes: Vec<String> = settings_json["warning_codes"]
        .as_array()
        .expect("warning_codes should be an array")
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();

    assert!(
        warning_codes.contains(&"master_web_ui_disabled".to_string()),
        "Expected master_web_ui_disabled in warning_codes after Master toggle OFF, got {:?}",
        warning_codes
    );

    println!("✅ Master toggle OFF → Slave warning broadcast test passed");
    println!(
        "   Slave {} received warning: {:?}",
        slave_account, warning_codes
    );
}

/// Test Master toggle ON → Slave warning clears
#[tokio::test]
async fn test_master_toggle_on_clears_slave_warning() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();
    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to database");

    let master_account = "TOGGLE_MASTER_002";
    let slave_account = "TOGGLE_SLAVE_002";

    // Seed trade group with Master DISABLED
    seed_trade_group(&db, master_account, slave_account, false)
        .await
        .expect("failed to seed trade group");

    // Connect WebSocket to monitor broadcasts
    let ws_url = format!("wss://{}:{}/ws", "127.0.0.1", server.http_port);
    let ws_stream = create_ws_connector(&ws_url)
        .await
        .expect("Failed to connect to WebSocket");
    let (_write, mut read) = ws_stream.split();

    // Start Master EA (online, auto-trading ON)
    let mut master = sandbox.create_master(master_account)
        .expect("Failed to create master simulator");
    master.set_trade_allowed(true);
    master.start().expect("master start should succeed");

    // Start Slave EA (online, auto-trading ON)
    let mut slave = sandbox.create_slave(slave_account, master_account)
        .expect("Failed to create slave simulator");
    slave.set_trade_allowed(true);
    slave.start().expect("slave start should succeed");

    // Wait for first broadcast with master_web_ui_disabled warning (since Master enabled=false)
    let first_broadcast = timeout(
        Duration::from_secs(BROADCAST_TIMEOUT_SECS),
        wait_for_slave_warning(&mut read, slave_account, "master_web_ui_disabled"),
    )
    .await;

    assert!(
        first_broadcast.is_ok(),
        "Slave should receive master_web_ui_disabled warning within {} seconds (Master enabled=false)",
        BROADCAST_TIMEOUT_SECS
    );
    println!("✅ Confirmed: Slave has master_web_ui_disabled initially");

    // Toggle Master ON via REST API
    let api_url = format!(
        "{}/api/trade-groups/{}/toggle",
        server.http_base_url(),
        master_account
    );
    let client = create_http_client();
    let response = client
        .post(&api_url)
        .json(&json!({ "enabled": true }))
        .send()
        .await
        .expect("Failed to toggle master");

    assert_eq!(
        response.status(),
        reqwest::StatusCode::OK,
        "Master toggle API should succeed"
    );

    // Wait for WebSocket broadcast WITHOUT master_web_ui_disabled warning
    let broadcast_result = timeout(
        Duration::from_secs(BROADCAST_TIMEOUT_SECS),
        wait_for_slave_warning_cleared(&mut read, slave_account, "master_web_ui_disabled"),
    )
    .await;

    assert!(
        broadcast_result.is_ok(),
        "WebSocket broadcast timeout: Slave should receive cleared warning_codes within {} seconds",
        BROADCAST_TIMEOUT_SECS
    );

    let settings_json = broadcast_result.unwrap();

    // Verify warning_codes does NOT contain master_web_ui_disabled
    let warning_codes: Vec<String> = settings_json["warning_codes"]
        .as_array()
        .expect("warning_codes should be an array")
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();

    assert!(
        !warning_codes.contains(&"master_web_ui_disabled".to_string()),
        "Expected master_web_ui_disabled to be cleared from warning_codes after Master toggle ON, got {:?}",
        warning_codes
    );

    println!("✅ Master toggle ON → Slave warning cleared test passed");
    println!(
        "   Slave {} warning codes after Master ON: {:?}",
        slave_account, warning_codes
    );
}

/// Test Master toggle cycle: OFF → ON → OFF
#[tokio::test]
async fn test_master_toggle_cycle() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();
    let db = Database::new(&server.db_url())
        .await
        .expect("Failed to connect to database");

    let master_account = "TOGGLE_MASTER_003";
    let slave_account = "TOGGLE_SLAVE_003";

    // Seed trade group with Master ENABLED
    seed_trade_group(&db, master_account, slave_account, true)
        .await
        .expect("failed to seed trade group");

    // Start Master and Slave EAs
    let mut master = sandbox.create_master(master_account)
        .expect("Failed to create master simulator");
    master.set_trade_allowed(true);
    master.start().expect("master start should succeed");

    let mut slave = sandbox.create_slave(slave_account, master_account)
        .expect("Failed to create slave simulator");
    slave.set_trade_allowed(true);
    slave.start().expect("slave start should succeed");

    sleep(Duration::from_millis(SETTLE_WAIT_MS * 2)).await;

    let client = create_http_client();
    let api_url = format!(
        "{}/api/trade-groups/{}/toggle",
        server.http_base_url(),
        master_account
    );

    // Step 1: Toggle Master OFF
    {
        let ws_url = format!("wss://{}:{}/ws", "127.0.0.1", server.http_port);
        let ws_stream = create_ws_connector(&ws_url)
            .await
            .expect("Failed to connect to WebSocket");
        let (_write, mut read) = ws_stream.split();

        let response = client
            .post(&api_url)
            .json(&json!({ "enabled": false }))
            .send()
            .await
            .expect("Failed to toggle master OFF");
        assert_eq!(response.status(), reqwest::StatusCode::OK);

        let broadcast_result = timeout(
            Duration::from_secs(BROADCAST_TIMEOUT_SECS),
            wait_for_slave_warning(&mut read, slave_account, "master_web_ui_disabled"),
        )
        .await;

        assert!(
            broadcast_result.is_ok(),
            "Step 1 failed: Slave should receive master_web_ui_disabled"
        );
        println!("✅ Step 1: Master OFF → Slave warning added");
    }

    sleep(Duration::from_millis(SETTLE_WAIT_MS)).await;

    // Step 2: Toggle Master ON
    {
        let ws_url = format!("wss://{}:{}/ws", "127.0.0.1", server.http_port);
        let ws_stream = create_ws_connector(&ws_url)
            .await
            .expect("Failed to connect to WebSocket");
        let (_write, mut read) = ws_stream.split();

        let response = client
            .post(&api_url)
            .json(&json!({ "enabled": true }))
            .send()
            .await
            .expect("Failed to toggle master ON");
        assert_eq!(response.status(), reqwest::StatusCode::OK);

        let broadcast_result = timeout(
            Duration::from_secs(BROADCAST_TIMEOUT_SECS),
            wait_for_slave_warning_cleared(&mut read, slave_account, "master_web_ui_disabled"),
        )
        .await;

        assert!(
            broadcast_result.is_ok(),
            "Step 2 failed: Slave warning should clear"
        );
        println!("✅ Step 2: Master ON → Slave warning cleared");
    }

    sleep(Duration::from_millis(SETTLE_WAIT_MS)).await;

    // Step 3: Toggle Master OFF again
    {
        let ws_url = format!("wss://{}:{}/ws", "127.0.0.1", server.http_port);
        let ws_stream = create_ws_connector(&ws_url)
            .await
            .expect("Failed to connect to WebSocket");
        let (_write, mut read) = ws_stream.split();

        let response = client
            .post(&api_url)
            .json(&json!({ "enabled": false }))
            .send()
            .await
            .expect("Failed to toggle master OFF");
        assert_eq!(response.status(), reqwest::StatusCode::OK);

        let broadcast_result = timeout(
            Duration::from_secs(BROADCAST_TIMEOUT_SECS),
            wait_for_slave_warning(&mut read, slave_account, "master_web_ui_disabled"),
        )
        .await;

        assert!(
            broadcast_result.is_ok(),
            "Step 3 failed: Slave should receive master_web_ui_disabled again"
        );
        println!("✅ Step 3: Master OFF → Slave warning added again");
    }

    println!("✅ Master toggle cycle test passed (OFF → ON → OFF)");
}

// === Helper Functions ===

async fn seed_trade_group(
    db: &Database,
    master_account: &str,
    slave_account: &str,
    master_enabled: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create Master (Trade Group)
    db.create_trade_group(master_account).await?;

    // Update Master settings
    let master_settings = MasterSettings {
        enabled: master_enabled,
        symbol_prefix: None,
        symbol_suffix: None,
        config_version: 1,
    };
    db.update_master_settings(master_account, master_settings)
        .await?;

    // Add Slave member
    let slave_settings = default_test_slave_settings();
    db.add_member(
        master_account,
        slave_account,
        slave_settings,
        STATUS_DISABLED,
    )
    .await?;

    // Enable the member
    db.update_member_enabled_flag(master_account, slave_account, true)
        .await?;

    Ok(())
}

/// Wait for settings_updated broadcast for specific slave with expected warning
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

    while let Some(msg) = read.next().await {
        if let Ok(tokio_tungstenite::tungstenite::Message::Text(text)) = msg {
            if let Some(stripped) = text.strip_prefix("settings_updated:") {
                if let Ok(json) = serde_json::from_str::<Value>(stripped) {
                    if json["slave_account"].as_str() == Some(slave_account) {
                        let warnings = json["warning_codes"]
                            .as_array()
                            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
                            .unwrap_or_default();

                        if warnings.contains(&expected_warning) {
                            return json;
                        }
                    }
                }
            }
        }
    }
    panic!("WebSocket stream ended before receiving expected warning");
}

/// Wait for settings_updated broadcast for specific slave WITHOUT expected warning
async fn wait_for_slave_warning_cleared(
    read: &mut futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
    slave_account: &str,
    cleared_warning: &str,
) -> Value {
    use futures_util::StreamExt;

    while let Some(msg) = read.next().await {
        if let Ok(tokio_tungstenite::tungstenite::Message::Text(text)) = msg {
            if let Some(stripped) = text.strip_prefix("settings_updated:") {
                if let Ok(json) = serde_json::from_str::<Value>(stripped) {
                    if json["slave_account"].as_str() == Some(slave_account) {
                        let warnings = json["warning_codes"]
                            .as_array()
                            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
                            .unwrap_or_default();

                        if !warnings.contains(&cleared_warning) {
                            return json;
                        }
                    }
                }
            }
        }
    }
    panic!("WebSocket stream ended before receiving cleared warning");
}
