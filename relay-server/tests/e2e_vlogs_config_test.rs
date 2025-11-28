// relay-server/tests/e2e_vlogs_config_test.rs
//
// E2E integration test for VictoriaLogs configuration distribution.
// Tests the complete flow from Web-UI API to EA via ZMQ.
//
// Flow tested:
// 1. Web-UI PUT /api/victoria-logs-settings → DB save → ZMQ broadcast
// 2. EA registration (Heartbeat) → Auto-send VLogs config
// 3. EA receives VLogsConfigMessage → Parse via mt-bridge FFI

mod test_server;

use sankey_copier_zmq::{
    zmq_context_create, zmq_context_destroy, zmq_socket_connect, zmq_socket_create,
    zmq_socket_destroy, zmq_socket_receive, zmq_socket_send_binary, zmq_socket_subscribe,
    HeartbeatMessage, VLogsConfigMessage, ZMQ_PUSH, ZMQ_SUB,
};
use std::ffi::c_char;
use test_server::TestServer;
use tokio::time::{sleep, Duration};

/// Generic EA Simulator for VLogs config testing
/// Subscribes to "vlogs_config" topic (global broadcast)
struct VLogsConfigSubscriber {
    context_handle: i32,
    push_socket_handle: i32,
    config_socket_handle: i32,
    account_id: String,
}

impl VLogsConfigSubscriber {
    /// Create a new simulator that subscribes to "vlogs_config" topic
    fn new(push_address: &str, config_address: &str, account_id: &str) -> anyhow::Result<Self> {
        let context_handle = zmq_context_create();
        if context_handle < 0 {
            anyhow::bail!("Failed to create ZMQ context");
        }

        let push_socket_handle = zmq_socket_create(context_handle, ZMQ_PUSH);
        if push_socket_handle < 0 {
            zmq_context_destroy(context_handle);
            anyhow::bail!("Failed to create PUSH socket");
        }

        let config_socket_handle = zmq_socket_create(context_handle, ZMQ_SUB);
        if config_socket_handle < 0 {
            zmq_socket_destroy(push_socket_handle);
            zmq_context_destroy(context_handle);
            anyhow::bail!("Failed to create SUB socket");
        }

        let push_addr_utf16: Vec<u16> = push_address.encode_utf16().chain(Some(0)).collect();
        let config_addr_utf16: Vec<u16> = config_address.encode_utf16().chain(Some(0)).collect();

        // Subscribe to "vlogs_config" topic for global VLogs settings
        let vlogs_topic_utf16: Vec<u16> = "vlogs_config".encode_utf16().chain(Some(0)).collect();

        unsafe {
            let push_result = zmq_socket_connect(push_socket_handle, push_addr_utf16.as_ptr());
            if push_result != 1 {
                zmq_socket_destroy(config_socket_handle);
                zmq_socket_destroy(push_socket_handle);
                zmq_context_destroy(context_handle);
                anyhow::bail!("Failed to connect PUSH socket");
            }

            let config_result =
                zmq_socket_connect(config_socket_handle, config_addr_utf16.as_ptr());
            if config_result != 1 {
                zmq_socket_destroy(config_socket_handle);
                zmq_socket_destroy(push_socket_handle);
                zmq_context_destroy(context_handle);
                anyhow::bail!("Failed to connect SUB socket");
            }

            // Subscribe to vlogs_config topic
            let sub_result = zmq_socket_subscribe(config_socket_handle, vlogs_topic_utf16.as_ptr());
            if sub_result != 1 {
                zmq_socket_destroy(config_socket_handle);
                zmq_socket_destroy(push_socket_handle);
                zmq_context_destroy(context_handle);
                anyhow::bail!("Failed to subscribe to vlogs_config topic");
            }
        }

        Ok(Self {
            context_handle,
            push_socket_handle,
            config_socket_handle,
            account_id: account_id.to_string(),
        })
    }

    /// Send a Heartbeat message to register with the server
    fn send_heartbeat(&self, ea_type: &str) -> anyhow::Result<()> {
        let msg = HeartbeatMessage {
            message_type: "Heartbeat".to_string(),
            account_id: self.account_id.clone(),
            balance: 10000.0,
            equity: 10000.0,
            open_positions: 0,
            timestamp: chrono::Utc::now().to_rfc3339(),
            version: "test-vlogs-1.0.0".to_string(),
            ea_type: ea_type.to_string(),
            platform: "MT5".to_string(),
            account_number: 12345,
            broker: "TestBroker".to_string(),
            account_name: "VLogsTestAccount".to_string(),
            server: "TestServer".to_string(),
            currency: "USD".to_string(),
            leverage: 100,
            is_trade_allowed: true,
            symbol_prefix: None,
            symbol_suffix: None,
            symbol_map: None,
        };

        let bytes = rmp_serde::to_vec_named(&msg)?;

        unsafe {
            let result =
                zmq_socket_send_binary(self.push_socket_handle, bytes.as_ptr(), bytes.len() as i32);
            if result != 1 {
                anyhow::bail!("Failed to send Heartbeat");
            }
        }

        Ok(())
    }

    /// Try to receive a VLogsConfigMessage with timeout
    fn try_receive_vlogs_config(
        &self,
        timeout_ms: i32,
    ) -> anyhow::Result<Option<VLogsConfigMessage>> {
        const BUFFER_SIZE: usize = 65536;
        let mut buffer = vec![0u8; BUFFER_SIZE];

        let start = std::time::Instant::now();
        let timeout_duration = std::time::Duration::from_millis(timeout_ms as u64);

        loop {
            let received_bytes = unsafe {
                zmq_socket_receive(
                    self.config_socket_handle,
                    buffer.as_mut_ptr() as *mut c_char,
                    BUFFER_SIZE as i32,
                )
            };

            if received_bytes > 0 {
                let bytes = &buffer[..received_bytes as usize];

                // Message format: topic + space + MessagePack payload
                let space_pos = bytes
                    .iter()
                    .position(|&b| b == b' ')
                    .ok_or_else(|| anyhow::anyhow!("Invalid message format: no space separator"))?;

                let topic = &bytes[..space_pos];
                let payload = &bytes[space_pos + 1..];

                // Verify topic is "vlogs_config"
                let topic_str = String::from_utf8_lossy(topic);
                if topic_str != "vlogs_config" {
                    // Skip non-vlogs_config messages
                    continue;
                }

                // Deserialize MessagePack payload
                let config: VLogsConfigMessage = rmp_serde::from_slice(payload)?;
                return Ok(Some(config));
            } else if received_bytes == 0 {
                // EAGAIN - no message available
                if start.elapsed() >= timeout_duration {
                    return Ok(None);
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            } else {
                return Err(anyhow::anyhow!("Failed to receive VLogsConfigMessage"));
            }
        }
    }
}

impl Drop for VLogsConfigSubscriber {
    fn drop(&mut self) {
        zmq_socket_destroy(self.config_socket_handle);
        zmq_socket_destroy(self.push_socket_handle);
        zmq_context_destroy(self.context_handle);
    }
}

/// Test that new EA receives VLogs config on registration (Heartbeat)
#[tokio::test]
async fn test_vlogs_config_sent_on_ea_registration() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let account_id = "VLOGS_TEST_MASTER_001";

    // First, set up VLogs settings in the database
    let settings = sankey_copier_relay_server::models::VLogsGlobalSettings {
        enabled: true,
        endpoint: "http://test-vlogs:9428/insert/jsonline".to_string(),
        batch_size: 50,
        flush_interval_secs: 10,
    };
    server
        .db
        .save_vlogs_settings(&settings)
        .await
        .expect("Failed to save VLogs settings");

    // Create EA simulator that subscribes to vlogs_config
    let subscriber = VLogsConfigSubscriber::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        account_id,
    )
    .expect("Failed to create VLogs subscriber");

    // Allow ZMQ connections to establish
    sleep(Duration::from_millis(200)).await;

    // Send Heartbeat (triggers EA registration)
    subscriber
        .send_heartbeat("Master")
        .expect("Failed to send heartbeat");

    // Wait for server to process and send VLogs config
    sleep(Duration::from_millis(200)).await;

    // Try to receive VLogsConfigMessage
    let config = subscriber
        .try_receive_vlogs_config(2000)
        .expect("Failed to receive VLogs config");

    assert!(
        config.is_some(),
        "EA should receive VLogsConfigMessage on registration"
    );

    let config = config.unwrap();

    // Verify config fields match what we saved
    assert!(config.enabled, "enabled should be true");
    assert_eq!(
        config.endpoint, "http://test-vlogs:9428/insert/jsonline",
        "endpoint should match"
    );
    assert_eq!(config.batch_size, 50, "batch_size should be 50");
    assert_eq!(
        config.flush_interval_secs, 10,
        "flush_interval_secs should be 10"
    );

    println!("✅ VLogs Config on Registration E2E test passed");
    println!("   enabled: {}", config.enabled);
    println!("   endpoint: {}", config.endpoint);
    println!("   batch_size: {}", config.batch_size);
    println!("   flush_interval_secs: {}", config.flush_interval_secs);

    server.shutdown().await;
}

/// Test that VLogs config update via API is broadcasted to all EAs
#[tokio::test]
async fn test_vlogs_config_broadcast_on_api_update() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "VLOGS_BROADCAST_MASTER";
    let slave_account = "VLOGS_BROADCAST_SLAVE";

    // Create two EA simulators
    let master_sub = VLogsConfigSubscriber::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .expect("Failed to create Master subscriber");

    let slave_sub = VLogsConfigSubscriber::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        slave_account,
    )
    .expect("Failed to create Slave subscriber");

    // Allow ZMQ connections to establish
    sleep(Duration::from_millis(200)).await;

    // Register both EAs
    master_sub
        .send_heartbeat("Master")
        .expect("Failed to send Master heartbeat");
    slave_sub
        .send_heartbeat("Slave")
        .expect("Failed to send Slave heartbeat");

    sleep(Duration::from_millis(500)).await;

    // Drain any registration messages (there may be multiple)
    while master_sub
        .try_receive_vlogs_config(100)
        .ok()
        .flatten()
        .is_some()
    {}
    while slave_sub
        .try_receive_vlogs_config(100)
        .ok()
        .flatten()
        .is_some()
    {}

    // Update VLogs enabled state via API (new API only toggles enabled)
    let client = reqwest::Client::new();
    let update_url = format!("{}/api/victoria-logs-settings", server.http_base_url());

    // New API only accepts { enabled: bool }
    let toggle_request = serde_json::json!({
        "enabled": true
    });

    let response = client
        .put(&update_url)
        .json(&toggle_request)
        .send()
        .await
        .expect("Failed to send update request");

    assert!(
        response.status().is_success(),
        "VLogs settings update should succeed: {:?}",
        response.status()
    );

    sleep(Duration::from_millis(300)).await;

    // Both EAs should receive the updated config
    let master_config = master_sub
        .try_receive_vlogs_config(2000)
        .expect("Master failed to receive config")
        .expect("Master should receive VLogs config after update");

    let slave_config = slave_sub
        .try_receive_vlogs_config(2000)
        .expect("Slave failed to receive config")
        .expect("Slave should receive VLogs config after update");

    // Verify both received the same updated config (enabled=true, other values from config.toml)
    assert!(master_config.enabled);
    assert_eq!(master_config.batch_size, 100); // From test config
    assert_eq!(master_config.flush_interval_secs, 5); // From test config

    assert!(slave_config.enabled);
    assert_eq!(slave_config.batch_size, 100);
    assert_eq!(slave_config.flush_interval_secs, 5);

    println!("✅ VLogs Config API Broadcast E2E test passed");
    println!("   Both Master and Slave received updated config");
    println!("   enabled: {}", master_config.enabled);
    println!("   endpoint: {}", master_config.endpoint);
    println!("   batch_size: {}", master_config.batch_size);

    server.shutdown().await;
}

/// Test VLogs config disable flow
#[tokio::test]
async fn test_vlogs_config_disable() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let account_id = "VLOGS_DISABLE_TEST";

    // First enable VLogs
    let enabled_settings = sankey_copier_relay_server::models::VLogsGlobalSettings {
        enabled: true,
        endpoint: "http://vlogs:9428/insert/jsonline".to_string(),
        batch_size: 100,
        flush_interval_secs: 5,
    };
    server
        .db
        .save_vlogs_settings(&enabled_settings)
        .await
        .expect("Failed to save enabled settings");

    let subscriber = VLogsConfigSubscriber::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        account_id,
    )
    .expect("Failed to create subscriber");

    sleep(Duration::from_millis(200)).await;

    // Register EA
    subscriber
        .send_heartbeat("Master")
        .expect("Failed to send heartbeat");
    sleep(Duration::from_millis(200)).await;

    // Drain registration message
    let _ = subscriber.try_receive_vlogs_config(100);

    // Disable VLogs via API (new API only accepts { enabled: bool })
    let client = reqwest::Client::new();
    let update_url = format!("{}/api/victoria-logs-settings", server.http_base_url());

    let disable_request = serde_json::json!({
        "enabled": false
    });

    let response = client
        .put(&update_url)
        .json(&disable_request)
        .send()
        .await
        .expect("Failed to send disable request");

    assert!(response.status().is_success());

    sleep(Duration::from_millis(300)).await;

    // EA should receive disabled config
    let config = subscriber
        .try_receive_vlogs_config(2000)
        .expect("Failed to receive config")
        .expect("Should receive disabled config");

    assert!(!config.enabled, "enabled should be false after disable");

    println!("✅ VLogs Config Disable E2E test passed");
    println!("   enabled: {} (disabled successfully)", config.enabled);

    server.shutdown().await;
}

/// Test VLogs config GET API returns config.toml settings (read-only) + runtime enabled state
#[tokio::test]
async fn test_vlogs_config_get_api() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    // GET via new API endpoint
    let client = reqwest::Client::new();
    let get_url = format!("{}/api/victoria-logs-config", server.http_base_url());

    let response = client
        .get(&get_url)
        .send()
        .await
        .expect("Failed to send GET request");

    assert!(response.status().is_success());

    let returned: serde_json::Value = response.json().await.expect("Failed to parse response");

    // New API returns: { configured: bool, config: { host, batch_size, ... }, enabled: bool }
    assert_eq!(
        returned["configured"], true,
        "VictoriaLogs should be configured"
    );
    assert_eq!(
        returned["enabled"], true,
        "enabled should be true (from test config)"
    );

    // Config fields from config.toml (test uses localhost:9428)
    let config = &returned["config"];
    assert!(
        config["host"].as_str().unwrap().contains("localhost:9428"),
        "host should contain localhost:9428"
    );
    assert_eq!(config["batch_size"], 100, "batch_size should be 100");
    assert_eq!(
        config["flush_interval_secs"], 5,
        "flush_interval_secs should be 5"
    );

    println!("✅ VLogs Config GET API E2E test passed");
    println!("   configured: {}", returned["configured"]);
    println!("   enabled: {}", returned["enabled"]);
    println!("   host: {}", config["host"]);

    server.shutdown().await;
}

/// Test VLogs config returns test server's configured values
#[tokio::test]
async fn test_vlogs_config_default_values() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    // GET config (TestServer is always configured with VLogsController)
    let client = reqwest::Client::new();
    let get_url = format!("{}/api/victoria-logs-config", server.http_base_url());

    let response = client
        .get(&get_url)
        .send()
        .await
        .expect("Failed to send GET request");

    assert!(response.status().is_success());

    let returned: serde_json::Value = response.json().await.expect("Failed to parse response");

    // Verify test server configured values
    assert_eq!(
        returned["configured"], true,
        "VictoriaLogs should be configured"
    );
    assert_eq!(
        returned["enabled"], true,
        "enabled should be true (test config)"
    );

    let config = &returned["config"];
    assert!(
        config["host"].as_str().unwrap().contains("localhost:9428"),
        "host should be localhost:9428"
    );
    assert_eq!(config["batch_size"], 100, "batch_size should be 100");
    assert_eq!(
        config["flush_interval_secs"], 5,
        "flush_interval_secs should be 5"
    );

    println!("✅ VLogs Config Values E2E test passed");
    println!("   configured: {}", returned["configured"]);
    println!("   enabled: {}", returned["enabled"]);
    println!("   batch_size: {}", config["batch_size"]);
    println!("   flush_interval_secs: {}", config["flush_interval_secs"]);

    server.shutdown().await;
}

/// Test VLogs config validation errors (new API only accepts { enabled: bool })
#[tokio::test]
async fn test_vlogs_config_validation_errors() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let client = reqwest::Client::new();
    let update_url = format!("{}/api/victoria-logs-settings", server.http_base_url());

    // Test 1: Invalid request format (old format should be rejected)
    let old_format = serde_json::json!({
        "enabled": true,
        "endpoint": "http://vlogs:9428",
        "batch_size": 100,
        "flush_interval_secs": 5
    });

    let response = client
        .put(&update_url)
        .json(&old_format)
        .send()
        .await
        .expect("Failed to send request");

    // Old format with extra fields should still work (extra fields ignored)
    // because serde will deserialize only the 'enabled' field
    assert!(
        response.status().is_success(),
        "Request with extra fields should succeed (fields ignored)"
    );

    // Test 2: Missing enabled field should fail
    let missing_enabled = serde_json::json!({
        "something_else": true
    });

    let response = client
        .put(&update_url)
        .json(&missing_enabled)
        .send()
        .await
        .expect("Failed to send request");

    assert!(
        response.status().is_client_error(),
        "Request without 'enabled' field should be rejected"
    );

    // Test 3: Valid toggle request should work
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
    println!("   Extra fields ignored ✓");
    println!("   Missing enabled field rejected ✓");
    println!("   Valid toggle accepted ✓");

    server.shutdown().await;
}

/// Test VLogs config FFI parsing (mt-bridge integration)
#[tokio::test]
async fn test_vlogs_config_ffi_parsing() {
    // Create a VLogsConfigMessage and serialize it
    let config = VLogsConfigMessage {
        enabled: true,
        endpoint: "http://ffi-test:9428/insert/jsonline".to_string(),
        batch_size: 75,
        flush_interval_secs: 8,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    let bytes = rmp_serde::to_vec_named(&config).expect("Failed to serialize");

    // Parse using FFI function
    let handle =
        unsafe { sankey_copier_zmq::parse_vlogs_config(bytes.as_ptr(), bytes.len() as i32) };

    assert!(
        !handle.is_null(),
        "parse_vlogs_config should return valid handle"
    );

    // Get fields using FFI getters
    unsafe {
        // Test enabled
        let enabled_field: Vec<u16> = "enabled".encode_utf16().chain(Some(0)).collect();
        let enabled = sankey_copier_zmq::vlogs_config_get_bool(handle, enabled_field.as_ptr());
        assert_eq!(enabled, 1, "enabled should be 1 (true)");

        // Test batch_size
        let batch_field: Vec<u16> = "batch_size".encode_utf16().chain(Some(0)).collect();
        let batch_size = sankey_copier_zmq::vlogs_config_get_int(handle, batch_field.as_ptr());
        assert_eq!(batch_size, 75, "batch_size should be 75");

        // Test flush_interval_secs
        let interval_field: Vec<u16> = "flush_interval_secs"
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let interval = sankey_copier_zmq::vlogs_config_get_int(handle, interval_field.as_ptr());
        assert_eq!(interval, 8, "flush_interval_secs should be 8");

        // Test endpoint string
        let endpoint_field: Vec<u16> = "endpoint".encode_utf16().chain(Some(0)).collect();
        let endpoint_ptr =
            sankey_copier_zmq::vlogs_config_get_string(handle, endpoint_field.as_ptr());
        assert!(!endpoint_ptr.is_null(), "endpoint should not be null");

        // Convert UTF-16 pointer to String
        let mut len = 0;
        while *endpoint_ptr.add(len) != 0 {
            len += 1;
        }
        let endpoint_slice = std::slice::from_raw_parts(endpoint_ptr, len);
        let endpoint = String::from_utf16(endpoint_slice).expect("Invalid UTF-16");
        assert_eq!(endpoint, "http://ffi-test:9428/insert/jsonline");

        // Free handle
        sankey_copier_zmq::vlogs_config_free(handle);
    }

    println!("✅ VLogs Config FFI Parsing test passed");
    println!("   parse_vlogs_config ✓");
    println!("   vlogs_config_get_bool ✓");
    println!("   vlogs_config_get_int ✓");
    println!("   vlogs_config_get_string ✓");
    println!("   vlogs_config_free ✓");
}
