// relay-server/tests/e2e_rest_api_test.rs
//
// E2E integration test for REST API operations from Web UI.
// This test verifies the complete flow of connection management via HTTP API:
// - Connection creation (POST /api/settings)
// - Connection toggle (POST /api/settings/:id/toggle)
// - Connection deletion (DELETE /api/settings/:id)
// - Settings retrieval (GET /api/settings, GET /api/settings/:id)
//
// These tests automatically spawn a relay-server instance with dynamic ports,
// making them suitable for CI/CD environments.

mod test_server;

use reqwest::Client;
use sankey_copier_relay_server::models::CopySettings;
use sankey_copier_zmq::{
    zmq_context_create, zmq_context_destroy, zmq_socket_connect, zmq_socket_create,
    zmq_socket_destroy, zmq_socket_receive, zmq_socket_send_binary, zmq_socket_subscribe,
    ConfigMessage, HeartbeatMessage, ZMQ_PUSH, ZMQ_SUB,
};
use serde::Serialize;
use std::ffi::c_char;
use test_server::TestServer;
use tokio::time::{sleep, Duration};

/// Request payload for creating connection settings
#[derive(Debug, Serialize)]
struct CreateSettingsRequest {
    master_account: String,
    slave_account: String,
    lot_multiplier: Option<f64>,
    reverse_trade: bool,
    status: Option<i32>,
}

/// Request payload for toggling connection status
#[derive(Debug, Serialize)]
struct ToggleRequest {
    status: i32, // 0=DISABLED, 1=ENABLED, 2=CONNECTED
}

/// Master EA Simulator for REST API testing
/// Sends Heartbeat messages to register with the relay server
struct MasterEaSimulator {
    context_handle: i32,
    socket_handle: i32,
    account_id: String,
}

impl MasterEaSimulator {
    /// Create a new Master EA simulator using mt-bridge FFI
    fn new(push_address: &str, account_id: &str) -> anyhow::Result<Self> {
        let context_handle = zmq_context_create();
        if context_handle < 0 {
            anyhow::bail!("Failed to create ZMQ context");
        }

        let socket_handle = zmq_socket_create(context_handle, ZMQ_PUSH);
        if socket_handle < 0 {
            zmq_context_destroy(context_handle);
            anyhow::bail!("Failed to create PUSH socket");
        }

        // Convert address to UTF-16 (MQL string format)
        let addr_utf16: Vec<u16> = push_address.encode_utf16().chain(Some(0)).collect();

        unsafe {
            let result = zmq_socket_connect(socket_handle, addr_utf16.as_ptr());
            if result != 1 {
                zmq_socket_destroy(socket_handle);
                zmq_context_destroy(context_handle);
                anyhow::bail!("Failed to connect to {}", push_address);
            }
        }

        Ok(Self {
            context_handle,
            socket_handle,
            account_id: account_id.to_string(),
        })
    }

    /// Send a Heartbeat message to register with relay server
    fn send_heartbeat(&self) -> anyhow::Result<()> {
        let msg = HeartbeatMessage {
            message_type: "Heartbeat".to_string(),
            account_id: self.account_id.clone(),
            balance: 50000.0,
            equity: 50000.0,
            open_positions: 3,
            timestamp: chrono::Utc::now().to_rfc3339(),
            version: "1.0.0".to_string(),
            ea_type: "Master".to_string(),
            platform: "MT5".to_string(),
            account_number: 123456,
            broker: "Test Broker".to_string(),
            account_name: "Master Account".to_string(),
            server: "Test-Server".to_string(),
            currency: "USD".to_string(),
            leverage: 500,
            is_trade_allowed: true,
            symbol_prefix: None,
            symbol_suffix: None,
            symbol_map: None,
        };

        let bytes = rmp_serde::to_vec_named(&msg)?;

        unsafe {
            let result =
                zmq_socket_send_binary(self.socket_handle, bytes.as_ptr(), bytes.len() as i32);
            if result != 1 {
                anyhow::bail!("Failed to send Heartbeat");
            }
        }

        Ok(())
    }
}

impl Drop for MasterEaSimulator {
    fn drop(&mut self) {
        zmq_socket_destroy(self.socket_handle);
        zmq_context_destroy(self.context_handle);
    }
}

/// Slave EA Simulator for REST API testing
/// Receives ConfigMessage from relay server via ZMQ
struct SlaveEaSimulator {
    context_handle: i32,
    push_socket_handle: i32,
    config_socket_handle: i32,
    account_id: String,
}

impl SlaveEaSimulator {
    /// Create a new Slave EA simulator using mt-bridge FFI
    fn new(
        push_address: &str,
        config_address: &str,
        account_id: &str,
    ) -> anyhow::Result<Self> {
        let context_handle = zmq_context_create();
        if context_handle < 0 {
            anyhow::bail!("Failed to create ZMQ context");
        }

        // Create PUSH socket for Heartbeat
        let push_socket_handle = zmq_socket_create(context_handle, ZMQ_PUSH);
        if push_socket_handle < 0 {
            zmq_context_destroy(context_handle);
            anyhow::bail!("Failed to create PUSH socket");
        }

        // Create SUB socket for ConfigMessage
        let config_socket_handle = zmq_socket_create(context_handle, ZMQ_SUB);
        if config_socket_handle < 0 {
            zmq_socket_destroy(push_socket_handle);
            zmq_context_destroy(context_handle);
            anyhow::bail!("Failed to create SUB socket");
        }

        // Convert addresses to UTF-16
        let push_addr_utf16: Vec<u16> = push_address.encode_utf16().chain(Some(0)).collect();
        let config_addr_utf16: Vec<u16> = config_address.encode_utf16().chain(Some(0)).collect();
        let topic_utf16: Vec<u16> = account_id.encode_utf16().chain(Some(0)).collect();

        unsafe {
            // Connect PUSH socket
            let push_result = zmq_socket_connect(push_socket_handle, push_addr_utf16.as_ptr());
            if push_result != 1 {
                zmq_socket_destroy(config_socket_handle);
                zmq_socket_destroy(push_socket_handle);
                zmq_context_destroy(context_handle);
                anyhow::bail!("Failed to connect PUSH socket to {}", push_address);
            }

            // Connect SUB socket
            let config_result =
                zmq_socket_connect(config_socket_handle, config_addr_utf16.as_ptr());
            if config_result != 1 {
                zmq_socket_destroy(config_socket_handle);
                zmq_socket_destroy(push_socket_handle);
                zmq_context_destroy(context_handle);
                anyhow::bail!("Failed to connect SUB socket to {}", config_address);
            }

            // Subscribe to config messages for this account_id
            let sub_result = zmq_socket_subscribe(config_socket_handle, topic_utf16.as_ptr());
            if sub_result != 1 {
                zmq_socket_destroy(config_socket_handle);
                zmq_socket_destroy(push_socket_handle);
                zmq_context_destroy(context_handle);
                anyhow::bail!("Failed to subscribe to topic: {}", account_id);
            }
        }

        Ok(Self {
            context_handle,
            push_socket_handle,
            config_socket_handle,
            account_id: account_id.to_string(),
        })
    }

    /// Send a Heartbeat message to register with relay server
    fn send_heartbeat(&self) -> anyhow::Result<()> {
        let msg = HeartbeatMessage {
            message_type: "Heartbeat".to_string(),
            account_id: self.account_id.clone(),
            balance: 10000.0,
            equity: 10000.0,
            open_positions: 1,
            timestamp: chrono::Utc::now().to_rfc3339(),
            version: "1.0.0".to_string(),
            ea_type: "Slave".to_string(),
            platform: "MT5".to_string(),
            account_number: 789012,
            broker: "Test Broker".to_string(),
            account_name: "Slave Account".to_string(),
            server: "Test-Server".to_string(),
            currency: "USD".to_string(),
            leverage: 100,
            is_trade_allowed: true,
            symbol_prefix: None,
            symbol_suffix: None,
            symbol_map: None,
        };

        let bytes = rmp_serde::to_vec_named(&msg)?;

        unsafe {
            let result = zmq_socket_send_binary(
                self.push_socket_handle,
                bytes.as_ptr(),
                bytes.len() as i32,
            );
            if result != 1 {
                anyhow::bail!("Failed to send Heartbeat");
            }
        }

        Ok(())
    }

    /// Try to receive a ConfigMessage (with timeout)
    fn try_receive_config(&self, timeout_ms: i32) -> anyhow::Result<Option<ConfigMessage>> {
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

                let payload = &bytes[space_pos + 1..];
                let config: ConfigMessage = rmp_serde::from_slice(payload)?;
                return Ok(Some(config));
            } else if received_bytes == 0 {
                if start.elapsed() >= timeout_duration {
                    return Ok(None); // Timeout
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            } else {
                return Err(anyhow::anyhow!("Failed to receive ConfigMessage"));
            }
        }
    }
}

impl Drop for SlaveEaSimulator {
    fn drop(&mut self) {
        zmq_socket_destroy(self.config_socket_handle);
        zmq_socket_destroy(self.push_socket_handle);
        zmq_context_destroy(self.context_handle);
    }
}

/// Test connection creation via REST API
/// Verifies that Slave EA receives configuration after POST /api/settings
#[tokio::test]
async fn test_create_connection_via_rest_api() {
    // Start test server with dynamic ports
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let push_address = format!("tcp://localhost:{}", server.zmq_pull_port);
    let config_address = format!("tcp://localhost:{}", server.zmq_pub_config_port);
    let http_base_url = format!("http://localhost:{}", server.http_port);

    // Create Master EA and send heartbeat
    let master = MasterEaSimulator::new(&push_address, "MASTER_API_TEST")
        .expect("Failed to create Master EA");
    master
        .send_heartbeat()
        .expect("Failed to send Master heartbeat");

    // Create Slave EA and send heartbeat
    let slave = SlaveEaSimulator::new(&push_address, &config_address, "SLAVE_API_TEST")
        .expect("Failed to create Slave EA");
    slave
        .send_heartbeat()
        .expect("Failed to send Slave heartbeat");

    sleep(Duration::from_millis(100)).await;

    // Create connection via REST API
    let client = Client::new();
    let create_req = CreateSettingsRequest {
        master_account: "MASTER_API_TEST".to_string(),
        slave_account: "SLAVE_API_TEST".to_string(),
        lot_multiplier: Some(2.0),
        reverse_trade: false,
        status: Some(1), // ENABLED
    };

    let response = client
        .post(format!("{}/api/settings", http_base_url))
        .json(&create_req)
        .send()
        .await
        .expect("Failed to send POST request");

    assert_eq!(response.status(), 201, "Expected 201 Created");

    let settings_id: i32 = response.json().await.expect("Failed to parse response");
    assert!(settings_id > 0, "Expected valid settings ID");

    println!("✅ Connection created with ID: {}", settings_id);

    // Wait for config distribution
    sleep(Duration::from_millis(200)).await;

    // Verify Slave EA received configuration
    let config = slave
        .try_receive_config(2000)
        .expect("Failed to receive config")
        .expect("Timeout: No config received");

    assert_eq!(config.account_id, "SLAVE_API_TEST");
    assert_eq!(config.master_account, "MASTER_API_TEST");
    assert_eq!(config.status, 2); // CONNECTED (Master trade allowed + Slave enabled)
    assert_eq!(config.lot_multiplier, Some(2.0));
    assert_eq!(config.reverse_trade, false);

    println!("✅ Slave EA received configuration:");
    println!("   Master: {}", config.master_account);
    println!("   Status: {} (CONNECTED)", config.status);
    println!("   Lot Multiplier: {:?}", config.lot_multiplier);

    server.shutdown().await;
}

/// Test connection toggle via REST API
/// Verifies that Slave EA receives updated configuration after POST /api/settings/:id/toggle
#[tokio::test]
async fn test_toggle_connection_via_rest_api() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let push_address = format!("tcp://localhost:{}", server.zmq_pull_port);
    let config_address = format!("tcp://localhost:{}", server.zmq_pub_config_port);
    let http_base_url = format!("http://localhost:{}", server.http_port);

    // Create Master and Slave EAs
    let master = MasterEaSimulator::new(&push_address, "MASTER_TOGGLE_TEST")
        .expect("Failed to create Master EA");
    master
        .send_heartbeat()
        .expect("Failed to send Master heartbeat");

    let slave = SlaveEaSimulator::new(&push_address, &config_address, "SLAVE_TOGGLE_TEST")
        .expect("Failed to create Slave EA");
    slave
        .send_heartbeat()
        .expect("Failed to send Slave heartbeat");

    sleep(Duration::from_millis(100)).await;

    // Create connection (initially enabled)
    let client = Client::new();
    let create_req = CreateSettingsRequest {
        master_account: "MASTER_TOGGLE_TEST".to_string(),
        slave_account: "SLAVE_TOGGLE_TEST".to_string(),
        lot_multiplier: Some(1.5),
        reverse_trade: false,
        status: Some(1), // ENABLED
    };

    let response = client
        .post(format!("{}/api/settings", http_base_url))
        .json(&create_req)
        .send()
        .await
        .expect("Failed to create connection");

    let settings_id: i32 = response.json().await.expect("Failed to parse response");

    sleep(Duration::from_millis(200)).await;

    // Receive initial config
    let config = slave
        .try_receive_config(2000)
        .expect("Failed to receive initial config")
        .expect("Timeout: No initial config");

    assert_eq!(config.status, 2); // CONNECTED
    println!("✅ Initial config status: {} (CONNECTED)", config.status);

    // Toggle to DISABLED
    let toggle_req = ToggleRequest { status: 0 };

    let response = client
        .post(format!("{}/api/settings/{}/toggle", http_base_url, settings_id))
        .json(&toggle_req)
        .send()
        .await
        .expect("Failed to toggle to DISABLED");

    assert_eq!(response.status(), 204, "Expected 204 No Content");

    sleep(Duration::from_millis(200)).await;

    // Verify Slave receives updated config with status=0
    let config = slave
        .try_receive_config(2000)
        .expect("Failed to receive config after toggle")
        .expect("Timeout: No config after toggle");

    assert_eq!(config.status, 0); // DISABLED
    println!("✅ After toggle OFF: status = {} (DISABLED)", config.status);

    // Toggle back to ENABLED
    let toggle_req = ToggleRequest { status: 1 };

    let response = client
        .post(format!("{}/api/settings/{}/toggle", http_base_url, settings_id))
        .json(&toggle_req)
        .send()
        .await
        .expect("Failed to toggle to ENABLED");

    assert_eq!(response.status(), 204);

    sleep(Duration::from_millis(200)).await;

    // Verify Slave receives config with status=2 (CONNECTED again)
    let config = slave
        .try_receive_config(2000)
        .expect("Failed to receive config after re-enable")
        .expect("Timeout: No config after re-enable");

    assert_eq!(config.status, 2); // CONNECTED
    println!("✅ After toggle ON: status = {} (CONNECTED)", config.status);

    server.shutdown().await;
}

/// Test connection deletion via REST API
/// Verifies that Slave EA receives status=0 config after DELETE /api/settings/:id
#[tokio::test]
async fn test_delete_connection_via_rest_api() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let push_address = format!("tcp://localhost:{}", server.zmq_pull_port);
    let config_address = format!("tcp://localhost:{}", server.zmq_pub_config_port);
    let http_base_url = format!("http://localhost:{}", server.http_port);

    // Create Master and Slave EAs
    let master = MasterEaSimulator::new(&push_address, "MASTER_DELETE_TEST")
        .expect("Failed to create Master EA");
    master
        .send_heartbeat()
        .expect("Failed to send Master heartbeat");

    let slave = SlaveEaSimulator::new(&push_address, &config_address, "SLAVE_DELETE_TEST")
        .expect("Failed to create Slave EA");
    slave
        .send_heartbeat()
        .expect("Failed to send Slave heartbeat");

    sleep(Duration::from_millis(100)).await;

    // Create connection
    let client = Client::new();
    let create_req = CreateSettingsRequest {
        master_account: "MASTER_DELETE_TEST".to_string(),
        slave_account: "SLAVE_DELETE_TEST".to_string(),
        lot_multiplier: Some(3.0),
        reverse_trade: true,
        status: Some(1),
    };

    let response = client
        .post(format!("{}/api/settings", http_base_url))
        .json(&create_req)
        .send()
        .await
        .expect("Failed to create connection");

    let settings_id: i32 = response.json().await.expect("Failed to parse response");

    sleep(Duration::from_millis(200)).await;

    // Receive initial config
    let _ = slave
        .try_receive_config(2000)
        .expect("Failed to receive initial config")
        .expect("Timeout: No initial config");

    // Delete connection
    let response = client
        .delete(format!("{}/api/settings/{}", http_base_url, settings_id))
        .send()
        .await
        .expect("Failed to delete connection");

    assert_eq!(response.status(), 204, "Expected 204 No Content");

    sleep(Duration::from_millis(200)).await;

    // Verify Slave receives config with status=0 (DISABLED)
    let config = slave
        .try_receive_config(2000)
        .expect("Failed to receive config after delete")
        .expect("Timeout: No config after delete");

    assert_eq!(config.status, 0); // DISABLED
    assert_eq!(config.account_id, "SLAVE_DELETE_TEST");

    println!("✅ After deletion: Slave received status = {} (DISABLED)", config.status);

    server.shutdown().await;
}

/// Test settings list and get endpoints
/// Verifies GET /api/settings and GET /api/settings/:id
#[tokio::test]
async fn test_list_and_get_settings_via_rest_api() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let push_address = format!("tcp://localhost:{}", server.zmq_pull_port);
    let http_base_url = format!("http://localhost:{}", server.http_port);

    // Create 2 Master EAs
    let master1 = MasterEaSimulator::new(&push_address, "MASTER_LIST_1")
        .expect("Failed to create Master 1");
    master1
        .send_heartbeat()
        .expect("Failed to send Master 1 heartbeat");

    let master2 = MasterEaSimulator::new(&push_address, "MASTER_LIST_2")
        .expect("Failed to create Master 2");
    master2
        .send_heartbeat()
        .expect("Failed to send Master 2 heartbeat");

    sleep(Duration::from_millis(100)).await;

    // Create 2 connections
    let client = Client::new();

    let req1 = CreateSettingsRequest {
        master_account: "MASTER_LIST_1".to_string(),
        slave_account: "SLAVE_LIST_1".to_string(),
        lot_multiplier: Some(1.0),
        reverse_trade: false,
        status: Some(0), // DISABLED
    };

    let response1 = client
        .post(format!("{}/api/settings", http_base_url))
        .json(&req1)
        .send()
        .await
        .expect("Failed to create connection 1");

    let id1: i32 = response1.json().await.expect("Failed to parse ID 1");

    let req2 = CreateSettingsRequest {
        master_account: "MASTER_LIST_2".to_string(),
        slave_account: "SLAVE_LIST_2".to_string(),
        lot_multiplier: Some(2.5),
        reverse_trade: true,
        status: Some(1), // ENABLED
    };

    let response2 = client
        .post(format!("{}/api/settings", http_base_url))
        .json(&req2)
        .send()
        .await
        .expect("Failed to create connection 2");

    let _id2: i32 = response2.json().await.expect("Failed to parse ID 2");

    sleep(Duration::from_millis(100)).await;

    // Test GET /api/settings (list all)
    let response = client
        .get(format!("{}/api/settings", http_base_url))
        .send()
        .await
        .expect("Failed to list settings");

    assert_eq!(response.status(), 200);

    let settings_list: Vec<CopySettings> = response.json().await.expect("Failed to parse settings list");

    assert!(settings_list.len() >= 2, "Expected at least 2 settings");

    println!("✅ GET /api/settings returned {} settings", settings_list.len());

    // Test GET /api/settings/:id (get specific)
    let response = client
        .get(format!("{}/api/settings/{}", http_base_url, id1))
        .send()
        .await
        .expect("Failed to get settings 1");

    assert_eq!(response.status(), 200);

    let settings1: CopySettings = response.json().await.expect("Failed to parse settings 1");

    assert_eq!(settings1.id, id1);
    assert_eq!(settings1.master_account, "MASTER_LIST_1");
    assert_eq!(settings1.slave_account, "SLAVE_LIST_1");
    assert_eq!(settings1.lot_multiplier, Some(1.0));
    assert_eq!(settings1.reverse_trade, false);
    assert_eq!(settings1.status, 0); // DISABLED

    println!("✅ GET /api/settings/{} returned correct settings", id1);

    server.shutdown().await;
}
