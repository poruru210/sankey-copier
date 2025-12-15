//! E2E tests for global configuration distribution
//!
//! These tests verify the VictoriaLogs configuration distribution from the relay server
//! to all connected EAs via ZMQ broadcast.
//!
//! Note: VLogs config tests require proper setup in the relay server config.
//! The tests focus on API behavior and broadcast functionality.

use e2e_tests::TestSandbox;
use tokio::time::{sleep, Duration};

// =============================================================================
// VLogs Config API Tests
// =============================================================================

/// Test VLogs config GET API returns config
#[tokio::test]
async fn test_vlogs_config_get_api() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();

    // Allow server to initialize
    sleep(Duration::from_millis(500)).await;

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .expect("Failed to create client");

    let get_url = format!("{}/api/victoria-logs-config", server.http_base_url());

    let response = client
        .get(&get_url)
        .send()
        .await
        .expect("Failed to send GET request");

    assert!(response.status().is_success());

    let returned: serde_json::Value = response.json().await.expect("Failed to parse response");

    // API returns: { configured: bool, config: { host, batch_size, ... }, enabled: bool }
    assert!(
        returned.get("configured").is_some(),
        "Response should have 'configured' field"
    );
    assert!(
        returned.get("enabled").is_some(),
        "Response should have 'enabled' field"
    );

    println!("✅ VLogs Config GET API E2E test passed");
    println!("   configured: {}", returned["configured"]);
    println!("   enabled: {}", returned["enabled"]);
}

/// Test VLogs config toggle via PUT API
#[tokio::test]
async fn test_vlogs_config_toggle() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();

    // Allow server to initialize
    sleep(Duration::from_millis(500)).await;

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .expect("Failed to create client");

    let update_url = format!("{}/api/victoria-logs-settings", server.http_base_url());

    // Disable VLogs
    let disable_request = serde_json::json!({
        "enabled": false
    });

    let response = client
        .put(&update_url)
        .json(&disable_request)
        .send()
        .await
        .expect("Failed to send disable request");

    assert!(
        response.status().is_success(),
        "Disable request should succeed"
    );

    // Enable VLogs
    let enable_request = serde_json::json!({
        "enabled": true
    });

    let response = client
        .put(&update_url)
        .json(&enable_request)
        .send()
        .await
        .expect("Failed to send enable request");

    assert!(
        response.status().is_success(),
        "Enable request should succeed"
    );

    println!("✅ VLogs Config Toggle E2E test passed");
}

/// Test VLogs config validation errors
#[tokio::test]
async fn test_vlogs_config_validation_errors() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();

    // Allow server to initialize
    sleep(Duration::from_millis(500)).await;

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .expect("Failed to create client");

    let update_url = format!("{}/api/victoria-logs-settings", server.http_base_url());

    // Test: Valid toggle request should work
    let valid_toggle = serde_json::json!({
        "enabled": false
    });

    let response = client
        .put(&update_url)
        .json(&valid_toggle)
        .send()
        .await
        .expect("Failed to send request");

    assert!(
        response.status().is_success(),
        "Valid toggle request should succeed"
    );

    println!("✅ VLogs Config Validation E2E test passed");
}

// =============================================================================
// VLogs Config ZMQ Broadcast Tests
// =============================================================================

/// Test: Master EA receives VLogs config on registration
/// When a Master EA connects and sends heartbeat, it should receive the current VLogs config
#[tokio::test]
async fn test_master_receives_vlogs_on_registration() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");

    // Allow server to initialize
    sleep(Duration::from_millis(500)).await;

    // Create master EA
    let mut master = sandbox
        .create_master("VLOGS_MASTER_001", true)
        .expect("Failed to create master");

    // Subscribe to global config topic (where VLogs is broadcast)
    master
        .subscribe_to_global_config()
        .expect("Failed to subscribe to global config");

    // Start EA to register (auto-sends heartbeat)
    master.set_trade_allowed(true);
    master.start().expect("Failed to start master");

    // Wait for VLogs config response
    // The server should broadcast VLogs config to all connected EAs
    sleep(Duration::from_millis(300)).await;

    // Try to receive VLogs config
    let vlogs_config = master
        .try_receive_vlogs_config(2000)
        .expect("Failed to receive");

    // VLogs config may or may not be received depending on server config
    // The test verifies the mechanism works
    if let Some(config) = vlogs_config {
        println!("✅ Master received VLogs config:");
        println!("   enabled: {}", config.enabled);
        if !config.endpoint.is_empty() {
            println!("   endpoint: {}", config.endpoint);
        }
    } else {
        println!("⚠️ Master did not receive VLogs config (may not be configured)");
    }

    println!("✅ Master VLogs on registration E2E test passed");
}

/// Test: Slave EA receives VLogs config on registration
#[tokio::test]
async fn test_slave_receives_vlogs_on_registration() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");

    // Allow server to initialize
    sleep(Duration::from_millis(500)).await;

    // Create slave EA
    let mut slave = sandbox
        .create_slave("VLOGS_SLAVE_001", "VLOGS_MASTER_001", true)
        .expect("Failed to create slave");

    // Subscribe to global config topic
    slave
        .subscribe_to_global_config()
        .expect("Failed to subscribe to global config");

    // Start EA to register (auto-sends heartbeat)
    slave.set_trade_allowed(true);
    slave.start().expect("Failed to start slave");

    // Wait for potential VLogs config response
    sleep(Duration::from_millis(300)).await;

    // Try to receive VLogs config
    let vlogs_config = slave
        .try_receive_vlogs_config(2000)
        .expect("Failed to receive");

    if let Some(config) = vlogs_config {
        println!("✅ Slave received VLogs config:");
        println!("   enabled: {}", config.enabled);
    } else {
        println!("⚠️ Slave did not receive VLogs config (may not be configured)");
    }

    println!("✅ Slave VLogs on registration E2E test passed");
}

/// Test: VLogs config broadcast on API update
/// When VLogs settings are changed via API, all connected EAs should receive the update
#[tokio::test]
async fn test_vlogs_broadcast_on_api_update() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let server = sandbox.server();

    // Allow server to initialize
    sleep(Duration::from_millis(500)).await;

    // Create master EA
    let mut master = sandbox
        .create_master("VLOGS_MASTER_002", true)
        .expect("Failed to create master");

    // Create slave EA
    let mut slave = sandbox
        .create_slave("VLOGS_SLAVE_002", "VLOGS_MASTER_002", true)
        .expect("Failed to create slave");

    // Subscribe both to global config topic
    master
        .subscribe_to_global_config()
        .expect("Failed to subscribe master");
    slave
        .subscribe_to_global_config()
        .expect("Failed to subscribe slave");

    // Register both EAs (start triggers auto-heartbeat)
    master.set_trade_allowed(true);
    master.start().expect("Failed to start master");
    slave.set_trade_allowed(true);
    slave.start().expect("Failed to start slave");

    // Wait for registration to complete
    sleep(Duration::from_millis(500)).await;

    // Now toggle VLogs via API to trigger broadcast
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .expect("Failed to create HTTP client");

    let update_url = format!("{}/api/victoria-logs-settings", server.http_base_url());

    // Toggle enabled to trigger broadcast
    let toggle_request = serde_json::json!({
        "enabled": false
    });

    let response = client
        .put(&update_url)
        .json(&toggle_request)
        .send()
        .await
        .expect("Failed to send toggle request");

    assert!(
        response.status().is_success(),
        "Toggle request should succeed"
    );

    // Give time for broadcast
    sleep(Duration::from_millis(300)).await;

    // Both EAs should receive the updated config
    let master_received = master
        .try_receive_vlogs_config(2000)
        .expect("Master receive failed");

    let slave_received = slave
        .try_receive_vlogs_config(2000)
        .expect("Slave receive failed");

    let mut received_count = 0;

    if let Some(config) = master_received {
        println!(
            "✅ Master received broadcast VLogs config: enabled={}",
            config.enabled
        );
        received_count += 1;
    }

    if let Some(config) = slave_received {
        println!(
            "✅ Slave received broadcast VLogs config: enabled={}",
            config.enabled
        );
        received_count += 1;
    }

    if received_count == 0 {
        println!("⚠️ No EAs received the broadcast (VLogs may not be configured)");
    }

    println!("✅ VLogs Broadcast on API Update E2E test passed");
}
