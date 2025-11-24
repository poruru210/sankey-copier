// relay-server/tests/e2e_trade_signal_test.rs
//
// E2E integration test for Trade Signal copying between Master and Slave EAs.
// This test verifies the complete flow of trade signal distribution:
// - Master EA sends trade signals via mt-bridge FFI (simulating real EA behavior)
// - Relay server processes and distributes signals
// - Slave EA receives and applies configured transformations (lot multiplier, reverse trade, symbol mapping)
//
// These tests automatically spawn a relay-server instance with dynamic ports,
// making them suitable for CI/CD environments.
//
// IMPORTANT: This test uses mt-bridge FFI functions to match the actual EA code path:
// EA (MQL) → mt-bridge DLL → ZMQ → Relay Server

mod test_server;

use chrono::Utc;
use sankey_copier_relay_server::models::{
    OrderType, SlaveSettings, SymbolMapping, TradeAction, TradeSignal,
};
use sankey_copier_zmq::*; // mt-bridge FFI functions
use test_server::TestServer;
use tokio::time::{sleep, Duration};

/// Master EA Simulator for trade signal testing
/// Simulates a Master EA sending trade signals to the relay server via mt-bridge FFI
/// This matches the actual EA behavior: EA (MQL) → mt-bridge DLL → ZMQ
struct MasterEaSimulator {
    context_handle: i32,
    socket_handle: i32,
    account_id: String,
}

impl MasterEaSimulator {
    /// Create a new Master EA simulator using mt-bridge FFI functions
    ///
    /// # Parameters
    /// - push_address: Address for PUSH socket (e.g., "tcp://localhost:5555")
    /// - account_id: Master account ID
    fn new(push_address: &str, account_id: &str) -> anyhow::Result<Self> {
        // Create ZMQ context via mt-bridge FFI (same as MQL EA would call)
        let context_handle = zmq_context_create();
        if context_handle < 0 {
            return Err(anyhow::anyhow!("Failed to create ZMQ context"));
        }

        // Create PUSH socket via mt-bridge FFI
        let socket_handle = zmq_socket_create(context_handle, ZMQ_PUSH);
        if socket_handle < 0 {
            zmq_context_destroy(context_handle);
            return Err(anyhow::anyhow!("Failed to create ZMQ PUSH socket"));
        }

        // Convert UTF-8 address to UTF-16 (as MQL would provide)
        let addr_utf16: Vec<u16> = push_address.encode_utf16().chain(Some(0)).collect();

        // Connect socket via mt-bridge FFI
        unsafe {
            let result = zmq_socket_connect(socket_handle, addr_utf16.as_ptr());
            if result != 1 {
                zmq_socket_destroy(socket_handle);
                zmq_context_destroy(context_handle);
                return Err(anyhow::anyhow!("Failed to connect to {}", push_address));
            }
        }

        Ok(Self {
            context_handle,
            socket_handle,
            account_id: account_id.to_string(),
        })
    }

    /// Send a trade signal via mt-bridge FFI (binary MessagePack)
    fn send_signal(&self, signal: &TradeSignal) -> anyhow::Result<()> {
        // Serialize to MessagePack
        let bytes = rmp_serde::to_vec_named(signal)?;

        // Send via mt-bridge FFI (same as MQL EA would call)
        unsafe {
            let result =
                zmq_socket_send_binary(self.socket_handle, bytes.as_ptr(), bytes.len() as i32);
            if result != 1 {
                return Err(anyhow::anyhow!("Failed to send signal"));
            }
        }

        Ok(())
    }

    /// Helper to create a Buy signal
    fn create_buy_signal(&self, ticket: i64, symbol: &str, lots: f64, price: f64) -> TradeSignal {
        TradeSignal {
            action: TradeAction::Open,
            ticket,
            symbol: symbol.to_string(),
            order_type: OrderType::Buy,
            lots,
            open_price: price,
            stop_loss: None,
            take_profit: None,
            magic_number: 12345,
            comment: "Test Buy".to_string(),
            timestamp: Utc::now(),
            source_account: self.account_id.clone(),
        }
    }

    /// Helper to create a Sell signal
    fn create_sell_signal(&self, ticket: i64, symbol: &str, lots: f64, price: f64) -> TradeSignal {
        TradeSignal {
            action: TradeAction::Open,
            ticket,
            symbol: symbol.to_string(),
            order_type: OrderType::Sell,
            lots,
            open_price: price,
            stop_loss: None,
            take_profit: None,
            magic_number: 12345,
            comment: "Test Sell".to_string(),
            timestamp: Utc::now(),
            source_account: self.account_id.clone(),
        }
    }
}

// Clean up ZMQ resources via mt-bridge FFI
impl Drop for MasterEaSimulator {
    fn drop(&mut self) {
        zmq_socket_destroy(self.socket_handle);
        zmq_context_destroy(self.context_handle);
    }
}

/// Slave EA Simulator for trade signal testing
/// Simulates a Slave EA receiving trade signals from the relay server via mt-bridge FFI
/// This matches the actual EA behavior: Relay Server → ZMQ → mt-bridge DLL → EA (MQL)
#[allow(dead_code)]
struct SlaveEaSimulator {
    context_handle: i32,
    socket_handle: i32,
    account_id: String,
}

impl SlaveEaSimulator {
    /// Create a new Slave EA simulator using mt-bridge FFI functions
    ///
    /// # Parameters
    /// - trade_address: Address for SUB socket (e.g., "tcp://localhost:5556")
    /// - account_id: Slave account ID
    /// - master_account: Master account ID to subscribe to
    fn new(trade_address: &str, account_id: &str, master_account: &str) -> anyhow::Result<Self> {
        // Create ZMQ context via mt-bridge FFI
        let context_handle = zmq_context_create();
        if context_handle < 0 {
            return Err(anyhow::anyhow!("Failed to create ZMQ context"));
        }

        // Create SUB socket via mt-bridge FFI
        let socket_handle = zmq_socket_create(context_handle, ZMQ_SUB);
        if socket_handle < 0 {
            zmq_context_destroy(context_handle);
            return Err(anyhow::anyhow!("Failed to create ZMQ SUB socket"));
        }

        // Convert addresses to UTF-16 (as MQL would provide)
        let addr_utf16: Vec<u16> = trade_address.encode_utf16().chain(Some(0)).collect();
        let topic_utf16: Vec<u16> = master_account.encode_utf16().chain(Some(0)).collect();

        unsafe {
            // Connect socket via mt-bridge FFI
            let result = zmq_socket_connect(socket_handle, addr_utf16.as_ptr());
            if result != 1 {
                zmq_socket_destroy(socket_handle);
                zmq_context_destroy(context_handle);
                return Err(anyhow::anyhow!("Failed to connect to {}", trade_address));
            }

            // Subscribe to signals from the specific master account via mt-bridge FFI
            let result = zmq_socket_subscribe(socket_handle, topic_utf16.as_ptr());
            if result != 1 {
                zmq_socket_destroy(socket_handle);
                zmq_context_destroy(context_handle);
                return Err(anyhow::anyhow!(
                    "Failed to subscribe to topic {}",
                    master_account
                ));
            }
        }

        Ok(Self {
            context_handle,
            socket_handle,
            account_id: account_id.to_string(),
        })
    }

    /// Try to receive a trade signal via mt-bridge FFI (with timeout)
    ///
    /// # Parameters
    /// - timeout_ms: Timeout in milliseconds
    ///
    /// # Returns
    /// - Ok(Some(signal)): Successfully received and parsed signal
    /// - Ok(None): Timeout (no signal received)
    /// - Err: Error during receive or parsing
    fn try_receive_signal(&self, timeout_ms: i32) -> anyhow::Result<Option<TradeSignal>> {
        // Buffer for receiving messages
        const BUFFER_SIZE: usize = 65536;
        let mut buffer: Vec<u8> = vec![0; BUFFER_SIZE];

        // Poll for messages with timeout
        let start = std::time::Instant::now();
        let timeout_duration = std::time::Duration::from_millis(timeout_ms as u64);

        loop {
            unsafe {
                let received_bytes = zmq_socket_receive(
                    self.socket_handle,
                    buffer.as_mut_ptr() as *mut i8,
                    BUFFER_SIZE as i32,
                );

                if received_bytes > 0 {
                    // Message format: topic + space + MessagePack payload
                    let bytes = &buffer[..received_bytes as usize];

                    let space_pos = bytes.iter().position(|&b| b == b' ').ok_or_else(|| {
                        anyhow::anyhow!("Invalid message format: no space separator")
                    })?;

                    // Extract payload (skip topic)
                    let payload = &bytes[space_pos + 1..];

                    // Deserialize MessagePack payload
                    let signal: TradeSignal = rmp_serde::from_slice(payload)?;
                    return Ok(Some(signal));
                } else if received_bytes == 0 {
                    // EAGAIN - no message available, check timeout
                    if start.elapsed() >= timeout_duration {
                        return Ok(None); // Timeout
                    }
                    // Sleep briefly before retrying
                    std::thread::sleep(std::time::Duration::from_millis(10));
                } else {
                    // Error
                    return Err(anyhow::anyhow!("Failed to receive message"));
                }
            }
        }
    }
}

// Clean up ZMQ resources via mt-bridge FFI
impl Drop for SlaveEaSimulator {
    fn drop(&mut self) {
        zmq_socket_destroy(self.socket_handle);
        zmq_context_destroy(self.context_handle);
    }
}

/// Test basic trade signal distribution
#[tokio::test]
async fn test_basic_signal_distribution() {
    // Start test server with dynamic ports
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_TRADE_001";
    let slave_account = "SLAVE_TRADE_001";

    // Create TradeGroup (Master)
    server
        .db
        .create_trade_group(master_account)
        .await
        .expect("Failed to create trade group");

    // Add Slave member with default settings
    server
        .db
        .add_member(master_account, slave_account, SlaveSettings::default())
        .await
        .expect("Failed to add member");

    // Reload settings cache to reflect the new member
    server
        .reload_settings_cache()
        .await
        .expect("Failed to reload settings cache");

    // Set all members to CONNECTED status (required for trade copying)
    server.set_all_members_connected().await;

    // Create Master EA simulator
    let master_sim = MasterEaSimulator::new(&server.zmq_pull_address(), master_account)
        .expect("Failed to create Master EA simulator");

    // Create Slave EA simulator
    let slave_sim = SlaveEaSimulator::new(
        &server.zmq_pub_trade_address(),
        slave_account,
        master_account,
    )
    .expect("Failed to create Slave EA simulator");

    // Allow ZMQ connections to establish
    sleep(Duration::from_millis(200)).await;

    // Master sends a Buy signal
    let signal = master_sim.create_buy_signal(1001, "EURUSD", 1.0, 1.1850);
    master_sim
        .send_signal(&signal)
        .expect("Failed to send signal");

    // Wait for server to process and distribute
    sleep(Duration::from_millis(200)).await;

    // Slave should receive the signal
    let received = slave_sim
        .try_receive_signal(2000)
        .expect("Failed to receive signal");

    assert!(received.is_some(), "Slave should receive trade signal");

    let received_signal = received.unwrap();
    assert_eq!(received_signal.symbol, "EURUSD");
    assert_eq!(received_signal.lots, 1.0);
    assert!(matches!(received_signal.order_type, OrderType::Buy));

    println!("✅ Basic signal distribution test passed: Slave received Buy signal for EURUSD");

    server.shutdown().await;
}

/// Test lot multiplier transformation
#[tokio::test]
async fn test_lot_multiplier() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_TRADE_002";
    let slave_account = "SLAVE_TRADE_002";

    // Create TradeGroup
    server
        .db
        .create_trade_group(master_account)
        .await
        .expect("Failed to create trade group");

    // Add Slave with lot_multiplier = 2.0
    let settings = SlaveSettings {
        lot_multiplier: Some(2.0),
        ..Default::default()
    };

    server
        .db
        .add_member(master_account, slave_account, settings)
        .await
        .expect("Failed to add member");

    // Reload settings cache to reflect the new member
    server
        .reload_settings_cache()
        .await
        .expect("Failed to reload settings cache");

    // Set all members to CONNECTED status (required for trade copying)
    server.set_all_members_connected().await;

    let master_sim = MasterEaSimulator::new(&server.zmq_pull_address(), master_account)
        .expect("Failed to create Master EA simulator");

    let slave_sim = SlaveEaSimulator::new(
        &server.zmq_pub_trade_address(),
        slave_account,
        master_account,
    )
    .expect("Failed to create Slave EA simulator");

    sleep(Duration::from_millis(200)).await;

    // Master sends 1.0 lot Buy signal
    let signal = master_sim.create_buy_signal(1002, "GBPUSD", 1.0, 1.2650);
    master_sim
        .send_signal(&signal)
        .expect("Failed to send signal");

    sleep(Duration::from_millis(200)).await;

    // Slave should receive 2.0 lot signal (1.0 * 2.0)
    let received = slave_sim
        .try_receive_signal(2000)
        .expect("Failed to receive signal");

    assert!(received.is_some(), "Slave should receive trade signal");

    let received_signal = received.unwrap();
    assert_eq!(received_signal.symbol, "GBPUSD");
    assert_eq!(
        received_signal.lots, 2.0,
        "Lot size should be multiplied by 2.0"
    );
    assert!(matches!(received_signal.order_type, OrderType::Buy));

    println!("✅ Lot multiplier test passed: 1.0 lot → 2.0 lot");

    server.shutdown().await;
}

/// Test reverse trade transformation
#[tokio::test]
async fn test_reverse_trade() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_TRADE_003";
    let slave_account = "SLAVE_TRADE_003";

    // Create TradeGroup
    server
        .db
        .create_trade_group(master_account)
        .await
        .expect("Failed to create trade group");

    // Add Slave with reverse_trade = true
    let settings = SlaveSettings {
        reverse_trade: true,
        ..Default::default()
    };

    server
        .db
        .add_member(master_account, slave_account, settings)
        .await
        .expect("Failed to add member");

    // Reload settings cache to reflect the new member
    server
        .reload_settings_cache()
        .await
        .expect("Failed to reload settings cache");

    // Set all members to CONNECTED status (required for trade copying)
    server.set_all_members_connected().await;

    let master_sim = MasterEaSimulator::new(&server.zmq_pull_address(), master_account)
        .expect("Failed to create Master EA simulator");

    let slave_sim = SlaveEaSimulator::new(
        &server.zmq_pub_trade_address(),
        slave_account,
        master_account,
    )
    .expect("Failed to create Slave EA simulator");

    sleep(Duration::from_millis(200)).await;

    // Master sends Buy signal
    let signal = master_sim.create_buy_signal(1003, "USDJPY", 0.5, 149.50);
    master_sim
        .send_signal(&signal)
        .expect("Failed to send signal");

    sleep(Duration::from_millis(200)).await;

    // Slave should receive Sell signal (reversed)
    let received = slave_sim
        .try_receive_signal(2000)
        .expect("Failed to receive signal");

    assert!(received.is_some(), "Slave should receive trade signal");

    let received_signal = received.unwrap();
    assert_eq!(received_signal.symbol, "USDJPY");
    assert_eq!(received_signal.lots, 0.5);
    assert!(
        matches!(received_signal.order_type, OrderType::Sell),
        "Buy signal should be reversed to Sell"
    );

    println!("✅ Reverse trade test passed: Buy → Sell");

    server.shutdown().await;
}

/// Test symbol mapping transformation
#[tokio::test]
async fn test_symbol_mapping() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_TRADE_004";
    let slave_account = "SLAVE_TRADE_004";

    // Create TradeGroup
    server
        .db
        .create_trade_group(master_account)
        .await
        .expect("Failed to create trade group");

    // Add Slave with symbol mapping: EURUSD → EURUSD.pro
    let settings = SlaveSettings {
        symbol_mappings: vec![SymbolMapping {
            source_symbol: "EURUSD".to_string(),
            target_symbol: "EURUSD.pro".to_string(),
        }],
        ..Default::default()
    };

    server
        .db
        .add_member(master_account, slave_account, settings)
        .await
        .expect("Failed to add member");

    // Reload settings cache to reflect the new member
    server
        .reload_settings_cache()
        .await
        .expect("Failed to reload settings cache");

    // Set all members to CONNECTED status (required for trade copying)
    server.set_all_members_connected().await;

    let master_sim = MasterEaSimulator::new(&server.zmq_pull_address(), master_account)
        .expect("Failed to create Master EA simulator");

    let slave_sim = SlaveEaSimulator::new(
        &server.zmq_pub_trade_address(),
        slave_account,
        master_account,
    )
    .expect("Failed to create Slave EA simulator");

    sleep(Duration::from_millis(200)).await;

    // Master sends signal for EURUSD
    let signal = master_sim.create_sell_signal(1004, "EURUSD", 0.8, 1.1840);
    master_sim
        .send_signal(&signal)
        .expect("Failed to send signal");

    sleep(Duration::from_millis(200)).await;

    // Slave should receive signal with mapped symbol EURUSD.pro
    let received = slave_sim
        .try_receive_signal(2000)
        .expect("Failed to receive signal");

    assert!(received.is_some(), "Slave should receive trade signal");

    let received_signal = received.unwrap();
    assert_eq!(
        received_signal.symbol, "EURUSD.pro",
        "Symbol should be mapped from EURUSD to EURUSD.pro"
    );
    assert_eq!(received_signal.lots, 0.8);
    assert!(matches!(received_signal.order_type, OrderType::Sell));

    println!("✅ Symbol mapping test passed: EURUSD → EURUSD.pro");

    server.shutdown().await;
}

/// Test multiple Slaves receiving the same signal
#[tokio::test]
async fn test_multiple_slaves() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_TRADE_005";
    let slave1_account = "SLAVE_TRADE_005A";
    let slave2_account = "SLAVE_TRADE_005B";

    // Create TradeGroup
    server
        .db
        .create_trade_group(master_account)
        .await
        .expect("Failed to create trade group");

    // Add two Slaves with different settings
    let settings1 = SlaveSettings {
        lot_multiplier: Some(1.5),
        ..Default::default()
    };

    let settings2 = SlaveSettings {
        lot_multiplier: Some(2.0),
        reverse_trade: true,
        ..Default::default()
    };

    server
        .db
        .add_member(master_account, slave1_account, settings1)
        .await
        .expect("Failed to add member 1");

    server
        .db
        .add_member(master_account, slave2_account, settings2)
        .await
        .expect("Failed to add member 2");

    // Reload settings cache to reflect the new members
    server
        .reload_settings_cache()
        .await
        .expect("Failed to reload settings cache");

    // Set all members to CONNECTED status (required for trade copying)
    server.set_all_members_connected().await;

    let master_sim = MasterEaSimulator::new(&server.zmq_pull_address(), master_account)
        .expect("Failed to create Master EA simulator");

    let slave1_sim = SlaveEaSimulator::new(
        &server.zmq_pub_trade_address(),
        slave1_account,
        master_account,
    )
    .expect("Failed to create Slave 1 EA simulator");

    let slave2_sim = SlaveEaSimulator::new(
        &server.zmq_pub_trade_address(),
        slave2_account,
        master_account,
    )
    .expect("Failed to create Slave 2 EA simulator");

    sleep(Duration::from_millis(200)).await;

    // Master sends 1.0 lot Buy signal
    let signal = master_sim.create_buy_signal(1005, "AUDUSD", 1.0, 0.6520);
    master_sim
        .send_signal(&signal)
        .expect("Failed to send signal");

    sleep(Duration::from_millis(200)).await;

    // Slave 1 should receive 1.5 lot Buy signal
    let received1 = slave1_sim
        .try_receive_signal(2000)
        .expect("Failed to receive signal for Slave 1");

    assert!(received1.is_some(), "Slave 1 should receive trade signal");
    let signal1 = received1.unwrap();
    assert_eq!(signal1.symbol, "AUDUSD");
    assert_eq!(signal1.lots, 1.5, "Slave 1: 1.0 * 1.5");
    assert!(matches!(signal1.order_type, OrderType::Buy));

    // Slave 2 receives both signals (Slave1's and Slave2's) because they subscribe to the same topic
    // First signal is for Slave 1 (1.5 lot Buy), second is for Slave 2 (2.0 lot Sell)
    let _signal_for_slave1 = slave2_sim
        .try_receive_signal(2000)
        .expect("Failed to receive first signal for Slave 2");

    // Now receive the second signal which is for Slave 2
    let received2 = slave2_sim
        .try_receive_signal(2000)
        .expect("Failed to receive second signal for Slave 2");

    assert!(received2.is_some(), "Slave 2 should receive trade signal");
    let signal2 = received2.unwrap();
    assert_eq!(signal2.symbol, "AUDUSD");
    assert_eq!(signal2.lots, 2.0, "Slave 2: 1.0 * 2.0");
    assert!(
        matches!(signal2.order_type, OrderType::Sell),
        "Slave 2: Buy reversed to Sell"
    );

    println!("✅ Multiple slaves test passed:");
    println!("   Slave 1: 1.5 lot Buy");
    println!("   Slave 2: 2.0 lot Sell (reversed)");

    server.shutdown().await;
}
