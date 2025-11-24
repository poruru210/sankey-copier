// relay-server/tests/e2e_trade_signal_test.rs
//
// E2E integration test for Trade Signal copying between Master and Slave EAs.
// This test verifies the complete flow of trade signal distribution:
// - Master EA sends trade signals
// - Relay server processes and distributes signals
// - Slave EA receives and applies configured transformations (lot multiplier, reverse trade, symbol mapping)
//
// These tests automatically spawn a relay-server instance with dynamic ports,
// making them suitable for CI/CD environments.

mod test_server;

use chrono::Utc;
use sankey_copier_relay_server::models::{
    OrderType, SlaveSettings, SymbolMapping, TradeAction, TradeSignal,
};
use test_server::TestServer;
use tokio::time::{sleep, Duration};
use zmq::{Context, Socket};

/// Master EA Simulator for trade signal testing
/// Simulates a Master EA sending trade signals to the relay server via ZMQ
struct MasterEaSimulator {
    _context: Context,
    push_socket: Socket,
    account_id: String,
}

impl MasterEaSimulator {
    /// Create a new Master EA simulator
    ///
    /// # Parameters
    /// - push_address: Address for PUSH socket (e.g., "tcp://localhost:5555")
    /// - account_id: Master account ID
    fn new(push_address: &str, account_id: &str) -> anyhow::Result<Self> {
        let context = Context::new();

        // PUSH socket for sending TradeSignals
        let push_socket = context.socket(zmq::PUSH)?;
        push_socket.set_linger(0)?;
        push_socket.connect(push_address)?;

        Ok(Self {
            _context: context,
            push_socket,
            account_id: account_id.to_string(),
        })
    }

    /// Send a trade signal
    fn send_signal(&self, signal: &TradeSignal) -> anyhow::Result<()> {
        let bytes = rmp_serde::to_vec_named(signal)?;
        self.push_socket.send(&bytes, 0)?;
        Ok(())
    }

    /// Helper to create a Buy signal
    fn create_buy_signal(
        &self,
        ticket: i64,
        symbol: &str,
        lots: f64,
        price: f64,
    ) -> TradeSignal {
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
    fn create_sell_signal(
        &self,
        ticket: i64,
        symbol: &str,
        lots: f64,
        price: f64,
    ) -> TradeSignal {
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

/// Slave EA Simulator for trade signal testing
/// Simulates a Slave EA receiving trade signals from the relay server via ZMQ
struct SlaveEaSimulator {
    _context: Context,
    trade_socket: Socket,
    account_id: String,
}

impl SlaveEaSimulator {
    /// Create a new Slave EA simulator
    ///
    /// # Parameters
    /// - trade_address: Address for SUB socket (e.g., "tcp://localhost:5556")
    /// - account_id: Slave account ID
    /// - master_account: Master account ID to subscribe to
    fn new(
        trade_address: &str,
        account_id: &str,
        master_account: &str,
    ) -> anyhow::Result<Self> {
        let context = Context::new();

        // SUB socket for receiving TradeSignals
        let trade_socket = context.socket(zmq::SUB)?;
        trade_socket.set_linger(0)?;
        trade_socket.connect(trade_address)?;
        // Subscribe to signals from the specific master account (topic-based filtering)
        trade_socket.set_subscribe(master_account.as_bytes())?;

        Ok(Self {
            _context: context,
            trade_socket,
            account_id: account_id.to_string(),
        })
    }

    /// Try to receive a trade signal (with timeout)
    ///
    /// # Parameters
    /// - timeout_ms: Timeout in milliseconds
    ///
    /// # Returns
    /// - Ok(Some(signal)): Successfully received and parsed signal
    /// - Ok(None): Timeout (no signal received)
    /// - Err: Error during receive or parsing
    fn try_receive_signal(&self, timeout_ms: i32) -> anyhow::Result<Option<TradeSignal>> {
        self.trade_socket.set_rcvtimeo(timeout_ms)?;

        match self.trade_socket.recv_bytes(0) {
            Ok(bytes) => {
                // Message format: topic + space + MessagePack payload
                let space_pos = bytes
                    .iter()
                    .position(|&b| b == b' ')
                    .ok_or_else(|| anyhow::anyhow!("Invalid message format: no space separator"))?;

                // Extract payload (skip topic)
                let payload = &bytes[space_pos + 1..];

                // Deserialize MessagePack payload
                let signal: TradeSignal = rmp_serde::from_slice(payload)?;
                Ok(Some(signal))
            }
            Err(zmq::Error::EAGAIN) => Ok(None), // Timeout
            Err(e) => Err(e.into()),
        }
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
    let master_sim =
        MasterEaSimulator::new(&server.zmq_pull_address(), master_account)
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
    let mut settings = SlaveSettings::default();
    settings.lot_multiplier = Some(2.0);

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
    let mut settings = SlaveSettings::default();
    settings.reverse_trade = true;

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
    let mut settings = SlaveSettings::default();
    settings.symbol_mappings = vec![SymbolMapping {
        source_symbol: "EURUSD".to_string(),
        target_symbol: "EURUSD.pro".to_string(),
    }];

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
    let mut settings1 = SlaveSettings::default();
    settings1.lot_multiplier = Some(1.5);

    let mut settings2 = SlaveSettings::default();
    settings2.lot_multiplier = Some(2.0);
    settings2.reverse_trade = true;

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
