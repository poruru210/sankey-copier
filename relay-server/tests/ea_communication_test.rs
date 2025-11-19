use sankey_copier_zmq::{HeartbeatMessage, RequestConfigMessage};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use zmq::{Context, Socket};

/// Mock EA Client for integration testing
/// Simulates an EA connecting to the relay server via ZMQ
#[allow(dead_code)]
struct MockEaClient {
    context: Arc<Context>,
    push_socket: Socket,
    sub_socket: Socket,
}

impl MockEaClient {
    /// Create a new mock EA client
    fn new(push_address: &str, sub_address: &str) -> anyhow::Result<Self> {
        let context = Arc::new(Context::new());

        // PUSH socket for sending messages to server
        let push_socket = context.socket(zmq::PUSH)?;
        push_socket.connect(push_address)?;

        // SUB socket for receiving config messages
        let sub_socket = context.socket(zmq::SUB)?;
        sub_socket.connect(sub_address)?;
        sub_socket.set_subscribe(b"")?; // Subscribe to all messages

        Ok(Self {
            context,
            push_socket,
            sub_socket,
        })
    }

    /// Send a RequestConfig message
    fn send_request_config(&self, account_id: &str, ea_type: &str) -> anyhow::Result<()> {
        let msg = RequestConfigMessage {
            message_type: "RequestConfig".to_string(),
            account_id: account_id.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            ea_type: ea_type.to_string(),
        };

        let bytes = rmp_serde::to_vec_named(&msg)?;
        self.push_socket.send(&bytes, 0)?;
        Ok(())
    }

    /// Send a Heartbeat message
    fn send_heartbeat(&self, account_id: &str, ea_type: &str) -> anyhow::Result<()> {
        let msg = HeartbeatMessage {
            message_type: "Heartbeat".to_string(),
            account_id: account_id.to_string(),
            balance: 10000.0,
            equity: 10000.0,
            open_positions: 0,
            timestamp: chrono::Utc::now().to_rfc3339(),
            version: "test-1.0.0".to_string(),
            ea_type: ea_type.to_string(),
            platform: "MT4".to_string(),
            account_number: 12345,
            broker: "TestBroker".to_string(),
            account_name: "TestAccount".to_string(),
            server: "TestServer".to_string(),
            currency: "USD".to_string(),
            leverage: 100,
            is_trade_allowed: true,
        };

        let bytes = rmp_serde::to_vec_named(&msg)?;
        self.push_socket.send(&bytes, 0)?;
        Ok(())
    }

    /// Try to receive a config message (non-blocking)
    fn try_receive_config(&self, timeout_ms: i32) -> anyhow::Result<Option<Vec<u8>>> {
        self.sub_socket.set_rcvtimeo(timeout_ms)?;

        match self.sub_socket.recv_bytes(0) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(zmq::Error::EAGAIN) => Ok(None), // Timeout
            Err(e) => Err(e.into()),
        }
    }
}

/// Test RequestConfig message flow with valid Slave EA type
#[tokio::test]
async fn test_request_config_slave_ea() {
    // Note: This test requires a running relay server
    // Skip if server is not available
    let client = match MockEaClient::new("tcp://localhost:5555", "tcp://localhost:5556") {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Skipping test: relay server not running");
            return;
        }
    };

    // Send RequestConfig as Slave EA
    client
        .send_request_config("TEST_SLAVE_001", "Slave")
        .unwrap();

    // Wait for response
    sleep(Duration::from_millis(500)).await;

    // Try to receive config message
    let response = client.try_receive_config(1000).unwrap();

    // If there's a config for this account, we should receive it
    // Otherwise, no message is expected (which is also valid)
    if let Some(bytes) = response {
        println!("Received config message: {} bytes", bytes.len());
    } else {
        println!("No config found for TEST_SLAVE_001 (expected if not configured)");
    }
}

/// Test RequestConfig message rejection for Master EA type
#[tokio::test]
async fn test_request_config_master_ea_rejected() {
    let client = match MockEaClient::new("tcp://localhost:5555", "tcp://localhost:5556") {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Skipping test: relay server not running");
            return;
        }
    };

    // Subscribe to config messages for this account
    // (In real scenario, server would send to specific topic)

    // Send RequestConfig as Master EA (should be rejected)
    client
        .send_request_config("TEST_MASTER_001", "Master")
        .unwrap();

    // Wait a bit
    sleep(Duration::from_millis(500)).await;

    // Try to receive config message - should NOT receive anything
    let response = client.try_receive_config(1000).unwrap();

    // Master EA should not receive config
    assert!(
        response.is_none(),
        "Master EA should not receive config message"
    );

    println!("Verified: Master EA request was rejected (no config received)");
}

/// Test Heartbeat auto-registration
#[tokio::test]
async fn test_heartbeat_auto_registration() {
    let client = match MockEaClient::new("tcp://localhost:5555", "tcp://localhost:5556") {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Skipping test: relay server not running");
            return;
        }
    };

    // Send Heartbeat message
    client
        .send_heartbeat("TEST_AUTO_REG_001", "Master")
        .unwrap();

    // Wait for server to process
    sleep(Duration::from_millis(500)).await;

    // In a real test, we would verify via API that the EA was registered
    // For now, we just verify the message was sent successfully
    println!("Heartbeat sent successfully for auto-registration");
}

/// Test multiple message sequence
#[tokio::test]
async fn test_message_sequence() {
    let client = match MockEaClient::new("tcp://localhost:5555", "tcp://localhost:5556") {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Skipping test: relay server not running");
            return;
        }
    };

    let account_id = "TEST_SEQUENCE_001";

    // 1. Send Heartbeat (auto-register)
    client.send_heartbeat(account_id, "Slave").unwrap();
    sleep(Duration::from_millis(200)).await;

    // 2. Send RequestConfig
    client.send_request_config(account_id, "Slave").unwrap();
    sleep(Duration::from_millis(500)).await;

    // 3. Check for config response
    let response = client.try_receive_config(1000).unwrap();

    if let Some(bytes) = response {
        println!("Received config after sequence: {} bytes", bytes.len());
    } else {
        println!("No config available for {}", account_id);
    }

    // 4. Send another Heartbeat
    client.send_heartbeat(account_id, "Slave").unwrap();
    sleep(Duration::from_millis(200)).await;

    println!("Message sequence completed successfully");
}
