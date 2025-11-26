// relay-server/tests/e2e_config_test.rs
//
// E2E integration test for Master/Slave EA configuration distribution.
// This test uses EA simulators via mt-bridge FFI to verify the complete flow:
// - Master EA: Heartbeat -> RequestConfig -> MasterConfigMessage
// - Slave EA: Heartbeat -> RequestConfig -> SlaveConfigMessage
//
// IMPORTANT: Uses mt-bridge FFI functions to match actual EA behavior.
// EA (MQL) -> mt-bridge DLL -> ZMQ -> Relay Server
//
// These tests automatically spawn a relay-server instance with dynamic ports,
// making them suitable for CI/CD environments.

mod test_server;

use sankey_copier_relay_server::models::{LotCalculationMode, SlaveSettings};
use sankey_copier_zmq::{
    zmq_context_create, zmq_context_destroy, zmq_socket_connect, zmq_socket_create,
    zmq_socket_destroy, zmq_socket_receive, zmq_socket_send_binary, zmq_socket_subscribe,
    HeartbeatMessage, MasterConfigMessage, RequestConfigMessage, SlaveConfigMessage, ZMQ_PUSH,
    ZMQ_SUB,
};
use std::ffi::c_char;
use test_server::TestServer;
use tokio::time::{sleep, Duration};

/// Master EA Simulator for integration testing
/// Simulates a Master EA connecting to the relay server via mt-bridge FFI
struct MasterEaSimulator {
    context_handle: i32,
    push_socket_handle: i32,
    config_socket_handle: i32,
    account_id: String,
}

impl MasterEaSimulator {
    /// Create a new Master EA simulator using mt-bridge FFI
    ///
    /// # Parameters
    /// - push_address: Address for PUSH socket (e.g., "tcp://localhost:5555")
    /// - config_address: Address for SUB socket (e.g., "tcp://localhost:5557")
    /// - account_id: Master account ID for topic subscription
    fn new(push_address: &str, config_address: &str, account_id: &str) -> anyhow::Result<Self> {
        // Create ZMQ context via mt-bridge FFI
        let context_handle = zmq_context_create();
        if context_handle < 0 {
            anyhow::bail!("Failed to create ZMQ context via mt-bridge FFI");
        }

        // Create PUSH socket for sending Heartbeat and RequestConfig
        let push_socket_handle = zmq_socket_create(context_handle, ZMQ_PUSH);
        if push_socket_handle < 0 {
            zmq_context_destroy(context_handle);
            anyhow::bail!("Failed to create PUSH socket via mt-bridge FFI");
        }

        // Create SUB socket for receiving MasterConfigMessage
        let config_socket_handle = zmq_socket_create(context_handle, ZMQ_SUB);
        if config_socket_handle < 0 {
            zmq_socket_destroy(push_socket_handle);
            zmq_context_destroy(context_handle);
            anyhow::bail!("Failed to create SUB socket via mt-bridge FFI");
        }

        // Convert addresses to UTF-16 (MQL string format)
        let push_addr_utf16: Vec<u16> = push_address.encode_utf16().chain(Some(0)).collect();
        let config_addr_utf16: Vec<u16> = config_address.encode_utf16().chain(Some(0)).collect();
        let topic_utf16: Vec<u16> = account_id.encode_utf16().chain(Some(0)).collect();

        // Connect sockets and subscribe to topic
        unsafe {
            let push_result = zmq_socket_connect(push_socket_handle, push_addr_utf16.as_ptr());
            if push_result != 1 {
                zmq_socket_destroy(config_socket_handle);
                zmq_socket_destroy(push_socket_handle);
                zmq_context_destroy(context_handle);
                anyhow::bail!("Failed to connect PUSH socket to {}", push_address);
            }

            let config_result =
                zmq_socket_connect(config_socket_handle, config_addr_utf16.as_ptr());
            if config_result != 1 {
                zmq_socket_destroy(config_socket_handle);
                zmq_socket_destroy(push_socket_handle);
                zmq_context_destroy(context_handle);
                anyhow::bail!("Failed to connect SUB socket to {}", config_address);
            }

            // Subscribe to config messages for this account_id (topic-based filtering)
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

    /// Send a Heartbeat message as Master EA using mt-bridge FFI
    fn send_heartbeat(&self) -> anyhow::Result<()> {
        let msg = HeartbeatMessage {
            message_type: "Heartbeat".to_string(),
            account_id: self.account_id.clone(),
            balance: 50000.0,
            equity: 50000.0,
            open_positions: 5,
            timestamp: chrono::Utc::now().to_rfc3339(),
            version: "test-master-1.0.0".to_string(),
            ea_type: "Master".to_string(),
            platform: "MT5".to_string(),
            account_number: 98765,
            broker: "TestBroker".to_string(),
            account_name: "MasterTestAccount".to_string(),
            server: "TestMasterServer".to_string(),
            currency: "USD".to_string(),
            leverage: 500,
            is_trade_allowed: true,
            symbol_prefix: None,
            symbol_suffix: None,
            symbol_map: None,
        };

        let bytes = rmp_serde::to_vec_named(&msg)?;

        // Send via mt-bridge FFI
        unsafe {
            let result =
                zmq_socket_send_binary(self.push_socket_handle, bytes.as_ptr(), bytes.len() as i32);
            if result != 1 {
                anyhow::bail!("Failed to send Heartbeat via mt-bridge FFI");
            }
        }

        Ok(())
    }

    /// Send a RequestConfig message as Master EA using mt-bridge FFI
    fn send_request_config(&self) -> anyhow::Result<()> {
        let msg = RequestConfigMessage {
            message_type: "RequestConfig".to_string(),
            account_id: self.account_id.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            ea_type: "Master".to_string(),
        };

        let bytes = rmp_serde::to_vec_named(&msg)?;

        // Send via mt-bridge FFI
        unsafe {
            let result =
                zmq_socket_send_binary(self.push_socket_handle, bytes.as_ptr(), bytes.len() as i32);
            if result != 1 {
                anyhow::bail!("Failed to send RequestConfig via mt-bridge FFI");
            }
        }

        Ok(())
    }

    /// Try to receive a MasterConfigMessage (with timeout) using mt-bridge FFI
    ///
    /// # Parameters
    /// - timeout_ms: Timeout in milliseconds
    ///
    /// # Returns
    /// - Ok(Some(config)): Successfully received and parsed config
    /// - Ok(None): Timeout (no message received)
    /// - Err: Error during receive or parsing
    fn try_receive_config(&self, timeout_ms: i32) -> anyhow::Result<Option<MasterConfigMessage>> {
        const BUFFER_SIZE: usize = 65536; // 64KB buffer for large config messages
        let mut buffer = vec![0u8; BUFFER_SIZE];

        // Poll for messages with timeout
        let start = std::time::Instant::now();
        let timeout_duration = std::time::Duration::from_millis(timeout_ms as u64);

        loop {
            // Receive via mt-bridge FFI
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

                // Extract topic and payload
                let topic = &bytes[..space_pos];
                let payload = &bytes[space_pos + 1..];

                // Verify topic matches account_id
                let topic_str = String::from_utf8_lossy(topic);
                if topic_str != self.account_id {
                    return Err(anyhow::anyhow!(
                        "Topic mismatch: expected '{}', got '{}'",
                        self.account_id,
                        topic_str
                    ));
                }

                // Deserialize MessagePack payload
                let config: MasterConfigMessage = rmp_serde::from_slice(payload)?;
                return Ok(Some(config));
            } else if received_bytes == 0 {
                // EAGAIN - no message available, check timeout
                if start.elapsed() >= timeout_duration {
                    return Ok(None); // Timeout
                }
                // Sleep briefly before retrying
                std::thread::sleep(std::time::Duration::from_millis(10));
            } else {
                // Error
                return Err(anyhow::anyhow!("Failed to receive MasterConfigMessage"));
            }
        }
    }
}

impl Drop for MasterEaSimulator {
    fn drop(&mut self) {
        // Clean up ZMQ resources via mt-bridge FFI
        zmq_socket_destroy(self.config_socket_handle);
        zmq_socket_destroy(self.push_socket_handle);
        zmq_context_destroy(self.context_handle);
    }
}

/// Slave EA Simulator for integration testing
/// Simulates a Slave EA connecting to the relay server via mt-bridge FFI
struct SlaveEaSimulator {
    context_handle: i32,
    push_socket_handle: i32,
    config_socket_handle: i32,
    trade_socket_handle: i32,
    account_id: String,
}

impl SlaveEaSimulator {
    /// Create a new Slave EA simulator using mt-bridge FFI
    ///
    /// # Parameters
    /// - push_address: Address for PUSH socket (e.g., "tcp://localhost:5555")
    /// - config_address: Address for SUB socket for config (e.g., "tcp://localhost:5557")
    /// - trade_address: Address for SUB socket for trades (e.g., "tcp://localhost:5556")
    /// - account_id: Slave account ID for topic subscription
    fn new(
        push_address: &str,
        config_address: &str,
        trade_address: &str,
        account_id: &str,
    ) -> anyhow::Result<Self> {
        // Create ZMQ context via mt-bridge FFI
        let context_handle = zmq_context_create();
        if context_handle < 0 {
            anyhow::bail!("Failed to create ZMQ context via mt-bridge FFI");
        }

        // Create PUSH socket for sending Heartbeat and RequestConfig
        let push_socket_handle = zmq_socket_create(context_handle, ZMQ_PUSH);
        if push_socket_handle < 0 {
            zmq_context_destroy(context_handle);
            anyhow::bail!("Failed to create PUSH socket via mt-bridge FFI");
        }

        // Create SUB socket for receiving SlaveConfigMessage
        let config_socket_handle = zmq_socket_create(context_handle, ZMQ_SUB);
        if config_socket_handle < 0 {
            zmq_socket_destroy(push_socket_handle);
            zmq_context_destroy(context_handle);
            anyhow::bail!("Failed to create Config SUB socket via mt-bridge FFI");
        }

        // Create SUB socket for receiving TradeSignals
        let trade_socket_handle = zmq_socket_create(context_handle, ZMQ_SUB);
        if trade_socket_handle < 0 {
            zmq_socket_destroy(config_socket_handle);
            zmq_socket_destroy(push_socket_handle);
            zmq_context_destroy(context_handle);
            anyhow::bail!("Failed to create Trade SUB socket via mt-bridge FFI");
        }

        // Convert addresses to UTF-16 (MQL string format)
        let push_addr_utf16: Vec<u16> = push_address.encode_utf16().chain(Some(0)).collect();
        let config_addr_utf16: Vec<u16> = config_address.encode_utf16().chain(Some(0)).collect();
        let trade_addr_utf16: Vec<u16> = trade_address.encode_utf16().chain(Some(0)).collect();
        let account_topic_utf16: Vec<u16> = account_id.encode_utf16().chain(Some(0)).collect();

        // Connect sockets and subscribe to config topic
        unsafe {
            let push_result = zmq_socket_connect(push_socket_handle, push_addr_utf16.as_ptr());
            if push_result != 1 {
                zmq_socket_destroy(trade_socket_handle);
                zmq_socket_destroy(config_socket_handle);
                zmq_socket_destroy(push_socket_handle);
                zmq_context_destroy(context_handle);
                anyhow::bail!("Failed to connect PUSH socket to {}", push_address);
            }

            let config_result =
                zmq_socket_connect(config_socket_handle, config_addr_utf16.as_ptr());
            if config_result != 1 {
                zmq_socket_destroy(trade_socket_handle);
                zmq_socket_destroy(config_socket_handle);
                zmq_socket_destroy(push_socket_handle);
                zmq_context_destroy(context_handle);
                anyhow::bail!("Failed to connect Config SUB socket to {}", config_address);
            }

            let trade_result = zmq_socket_connect(trade_socket_handle, trade_addr_utf16.as_ptr());
            if trade_result != 1 {
                zmq_socket_destroy(trade_socket_handle);
                zmq_socket_destroy(config_socket_handle);
                zmq_socket_destroy(push_socket_handle);
                zmq_context_destroy(context_handle);
                anyhow::bail!("Failed to connect Trade SUB socket to {}", trade_address);
            }

            // Subscribe to config messages for this account_id (topic-based filtering)
            let sub_result =
                zmq_socket_subscribe(config_socket_handle, account_topic_utf16.as_ptr());
            if sub_result != 1 {
                zmq_socket_destroy(trade_socket_handle);
                zmq_socket_destroy(config_socket_handle);
                zmq_socket_destroy(push_socket_handle);
                zmq_context_destroy(context_handle);
                anyhow::bail!("Failed to subscribe to config topic: {}", account_id);
            }
        }

        Ok(Self {
            context_handle,
            push_socket_handle,
            config_socket_handle,
            trade_socket_handle,
            account_id: account_id.to_string(),
        })
    }

    /// Subscribe to trade signals from a specific Master account using mt-bridge FFI
    #[allow(dead_code)]
    fn subscribe_to_master(&self, master_account: &str) -> anyhow::Result<()> {
        let topic_utf16: Vec<u16> = master_account.encode_utf16().chain(Some(0)).collect();

        unsafe {
            let result = zmq_socket_subscribe(self.trade_socket_handle, topic_utf16.as_ptr());
            if result != 1 {
                anyhow::bail!("Failed to subscribe to master account: {}", master_account);
            }
        }

        Ok(())
    }

    /// Send a Heartbeat message as Slave EA using mt-bridge FFI
    fn send_heartbeat(&self) -> anyhow::Result<()> {
        let msg = HeartbeatMessage {
            message_type: "Heartbeat".to_string(),
            account_id: self.account_id.clone(),
            balance: 10000.0,
            equity: 10000.0,
            open_positions: 2,
            timestamp: chrono::Utc::now().to_rfc3339(),
            version: "test-slave-1.0.0".to_string(),
            ea_type: "Slave".to_string(),
            platform: "MT5".to_string(),
            account_number: 54321,
            broker: "TestBroker".to_string(),
            account_name: "SlaveTestAccount".to_string(),
            server: "TestSlaveServer".to_string(),
            currency: "USD".to_string(),
            leverage: 100,
            is_trade_allowed: true,
            symbol_prefix: None,
            symbol_suffix: None,
            symbol_map: None,
        };

        let bytes = rmp_serde::to_vec_named(&msg)?;

        // Send via mt-bridge FFI
        unsafe {
            let result =
                zmq_socket_send_binary(self.push_socket_handle, bytes.as_ptr(), bytes.len() as i32);
            if result != 1 {
                anyhow::bail!("Failed to send Heartbeat via mt-bridge FFI");
            }
        }

        Ok(())
    }

    /// Send a RequestConfig message as Slave EA using mt-bridge FFI
    fn send_request_config(&self) -> anyhow::Result<()> {
        let msg = RequestConfigMessage {
            message_type: "RequestConfig".to_string(),
            account_id: self.account_id.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            ea_type: "Slave".to_string(),
        };

        let bytes = rmp_serde::to_vec_named(&msg)?;

        // Send via mt-bridge FFI
        unsafe {
            let result =
                zmq_socket_send_binary(self.push_socket_handle, bytes.as_ptr(), bytes.len() as i32);
            if result != 1 {
                anyhow::bail!("Failed to send RequestConfig via mt-bridge FFI");
            }
        }

        Ok(())
    }

    /// Try to receive a SlaveConfigMessage (with timeout) using mt-bridge FFI
    ///
    /// # Parameters
    /// - timeout_ms: Timeout in milliseconds
    ///
    /// # Returns
    /// - Ok(Some(config)): Successfully received and parsed config
    /// - Ok(None): Timeout (no message received)
    /// - Err: Error during receive or parsing
    fn try_receive_config(&self, timeout_ms: i32) -> anyhow::Result<Option<SlaveConfigMessage>> {
        const BUFFER_SIZE: usize = 65536; // 64KB buffer for large config messages
        let mut buffer = vec![0u8; BUFFER_SIZE];

        // Poll for messages with timeout
        let start = std::time::Instant::now();
        let timeout_duration = std::time::Duration::from_millis(timeout_ms as u64);

        loop {
            // Receive via mt-bridge FFI
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

                // Extract topic and payload
                let topic = &bytes[..space_pos];
                let payload = &bytes[space_pos + 1..];

                // Verify topic matches account_id
                let topic_str = String::from_utf8_lossy(topic);
                if topic_str != self.account_id {
                    return Err(anyhow::anyhow!(
                        "Topic mismatch: expected '{}', got '{}'",
                        self.account_id,
                        topic_str
                    ));
                }

                // Deserialize MessagePack payload
                let config: SlaveConfigMessage = rmp_serde::from_slice(payload)?;
                return Ok(Some(config));
            } else if received_bytes == 0 {
                // EAGAIN - no message available, check timeout
                if start.elapsed() >= timeout_duration {
                    return Ok(None); // Timeout
                }
                // Sleep briefly before retrying
                std::thread::sleep(std::time::Duration::from_millis(10));
            } else {
                // Error
                return Err(anyhow::anyhow!("Failed to receive SlaveConfigMessage"));
            }
        }
    }
}

impl Drop for SlaveEaSimulator {
    fn drop(&mut self) {
        // Clean up ZMQ resources via mt-bridge FFI
        zmq_socket_destroy(self.trade_socket_handle);
        zmq_socket_destroy(self.config_socket_handle);
        zmq_socket_destroy(self.push_socket_handle);
        zmq_context_destroy(self.context_handle);
    }
}

/// Test Master EA config distribution flow
#[tokio::test]
async fn test_master_config_distribution() {
    // Start test server with dynamic ports
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_E2E_TEST_001";

    // Create TradeGroup with Master settings using server's database
    let trade_group = server
        .db
        .create_trade_group(master_account)
        .await
        .expect("Failed to create trade group");

    // Verify TradeGroup was created
    assert_eq!(trade_group.id, master_account);

    // Create Master EA simulator with dynamic ports
    let simulator = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .expect("Failed to create Master EA simulator");

    // Allow ZMQ connections to establish
    sleep(Duration::from_millis(200)).await;

    // Step 1: Send Heartbeat (auto-registration)
    simulator
        .send_heartbeat()
        .expect("Failed to send heartbeat");

    // Wait for server to process heartbeat
    sleep(Duration::from_millis(100)).await;

    // Step 2: Send RequestConfig
    simulator
        .send_request_config()
        .expect("Failed to send RequestConfig");

    // Wait for server to process and send config
    sleep(Duration::from_millis(200)).await;

    // Step 3: Try to receive MasterConfigMessage
    let config = simulator
        .try_receive_config(2000)
        .expect("Failed to receive config");

    // Verify config was received
    assert!(
        config.is_some(),
        "Master EA should receive MasterConfigMessage"
    );

    let config = config.unwrap();

    // Verify config fields
    assert_eq!(
        config.account_id, master_account,
        "Config account_id should match"
    );
    assert_eq!(
        config.config_version, 0,
        "Initial config_version should be 0"
    );
    assert!(config.symbol_prefix.is_none(), "Default prefix is None");
    assert!(config.symbol_suffix.is_none(), "Default suffix is None");

    println!(
        "✅ Master EA E2E test passed: Received config for {} (version: {})",
        config.account_id, config.config_version
    );

    // Explicitly shutdown server and wait for all tasks to complete
    server.shutdown().await;
}

/// Test Master EA config distribution with non-existent account
#[tokio::test]
async fn test_master_config_not_found() {
    // Start test server with dynamic ports
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "NONEXISTENT_MASTER_E2E";

    // Create Master EA simulator (no DB setup) with dynamic ports
    let simulator = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .expect("Failed to create Master EA simulator");

    // Allow ZMQ connections to establish
    sleep(Duration::from_millis(200)).await;

    // Send Heartbeat
    simulator
        .send_heartbeat()
        .expect("Failed to send heartbeat");
    sleep(Duration::from_millis(100)).await;

    // Send RequestConfig
    simulator
        .send_request_config()
        .expect("Failed to send RequestConfig");
    sleep(Duration::from_millis(200)).await;

    // Try to receive config (should timeout)
    let config = simulator
        .try_receive_config(1000)
        .expect("Failed to check for config");

    // No config should be received
    assert!(
        config.is_none(),
        "Non-existent Master should not receive config"
    );

    println!("✅ Master EA E2E test passed: No config for non-existent account");

    // Explicitly shutdown server and wait for all tasks to complete
    server.shutdown().await;
}

/// Test Slave EA config distribution flow
#[tokio::test]
async fn test_slave_config_distribution() {
    // Start test server with dynamic ports
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_E2E_002";
    let slave_account = "SLAVE_E2E_001";

    // Create TradeGroup (Master)
    server
        .db
        .create_trade_group(master_account)
        .await
        .expect("Failed to create trade group");

    // Add Slave member to TradeGroup with default settings
    server
        .db
        .add_member(master_account, slave_account, SlaveSettings::default())
        .await
        .expect("Failed to add member");

    // Create Slave EA simulator with dynamic ports
    let simulator = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave_account,
    )
    .expect("Failed to create Slave EA simulator");

    // Allow ZMQ connections to establish
    sleep(Duration::from_millis(200)).await;

    // Step 1: Send Heartbeat (auto-registration)
    simulator
        .send_heartbeat()
        .expect("Failed to send heartbeat");
    sleep(Duration::from_millis(100)).await;

    // Step 2: Send RequestConfig
    simulator
        .send_request_config()
        .expect("Failed to send RequestConfig");
    sleep(Duration::from_millis(200)).await;

    // Step 3: Try to receive SlaveConfigMessage
    let config = simulator
        .try_receive_config(2000)
        .expect("Failed to receive config");

    // Verify config was received
    assert!(
        config.is_some(),
        "Slave EA should receive SlaveConfigMessage"
    );

    let config = config.unwrap();

    // Verify config fields
    assert_eq!(
        config.account_id, slave_account,
        "Config account_id should match"
    );
    assert_eq!(
        config.master_account, master_account,
        "Config master_account should match"
    );
    assert_eq!(
        config.config_version, 0,
        "Initial config_version should be 0"
    );

    println!(
        "✅ Slave EA E2E test passed: Received config for {} from master {}",
        config.account_id, config.master_account
    );

    // Explicitly shutdown server and wait for all tasks to complete
    server.shutdown().await;
}

/// Test Master-Slave config distribution flow (both EAs)
#[tokio::test]
async fn test_master_slave_config_distribution() {
    // Start test server with dynamic ports
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_E2E_003";
    let slave_account = "SLAVE_E2E_002";

    // Create TradeGroup (Master)
    server
        .db
        .create_trade_group(master_account)
        .await
        .expect("Failed to create trade group");

    // Add Slave member to TradeGroup with default settings
    server
        .db
        .add_member(master_account, slave_account, SlaveSettings::default())
        .await
        .expect("Failed to add member");

    // Create Master EA simulator with dynamic ports
    let master_sim = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .expect("Failed to create Master EA simulator");

    // Create Slave EA simulator with dynamic ports
    let slave_sim = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave_account,
    )
    .expect("Failed to create Slave EA simulator");

    // Allow ZMQ connections to establish
    sleep(Duration::from_millis(200)).await;

    // Step 1: Both EAs send Heartbeat
    master_sim
        .send_heartbeat()
        .expect("Failed to send Master heartbeat");
    slave_sim
        .send_heartbeat()
        .expect("Failed to send Slave heartbeat");
    sleep(Duration::from_millis(100)).await;

    // Step 2: Both EAs request config
    master_sim
        .send_request_config()
        .expect("Failed to send Master RequestConfig");
    slave_sim
        .send_request_config()
        .expect("Failed to send Slave RequestConfig");
    sleep(Duration::from_millis(300)).await;

    // Step 3: Master EA receives MasterConfigMessage
    let master_config = master_sim
        .try_receive_config(2000)
        .expect("Failed to receive Master config");

    assert!(
        master_config.is_some(),
        "Master EA should receive MasterConfigMessage"
    );
    let master_config = master_config.unwrap();
    assert_eq!(master_config.account_id, master_account);

    // Step 4: Slave EA receives SlaveConfigMessage
    let slave_config = slave_sim
        .try_receive_config(2000)
        .expect("Failed to receive Slave config");

    assert!(
        slave_config.is_some(),
        "Slave EA should receive SlaveConfigMessage"
    );
    let slave_config = slave_config.unwrap();
    assert_eq!(slave_config.account_id, slave_account);
    assert_eq!(slave_config.master_account, master_account);

    println!(
        "✅ Master-Slave E2E test passed:\n   Master {} received config (version: {})\n   Slave {} received config from Master {}",
        master_config.account_id,
        master_config.config_version,
        slave_config.account_id,
        slave_config.master_account
    );

    // Explicitly shutdown server and wait for all tasks to complete
    server.shutdown().await;
}

/// Test one Master with multiple Slaves (1:N relationship)
#[tokio::test]
async fn test_multiple_slaves_same_master() {
    // Start test server with dynamic ports
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_MULTI_SLAVE";
    let slave_accounts = ["SLAVE_A", "SLAVE_B", "SLAVE_C"];

    // Create TradeGroup (Master)
    server
        .db
        .create_trade_group(master_account)
        .await
        .expect("Failed to create trade group");

    // Add 3 Slaves to the same Master with different lot multipliers
    for (i, slave_account) in slave_accounts.iter().enumerate() {
        let settings = SlaveSettings {
            lot_calculation_mode: LotCalculationMode::default(),
            lot_multiplier: Some((i + 1) as f64 * 0.5), // 0.5, 1.0, 1.5
            reverse_trade: false,
            symbol_prefix: None,
            symbol_suffix: None,
            symbol_mappings: vec![],
            filters: Default::default(),
            config_version: 0,
            source_lot_min: None,
            source_lot_max: None,
        };

        server
            .db
            .add_member(master_account, slave_account, settings)
            .await
            .expect("Failed to add member");
    }

    // Create 3 Slave EA simulators
    let mut slave_simulators = Vec::new();
    for slave_account in &slave_accounts {
        let simulator = SlaveEaSimulator::new(
            &server.zmq_pull_address(),
            &server.zmq_pub_config_address(),
            &server.zmq_pub_trade_address(),
            slave_account,
        )
        .expect("Failed to create Slave EA simulator");
        slave_simulators.push(simulator);
    }

    // Allow ZMQ connections to establish
    sleep(Duration::from_millis(200)).await;

    // All Slaves send Heartbeat
    for simulator in &slave_simulators {
        simulator
            .send_heartbeat()
            .expect("Failed to send Slave heartbeat");
    }
    sleep(Duration::from_millis(100)).await;

    // All Slaves request config
    for simulator in &slave_simulators {
        simulator
            .send_request_config()
            .expect("Failed to send Slave RequestConfig");
    }
    sleep(Duration::from_millis(300)).await;

    // Verify all Slaves receive their respective configs
    for (i, simulator) in slave_simulators.iter().enumerate() {
        let config = simulator
            .try_receive_config(2000)
            .expect("Failed to receive Slave config");

        assert!(
            config.is_some(),
            "Slave {} should receive SlaveConfigMessage",
            slave_accounts[i]
        );

        let config = config.unwrap();
        assert_eq!(config.account_id, slave_accounts[i]);
        assert_eq!(config.master_account, master_account);
        assert_eq!(
            config.lot_multiplier,
            Some((i + 1) as f64 * 0.5),
            "Slave {} should have correct lot_multiplier",
            slave_accounts[i]
        );

        println!(
            "  ✅ Slave {} received config with lot_multiplier: {:?}",
            slave_accounts[i], config.lot_multiplier
        );
    }

    println!(
        "✅ Multiple Slaves E2E test passed: {} slaves under Master {}",
        slave_accounts.len(),
        master_account
    );

    // Explicitly shutdown server and wait for all tasks to complete
    server.shutdown().await;
}

/// Test that new member is created with DISABLED status (user must explicitly enable)
#[tokio::test]
async fn test_new_member_initial_status_disabled() {
    // Start test server with dynamic ports
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_INITIAL_STATUS_TEST";
    let slave_account = "SLAVE_INITIAL_STATUS_TEST";

    // Create TradeGroup (Master)
    server
        .db
        .create_trade_group(master_account)
        .await
        .expect("Failed to create trade group");

    // Add Slave member to TradeGroup with default settings
    server
        .db
        .add_member(master_account, slave_account, SlaveSettings::default())
        .await
        .expect("Failed to add member");

    // Create Slave EA simulator
    let simulator = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave_account,
    )
    .expect("Failed to create Slave EA simulator");

    // Allow ZMQ connections to establish
    sleep(Duration::from_millis(200)).await;

    // Send Heartbeat and RequestConfig
    simulator
        .send_heartbeat()
        .expect("Failed to send heartbeat");
    sleep(Duration::from_millis(100)).await;

    simulator
        .send_request_config()
        .expect("Failed to send RequestConfig");
    sleep(Duration::from_millis(200)).await;

    // Receive config
    let config = simulator
        .try_receive_config(2000)
        .expect("Failed to receive config");
    assert!(config.is_some(), "Should receive config");
    let config = config.unwrap();

    // Verify initial status is DISABLED (0)
    assert_eq!(
        config.status, 0,
        "New member initial status should be DISABLED (0)"
    );

    println!("✅ New Member Initial Status E2E test passed: status=0 (DISABLED)");

    server.shutdown().await;
}

/// Test that toggling member status OFF sends status=0 config to Slave EA
#[tokio::test]
async fn test_toggle_member_status_off_sends_disabled_config() {
    // Start test server with dynamic ports
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_TOGGLE_TEST";
    let slave_account = "SLAVE_TOGGLE_TEST";

    // Create TradeGroup (Master)
    server
        .db
        .create_trade_group(master_account)
        .await
        .expect("Failed to create trade group");

    // Add Slave member to TradeGroup (initial status = DISABLED)
    server
        .db
        .add_member(master_account, slave_account, SlaveSettings::default())
        .await
        .expect("Failed to add member");

    // Enable the member first (so we can test toggle OFF)
    server
        .db
        .update_member_status(master_account, slave_account, 1)
        .await
        .expect("Failed to enable member");

    // Create Slave EA simulator
    let simulator = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave_account,
    )
    .expect("Failed to create Slave EA simulator");

    // Allow ZMQ connections to establish
    sleep(Duration::from_millis(200)).await;

    // Step 1: Send Heartbeat and RequestConfig to get initial config
    simulator
        .send_heartbeat()
        .expect("Failed to send heartbeat");
    sleep(Duration::from_millis(100)).await;

    simulator
        .send_request_config()
        .expect("Failed to send RequestConfig");
    sleep(Duration::from_millis(200)).await;

    // Receive initial config (should be ENABLED since we set it above)
    let initial_config = simulator
        .try_receive_config(2000)
        .expect("Failed to receive initial config");
    assert!(initial_config.is_some(), "Should receive initial config");
    let initial_config = initial_config.unwrap();
    assert_eq!(
        initial_config.status, 1,
        "Status should be ENABLED (1) after manual enable"
    );

    // Step 2: Toggle OFF via API (which triggers config distribution)
    let client = reqwest::Client::new();
    let toggle_url = format!(
        "{}/api/trade-groups/{}/members/{}/toggle",
        server.http_base_url(),
        master_account,
        slave_account
    );

    let response = client
        .post(&toggle_url)
        .json(&serde_json::json!({ "enabled": false }))
        .send()
        .await
        .expect("Failed to send toggle request");
    assert!(
        response.status().is_success(),
        "Toggle request should succeed"
    );

    sleep(Duration::from_millis(200)).await;

    // Step 3: Slave should receive config with status=0
    let disabled_config = simulator
        .try_receive_config(2000)
        .expect("Failed to receive disabled config");

    assert!(
        disabled_config.is_some(),
        "Slave should receive config after status toggle OFF"
    );
    let disabled_config = disabled_config.unwrap();
    assert_eq!(
        disabled_config.status, 0,
        "Config status should be DISABLED (0) after toggle OFF"
    );

    println!("✅ Toggle Status OFF E2E test passed: Slave received status=0 config");

    server.shutdown().await;
}

/// Test that deleting a member sends status=0 config to Slave EA
#[tokio::test]
async fn test_delete_member_sends_disabled_config() {
    // Start test server with dynamic ports
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_DELETE_TEST";
    let slave_account = "SLAVE_DELETE_TEST";

    // Create TradeGroup (Master)
    server
        .db
        .create_trade_group(master_account)
        .await
        .expect("Failed to create trade group");

    // Add Slave member to TradeGroup with default settings
    server
        .db
        .add_member(master_account, slave_account, SlaveSettings::default())
        .await
        .expect("Failed to add member");

    // Create Slave EA simulator
    let simulator = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave_account,
    )
    .expect("Failed to create Slave EA simulator");

    // Allow ZMQ connections to establish
    sleep(Duration::from_millis(200)).await;

    // Step 1: Send Heartbeat and RequestConfig to get initial config
    simulator
        .send_heartbeat()
        .expect("Failed to send heartbeat");
    sleep(Duration::from_millis(100)).await;

    simulator
        .send_request_config()
        .expect("Failed to send RequestConfig");
    sleep(Duration::from_millis(200)).await;

    // Receive initial config
    let initial_config = simulator
        .try_receive_config(2000)
        .expect("Failed to receive initial config");
    assert!(initial_config.is_some(), "Should receive initial config");

    // Step 2: Delete member via API
    let client = reqwest::Client::new();
    let delete_url = format!(
        "{}/api/trade-groups/{}/members/{}",
        server.http_base_url(),
        master_account,
        slave_account
    );

    let response = client
        .delete(&delete_url)
        .send()
        .await
        .expect("Failed to send delete request");
    assert!(
        response.status().is_success(),
        "Delete request should succeed"
    );

    sleep(Duration::from_millis(200)).await;

    // Step 3: Slave should receive config with status=0
    let disabled_config = simulator
        .try_receive_config(2000)
        .expect("Failed to receive disabled config after delete");

    assert!(
        disabled_config.is_some(),
        "Slave should receive config after member deletion"
    );
    let disabled_config = disabled_config.unwrap();
    assert_eq!(
        disabled_config.status, 0,
        "Config status should be DISABLED (0) after member deletion"
    );

    println!("✅ Delete Member E2E test passed: Slave received status=0 config");

    server.shutdown().await;
}

/// Test multiple Masters with multiple Slaves (N:M isolation)
#[tokio::test]
async fn test_multiple_masters_multiple_slaves() {
    // Start test server with dynamic ports
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master1 = "MASTER_GROUP_1";
    let master2 = "MASTER_GROUP_2";
    let slave1 = "SLAVE_G1_A";
    let slave2 = "SLAVE_G1_B";
    let slave3 = "SLAVE_G2_A";

    // Create 2 TradeGroups (Masters)
    server
        .db
        .create_trade_group(master1)
        .await
        .expect("Failed to create trade group 1");
    server
        .db
        .create_trade_group(master2)
        .await
        .expect("Failed to create trade group 2");

    // Master1 has Slave1 and Slave2
    server
        .db
        .add_member(
            master1,
            slave1,
            SlaveSettings {
                lot_calculation_mode: LotCalculationMode::default(),
                lot_multiplier: Some(1.0),
                reverse_trade: false,
                symbol_prefix: Some("M1_".to_string()),
                symbol_suffix: None,
                symbol_mappings: vec![],
                filters: Default::default(),
                config_version: 0,
                source_lot_min: None,
                source_lot_max: None,
            },
        )
        .await
        .expect("Failed to add slave1 to master1");

    server
        .db
        .add_member(
            master1,
            slave2,
            SlaveSettings {
                lot_calculation_mode: LotCalculationMode::default(),
                lot_multiplier: Some(2.0),
                reverse_trade: false,
                symbol_prefix: Some("M1_".to_string()),
                symbol_suffix: None,
                symbol_mappings: vec![],
                filters: Default::default(),
                config_version: 0,
                source_lot_min: None,
                source_lot_max: None,
            },
        )
        .await
        .expect("Failed to add slave2 to master1");

    // Master2 has Slave3
    server
        .db
        .add_member(
            master2,
            slave3,
            SlaveSettings {
                lot_calculation_mode: LotCalculationMode::default(),
                lot_multiplier: Some(0.5),
                reverse_trade: true,
                symbol_prefix: Some("M2_".to_string()),
                symbol_suffix: None,
                symbol_mappings: vec![],
                filters: Default::default(),
                config_version: 0,
                source_lot_min: None,
                source_lot_max: None,
            },
        )
        .await
        .expect("Failed to add slave3 to master2");

    // Create Master EA simulators
    let master1_sim = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master1,
    )
    .expect("Failed to create Master1 simulator");

    let master2_sim = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master2,
    )
    .expect("Failed to create Master2 simulator");

    // Create Slave EA simulators
    let slave1_sim = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave1,
    )
    .expect("Failed to create Slave1 simulator");

    let slave2_sim = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave2,
    )
    .expect("Failed to create Slave2 simulator");

    let slave3_sim = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave3,
    )
    .expect("Failed to create Slave3 simulator");

    // Allow ZMQ connections to establish
    sleep(Duration::from_millis(200)).await;

    // All EAs send Heartbeat
    master1_sim
        .send_heartbeat()
        .expect("Failed to send Master1 heartbeat");
    master2_sim
        .send_heartbeat()
        .expect("Failed to send Master2 heartbeat");
    slave1_sim
        .send_heartbeat()
        .expect("Failed to send Slave1 heartbeat");
    slave2_sim
        .send_heartbeat()
        .expect("Failed to send Slave2 heartbeat");
    slave3_sim
        .send_heartbeat()
        .expect("Failed to send Slave3 heartbeat");
    sleep(Duration::from_millis(100)).await;

    // All EAs request config
    master1_sim
        .send_request_config()
        .expect("Failed to send Master1 RequestConfig");
    master2_sim
        .send_request_config()
        .expect("Failed to send Master2 RequestConfig");
    slave1_sim
        .send_request_config()
        .expect("Failed to send Slave1 RequestConfig");
    slave2_sim
        .send_request_config()
        .expect("Failed to send Slave2 RequestConfig");
    slave3_sim
        .send_request_config()
        .expect("Failed to send Slave3 RequestConfig");
    sleep(Duration::from_millis(300)).await;

    // Verify Master1 receives config
    let master1_config = master1_sim
        .try_receive_config(2000)
        .expect("Failed to receive Master1 config");
    assert!(master1_config.is_some(), "Master1 should receive config");
    assert_eq!(master1_config.unwrap().account_id, master1);

    // Verify Master2 receives config
    let master2_config = master2_sim
        .try_receive_config(2000)
        .expect("Failed to receive Master2 config");
    assert!(master2_config.is_some(), "Master2 should receive config");
    assert_eq!(master2_config.unwrap().account_id, master2);

    // Verify Slave1 receives config from Master1
    let slave1_config = slave1_sim
        .try_receive_config(2000)
        .expect("Failed to receive Slave1 config");
    assert!(slave1_config.is_some(), "Slave1 should receive config");
    let slave1_config = slave1_config.unwrap();
    assert_eq!(slave1_config.account_id, slave1);
    assert_eq!(
        slave1_config.master_account, master1,
        "Slave1 should belong to Master1"
    );
    assert_eq!(
        slave1_config.lot_multiplier,
        Some(1.0),
        "Slave1 should have lot_multiplier 1.0"
    );
    assert_eq!(
        slave1_config.symbol_prefix,
        Some("M1_".to_string()),
        "Slave1 should have M1_ prefix"
    );

    // Verify Slave2 receives config from Master1
    let slave2_config = slave2_sim
        .try_receive_config(2000)
        .expect("Failed to receive Slave2 config");
    assert!(slave2_config.is_some(), "Slave2 should receive config");
    let slave2_config = slave2_config.unwrap();
    assert_eq!(slave2_config.account_id, slave2);
    assert_eq!(
        slave2_config.master_account, master1,
        "Slave2 should belong to Master1"
    );
    assert_eq!(
        slave2_config.lot_multiplier,
        Some(2.0),
        "Slave2 should have lot_multiplier 2.0"
    );

    // Verify Slave3 receives config from Master2
    let slave3_config = slave3_sim
        .try_receive_config(2000)
        .expect("Failed to receive Slave3 config");
    assert!(slave3_config.is_some(), "Slave3 should receive config");
    let slave3_config = slave3_config.unwrap();
    assert_eq!(slave3_config.account_id, slave3);
    assert_eq!(
        slave3_config.master_account, master2,
        "Slave3 should belong to Master2"
    );
    assert_eq!(
        slave3_config.lot_multiplier,
        Some(0.5),
        "Slave3 should have lot_multiplier 0.5"
    );
    assert!(
        slave3_config.reverse_trade,
        "Slave3 should have reverse_trade enabled"
    );
    assert_eq!(
        slave3_config.symbol_prefix,
        Some("M2_".to_string()),
        "Slave3 should have M2_ prefix"
    );

    println!("✅ Multiple Masters/Slaves E2E test passed:");
    println!(
        "   Master1 ({}) → Slave1 ({}) + Slave2 ({})",
        master1, slave1, slave2
    );
    println!("   Master2 ({}) → Slave3 ({})", master2, slave3);
    println!("   All configs correctly isolated and distributed");

    // Explicitly shutdown server and wait for all tasks to complete
    server.shutdown().await;
}
