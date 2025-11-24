// relay-server/tests/e2e_master_config_test.rs
//
// E2E integration test for Master/Slave EA configuration distribution.
// This test uses EA simulators to verify the complete flow:
// - Master EA: Heartbeat -> RequestConfig -> MasterConfigMessage
// - Slave EA: Heartbeat -> RequestConfig -> ConfigMessage
//
// These tests automatically spawn a relay-server instance with dynamic ports,
// making them suitable for CI/CD environments.

mod test_server;

use sankey_copier_relay_server::models::SlaveSettings;
use sankey_copier_zmq::{
    ConfigMessage, HeartbeatMessage, MasterConfigMessage, RequestConfigMessage,
};
use test_server::TestServer;
use tokio::time::{sleep, Duration};
use zmq::{Context, Socket};

/// Master EA Simulator for integration testing
/// Simulates a Master EA connecting to the relay server via ZMQ
struct MasterEaSimulator {
    _context: Context, // Owned context (not Arc) for proper cleanup
    push_socket: Socket,
    config_socket: Socket,
    account_id: String,
}

impl MasterEaSimulator {
    /// Create a new Master EA simulator
    ///
    /// # Parameters
    /// - push_address: Address for PUSH socket (e.g., "tcp://localhost:5555")
    /// - config_address: Address for SUB socket (e.g., "tcp://localhost:5557")
    /// - account_id: Master account ID for topic subscription
    fn new(push_address: &str, config_address: &str, account_id: &str) -> anyhow::Result<Self> {
        let context = Context::new();

        // PUSH socket for sending Heartbeat and RequestConfig
        let push_socket = context.socket(zmq::PUSH)?;
        push_socket.set_linger(0)?; // Don't wait for unsent messages on close
        push_socket.connect(push_address)?;

        // SUB socket for receiving MasterConfigMessage
        let config_socket = context.socket(zmq::SUB)?;
        config_socket.set_linger(0)?;
        config_socket.connect(config_address)?;
        // Subscribe to messages for this specific account_id (topic-based filtering)
        config_socket.set_subscribe(account_id.as_bytes())?;

        Ok(Self {
            _context: context,
            push_socket,
            config_socket,
            account_id: account_id.to_string(),
        })
    }

    /// Send a Heartbeat message as Master EA
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
        self.push_socket.send(&bytes, 0)?;
        Ok(())
    }

    /// Send a RequestConfig message as Master EA
    fn send_request_config(&self) -> anyhow::Result<()> {
        let msg = RequestConfigMessage {
            message_type: "RequestConfig".to_string(),
            account_id: self.account_id.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            ea_type: "Master".to_string(),
        };

        let bytes = rmp_serde::to_vec_named(&msg)?;
        self.push_socket.send(&bytes, 0)?;
        Ok(())
    }

    /// Try to receive a MasterConfigMessage (with timeout)
    ///
    /// # Parameters
    /// - timeout_ms: Timeout in milliseconds
    ///
    /// # Returns
    /// - Ok(Some(config)): Successfully received and parsed config
    /// - Ok(None): Timeout (no message received)
    /// - Err: Error during receive or parsing
    fn try_receive_config(&self, timeout_ms: i32) -> anyhow::Result<Option<MasterConfigMessage>> {
        self.config_socket.set_rcvtimeo(timeout_ms)?;

        match self.config_socket.recv_bytes(0) {
            Ok(bytes) => {
                // Message format: topic + space + MessagePack payload
                // Find the space separator
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
                Ok(Some(config))
            }
            Err(zmq::Error::EAGAIN) => Ok(None), // Timeout
            Err(e) => Err(e.into()),
        }
    }
}

/// Slave EA Simulator for integration testing
/// Simulates a Slave EA connecting to the relay server via ZMQ
struct SlaveEaSimulator {
    _context: Context, // Owned context (not Arc) for proper cleanup
    push_socket: Socket,
    config_socket: Socket,
    _trade_socket: Socket,
    account_id: String,
}

impl SlaveEaSimulator {
    /// Create a new Slave EA simulator
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
        let context = Context::new();

        // PUSH socket for sending Heartbeat and RequestConfig
        let push_socket = context.socket(zmq::PUSH)?;
        push_socket.set_linger(0)?;
        push_socket.connect(push_address)?;

        // SUB socket for receiving ConfigMessage
        let config_socket = context.socket(zmq::SUB)?;
        config_socket.set_linger(0)?;
        config_socket.connect(config_address)?;
        // Subscribe to config messages for this account_id
        config_socket.set_subscribe(account_id.as_bytes())?;

        // SUB socket for receiving TradeSignals
        let trade_socket = context.socket(zmq::SUB)?;
        trade_socket.set_linger(0)?;
        trade_socket.connect(trade_address)?;
        // Trade socket subscription will be set based on master_account

        Ok(Self {
            _context: context,
            push_socket,
            config_socket,
            _trade_socket: trade_socket,
            account_id: account_id.to_string(),
        })
    }

    /// Subscribe to trade signals from a specific Master account
    fn subscribe_to_master(&self, master_account: &str) -> anyhow::Result<()> {
        self._trade_socket
            .set_subscribe(master_account.as_bytes())?;
        Ok(())
    }

    /// Send a Heartbeat message as Slave EA
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
        self.push_socket.send(&bytes, 0)?;
        Ok(())
    }

    /// Send a RequestConfig message as Slave EA
    fn send_request_config(&self) -> anyhow::Result<()> {
        let msg = RequestConfigMessage {
            message_type: "RequestConfig".to_string(),
            account_id: self.account_id.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            ea_type: "Slave".to_string(),
        };

        let bytes = rmp_serde::to_vec_named(&msg)?;
        self.push_socket.send(&bytes, 0)?;
        Ok(())
    }

    /// Try to receive a ConfigMessage (with timeout)
    ///
    /// # Parameters
    /// - timeout_ms: Timeout in milliseconds
    ///
    /// # Returns
    /// - Ok(Some(config)): Successfully received and parsed config
    /// - Ok(None): Timeout (no message received)
    /// - Err: Error during receive or parsing
    fn try_receive_config(&self, timeout_ms: i32) -> anyhow::Result<Option<ConfigMessage>> {
        self.config_socket.set_rcvtimeo(timeout_ms)?;

        match self.config_socket.recv_bytes(0) {
            Ok(bytes) => {
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
                let config: ConfigMessage = rmp_serde::from_slice(payload)?;
                Ok(Some(config))
            }
            Err(zmq::Error::EAGAIN) => Ok(None), // Timeout
            Err(e) => Err(e.into()),
        }
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

    // Step 3: Try to receive ConfigMessage
    let config = simulator
        .try_receive_config(2000)
        .expect("Failed to receive config");

    // Verify config was received
    assert!(config.is_some(), "Slave EA should receive ConfigMessage");

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

    // Step 4: Slave EA receives ConfigMessage
    let slave_config = slave_sim
        .try_receive_config(2000)
        .expect("Failed to receive Slave config");

    assert!(
        slave_config.is_some(),
        "Slave EA should receive ConfigMessage"
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
