// relay-server/tests/e2e_trade_signal_test.rs
//
// E2E integration tests for TradeSignal processing.
// Tests the complete flow of trade signal distribution:
// - Master EA sends Open/Close/Modify signals
// - Relay Server processes and distributes to Slaves
// - Slave EAs receive transformed signals
//
// Categories covered:
// 1. Basic order lifecycle (Open -> Close)
// 2. Multiple orders (sequential/parallel)
// 3. Multiple Masters (signal isolation)
// 4. Multiple Slaves (broadcast/filtering)
// 5. TP/SL modifications
// 6. Latency measurements

mod test_server;

use chrono::{Duration as ChronoDuration, Utc};
use sankey_copier_relay_server::models::{
    LotCalculationMode, MasterSettings, OrderType, SlaveSettings, SyncMode, TradeAction,
    TradeSignal,
};
use sankey_copier_zmq::{
    zmq_context_create, zmq_context_destroy, zmq_socket_connect, zmq_socket_create,
    zmq_socket_destroy, zmq_socket_receive, zmq_socket_send_binary, zmq_socket_subscribe,
    HeartbeatMessage, SymbolMapping, TradeFilters, TradeSignalMessage, ZMQ_PUSH, ZMQ_SUB,
};
use std::ffi::c_char;
use test_server::TestServer;
use tokio::time::{sleep, Duration};

// =============================================================================
// Constants
// =============================================================================

const STATUS_CONNECTED: i32 = 2;
const BUFFER_SIZE: usize = 65536;

// =============================================================================
// Master EA Simulator (Extended for TradeSignal)
// =============================================================================

/// Master EA Simulator with TradeSignal support
struct MasterEaSimulator {
    context_handle: i32,
    push_socket_handle: i32,
    config_socket_handle: i32,
    account_id: String,
}

impl MasterEaSimulator {
    /// Create a new Master EA simulator
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
        let topic_utf16: Vec<u16> = account_id.encode_utf16().chain(Some(0)).collect();

        unsafe {
            if zmq_socket_connect(push_socket_handle, push_addr_utf16.as_ptr()) != 1 {
                zmq_socket_destroy(config_socket_handle);
                zmq_socket_destroy(push_socket_handle);
                zmq_context_destroy(context_handle);
                anyhow::bail!("Failed to connect PUSH socket");
            }

            if zmq_socket_connect(config_socket_handle, config_addr_utf16.as_ptr()) != 1 {
                zmq_socket_destroy(config_socket_handle);
                zmq_socket_destroy(push_socket_handle);
                zmq_context_destroy(context_handle);
                anyhow::bail!("Failed to connect config SUB socket");
            }

            if zmq_socket_subscribe(config_socket_handle, topic_utf16.as_ptr()) != 1 {
                zmq_socket_destroy(config_socket_handle);
                zmq_socket_destroy(push_socket_handle);
                zmq_context_destroy(context_handle);
                anyhow::bail!("Failed to subscribe to config topic");
            }
        }

        Ok(Self {
            context_handle,
            push_socket_handle,
            config_socket_handle,
            account_id: account_id.to_string(),
        })
    }

    /// Get account ID
    #[allow(dead_code)]
    fn account_id(&self) -> &str {
        &self.account_id
    }

    /// Send heartbeat message
    fn send_heartbeat(&self) -> anyhow::Result<()> {
        let msg = HeartbeatMessage {
            message_type: "Heartbeat".to_string(),
            account_id: self.account_id.clone(),
            balance: 50000.0,
            equity: 50000.0,
            open_positions: 0,
            timestamp: Utc::now().to_rfc3339(),
            version: "test-master-1.0.0".to_string(),
            ea_type: "Master".to_string(),
            platform: "MT5".to_string(),
            account_number: 12345,
            broker: "TestBroker".to_string(),
            account_name: "MasterTestAccount".to_string(),
            server: "TestServer".to_string(),
            currency: "USD".to_string(),
            leverage: 500,
            is_trade_allowed: true,
            symbol_prefix: None,
            symbol_suffix: None,
            symbol_map: None,
        };

        let bytes = rmp_serde::to_vec_named(&msg)?;
        unsafe {
            if zmq_socket_send_binary(self.push_socket_handle, bytes.as_ptr(), bytes.len() as i32)
                != 1
            {
                anyhow::bail!("Failed to send heartbeat");
            }
        }
        Ok(())
    }

    /// Send a TradeSignal using mt-bridge's TradeSignalMessage format
    /// This tests the full flow: mt-bridge serialization -> relay-server parsing
    fn send_trade_signal(&self, signal: &TradeSignalMessage) -> anyhow::Result<()> {
        let bytes = rmp_serde::to_vec_named(signal)?;
        unsafe {
            if zmq_socket_send_binary(self.push_socket_handle, bytes.as_ptr(), bytes.len() as i32)
                != 1
            {
                anyhow::bail!("Failed to send trade signal");
            }
        }
        Ok(())
    }

    /// Convert OrderType to string format used by mt-bridge
    fn order_type_to_string(order_type: OrderType) -> String {
        match order_type {
            OrderType::Buy => "Buy".to_string(),
            OrderType::Sell => "Sell".to_string(),
            OrderType::BuyLimit => "BuyLimit".to_string(),
            OrderType::SellLimit => "SellLimit".to_string(),
            OrderType::BuyStop => "BuyStop".to_string(),
            OrderType::SellStop => "SellStop".to_string(),
        }
    }

    /// Create an Open signal using mt-bridge's TradeSignalMessage format
    #[allow(clippy::too_many_arguments)]
    fn create_open_signal(
        &self,
        ticket: i64,
        symbol: &str,
        order_type: OrderType,
        lots: f64,
        price: f64,
        sl: Option<f64>,
        tp: Option<f64>,
        magic: i32,
    ) -> TradeSignalMessage {
        TradeSignalMessage {
            action: "Open".to_string(),
            ticket,
            symbol: Some(symbol.to_string()),
            order_type: Some(Self::order_type_to_string(order_type)),
            lots: Some(lots),
            open_price: Some(price),
            stop_loss: sl,
            take_profit: tp,
            magic_number: Some(magic as i64),
            comment: Some("E2E Test".to_string()),
            timestamp: Utc::now().to_rfc3339(),
            source_account: self.account_id.clone(),
            close_ratio: None, // Not applicable for Open
        }
    }

    /// Create a Close signal (full close) using mt-bridge's TradeSignalMessage format
    fn create_close_signal(&self, ticket: i64, symbol: &str, lots: f64) -> TradeSignalMessage {
        TradeSignalMessage {
            action: "Close".to_string(),
            ticket,
            symbol: Some(symbol.to_string()),
            order_type: Some("Buy".to_string()),
            lots: Some(lots),
            open_price: None,
            stop_loss: None,
            take_profit: None,
            magic_number: Some(0),
            comment: Some("E2E Test Close".to_string()),
            timestamp: Utc::now().to_rfc3339(),
            source_account: self.account_id.clone(),
            close_ratio: None, // None = full close
        }
    }

    /// Create a Partial Close signal with close_ratio using mt-bridge format
    /// close_ratio: 0.0 < ratio < 1.0 indicates partial close percentage
    fn create_partial_close_signal(
        &self,
        ticket: i64,
        symbol: &str,
        lots: f64,
        close_ratio: f64,
    ) -> TradeSignalMessage {
        TradeSignalMessage {
            action: "Close".to_string(),
            ticket,
            symbol: Some(symbol.to_string()),
            order_type: Some("Buy".to_string()),
            lots: Some(lots),
            open_price: None,
            stop_loss: None,
            take_profit: None,
            magic_number: Some(0),
            comment: Some("E2E Test Partial Close".to_string()),
            timestamp: Utc::now().to_rfc3339(),
            source_account: self.account_id.clone(),
            close_ratio: Some(close_ratio),
        }
    }

    /// Create a Modify signal (TP/SL change) using mt-bridge format
    fn create_modify_signal(
        &self,
        ticket: i64,
        symbol: &str,
        sl: Option<f64>,
        tp: Option<f64>,
    ) -> TradeSignalMessage {
        TradeSignalMessage {
            action: "Modify".to_string(),
            ticket,
            symbol: Some(symbol.to_string()),
            order_type: None,
            lots: None,
            open_price: None,
            stop_loss: sl,
            take_profit: tp,
            magic_number: Some(0),
            comment: Some("E2E Test Modify".to_string()),
            timestamp: Utc::now().to_rfc3339(),
            source_account: self.account_id.clone(),
            close_ratio: None,
        }
    }

    /// Create a delayed signal (timestamp in the past)
    fn create_delayed_signal(
        &self,
        mut signal: TradeSignalMessage,
        delay_ms: i64,
    ) -> TradeSignalMessage {
        let past_time = Utc::now() - ChronoDuration::milliseconds(delay_ms);
        signal.timestamp = past_time.to_rfc3339();
        signal
    }
}

impl Drop for MasterEaSimulator {
    fn drop(&mut self) {
        zmq_socket_destroy(self.config_socket_handle);
        zmq_socket_destroy(self.push_socket_handle);
        zmq_context_destroy(self.context_handle);
    }
}

// =============================================================================
// Slave EA Simulator (Extended for TradeSignal)
// =============================================================================

/// Slave EA Simulator with TradeSignal support
struct SlaveEaSimulator {
    context_handle: i32,
    push_socket_handle: i32,
    config_socket_handle: i32,
    trade_socket_handle: i32,
    account_id: String,
}

impl SlaveEaSimulator {
    /// Create a new Slave EA simulator
    fn new(
        push_address: &str,
        config_address: &str,
        trade_address: &str,
        account_id: &str,
    ) -> anyhow::Result<Self> {
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
            anyhow::bail!("Failed to create config SUB socket");
        }

        let trade_socket_handle = zmq_socket_create(context_handle, ZMQ_SUB);
        if trade_socket_handle < 0 {
            zmq_socket_destroy(config_socket_handle);
            zmq_socket_destroy(push_socket_handle);
            zmq_context_destroy(context_handle);
            anyhow::bail!("Failed to create trade SUB socket");
        }

        let push_addr_utf16: Vec<u16> = push_address.encode_utf16().chain(Some(0)).collect();
        let config_addr_utf16: Vec<u16> = config_address.encode_utf16().chain(Some(0)).collect();
        let trade_addr_utf16: Vec<u16> = trade_address.encode_utf16().chain(Some(0)).collect();
        let account_topic_utf16: Vec<u16> = account_id.encode_utf16().chain(Some(0)).collect();

        unsafe {
            if zmq_socket_connect(push_socket_handle, push_addr_utf16.as_ptr()) != 1 {
                zmq_socket_destroy(trade_socket_handle);
                zmq_socket_destroy(config_socket_handle);
                zmq_socket_destroy(push_socket_handle);
                zmq_context_destroy(context_handle);
                anyhow::bail!("Failed to connect PUSH socket");
            }

            if zmq_socket_connect(config_socket_handle, config_addr_utf16.as_ptr()) != 1 {
                zmq_socket_destroy(trade_socket_handle);
                zmq_socket_destroy(config_socket_handle);
                zmq_socket_destroy(push_socket_handle);
                zmq_context_destroy(context_handle);
                anyhow::bail!("Failed to connect config SUB socket");
            }

            if zmq_socket_connect(trade_socket_handle, trade_addr_utf16.as_ptr()) != 1 {
                zmq_socket_destroy(trade_socket_handle);
                zmq_socket_destroy(config_socket_handle);
                zmq_socket_destroy(push_socket_handle);
                zmq_context_destroy(context_handle);
                anyhow::bail!("Failed to connect trade SUB socket");
            }

            // Subscribe to config messages for this slave account
            if zmq_socket_subscribe(config_socket_handle, account_topic_utf16.as_ptr()) != 1 {
                zmq_socket_destroy(trade_socket_handle);
                zmq_socket_destroy(config_socket_handle);
                zmq_socket_destroy(push_socket_handle);
                zmq_context_destroy(context_handle);
                anyhow::bail!("Failed to subscribe to config topic");
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

    /// Subscribe to trade signals from a specific master account
    /// Server sends signals to trade_group_id (master_account) topic
    /// All slaves under the same master subscribe to this topic
    fn subscribe_to_master(&self, master_account: &str) -> anyhow::Result<()> {
        let topic_utf16: Vec<u16> = master_account.encode_utf16().chain(Some(0)).collect();
        unsafe {
            if zmq_socket_subscribe(self.trade_socket_handle, topic_utf16.as_ptr()) != 1 {
                anyhow::bail!("Failed to subscribe to master: {}", master_account);
            }
        }
        Ok(())
    }

    #[allow(dead_code)]
    /// Subscribe to trade signals for this slave's own account (not used in current design)
    fn subscribe_to_self(&self) -> anyhow::Result<()> {
        let topic_utf16: Vec<u16> = self.account_id.encode_utf16().chain(Some(0)).collect();
        unsafe {
            if zmq_socket_subscribe(self.trade_socket_handle, topic_utf16.as_ptr()) != 1 {
                anyhow::bail!("Failed to subscribe to self: {}", self.account_id);
            }
        }
        Ok(())
    }

    /// Send heartbeat message
    fn send_heartbeat(&self) -> anyhow::Result<()> {
        let msg = HeartbeatMessage {
            message_type: "Heartbeat".to_string(),
            account_id: self.account_id.clone(),
            balance: 10000.0,
            equity: 10000.0,
            open_positions: 0,
            timestamp: Utc::now().to_rfc3339(),
            version: "test-slave-1.0.0".to_string(),
            ea_type: "Slave".to_string(),
            platform: "MT5".to_string(),
            account_number: 54321,
            broker: "TestBroker".to_string(),
            account_name: "SlaveTestAccount".to_string(),
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
            if zmq_socket_send_binary(self.push_socket_handle, bytes.as_ptr(), bytes.len() as i32)
                != 1
            {
                anyhow::bail!("Failed to send heartbeat");
            }
        }
        Ok(())
    }

    /// Try to receive a TradeSignal with timeout
    fn try_receive_trade_signal(
        &self,
        timeout_ms: i32,
    ) -> anyhow::Result<Option<(String, TradeSignal)>> {
        let mut buffer = vec![0u8; BUFFER_SIZE];
        let start = std::time::Instant::now();
        let timeout_duration = std::time::Duration::from_millis(timeout_ms as u64);

        loop {
            let received_bytes = unsafe {
                zmq_socket_receive(
                    self.trade_socket_handle,
                    buffer.as_mut_ptr() as *mut c_char,
                    BUFFER_SIZE as i32,
                )
            };

            if received_bytes > 0 {
                let bytes = &buffer[..received_bytes as usize];

                // Parse topic + space + payload format
                let space_pos = bytes
                    .iter()
                    .position(|&b| b == b' ')
                    .ok_or_else(|| anyhow::anyhow!("Invalid message format: no space separator"))?;

                let topic = String::from_utf8_lossy(&bytes[..space_pos]).to_string();
                let payload = &bytes[space_pos + 1..];

                let signal: TradeSignal = rmp_serde::from_slice(payload)?;
                return Ok(Some((topic, signal)));
            } else if received_bytes == 0 {
                if start.elapsed() >= timeout_duration {
                    return Ok(None);
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            } else {
                return Err(anyhow::anyhow!("Failed to receive trade signal"));
            }
        }
    }

    /// Collect multiple trade signals within timeout
    fn collect_trade_signals(
        &self,
        timeout_ms: i32,
        max_signals: usize,
    ) -> anyhow::Result<Vec<(String, TradeSignal)>> {
        let mut signals = Vec::new();
        let start = std::time::Instant::now();
        let timeout_duration = std::time::Duration::from_millis(timeout_ms as u64);

        while signals.len() < max_signals && start.elapsed() < timeout_duration {
            let remaining = (timeout_duration - start.elapsed()).as_millis() as i32;
            if remaining <= 0 {
                break;
            }

            // Use shorter poll interval for better responsiveness
            match self.try_receive_trade_signal(remaining.min(50))? {
                Some(signal) => {
                    signals.push(signal);
                    // Brief pause to allow more signals to arrive
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                None => {
                    // Keep polling until timeout
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }
        }
        Ok(signals)
    }
}

impl Drop for SlaveEaSimulator {
    fn drop(&mut self) {
        zmq_socket_destroy(self.trade_socket_handle);
        zmq_socket_destroy(self.config_socket_handle);
        zmq_socket_destroy(self.push_socket_handle);
        zmq_context_destroy(self.context_handle);
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Create default slave settings for testing
fn default_test_slave_settings() -> SlaveSettings {
    SlaveSettings {
        lot_calculation_mode: LotCalculationMode::Multiplier,
        lot_multiplier: Some(1.0),
        reverse_trade: false,
        symbol_prefix: None,
        symbol_suffix: None,
        symbol_mappings: vec![],
        filters: TradeFilters::default(),
        config_version: 0,
        source_lot_min: None,
        source_lot_max: None,
        sync_mode: SyncMode::Skip,
        limit_order_expiry_min: None,
        market_sync_max_pips: None,
        max_slippage: None,
        copy_pending_orders: false,
        // Trade Execution defaults
        max_retries: 3,
        max_signal_delay_ms: 5000,
        use_pending_order_for_delayed: false,
    }
}

/// Setup test scenario with master and slaves
async fn setup_test_scenario(
    server: &TestServer,
    master_account: &str,
    slave_accounts: &[&str],
    slave_settings_fn: impl Fn(usize) -> SlaveSettings,
) -> anyhow::Result<()> {
    // Create trade group for master
    server.db.create_trade_group(master_account).await?;

    // Add slaves with settings
    for (i, slave_account) in slave_accounts.iter().enumerate() {
        let settings = slave_settings_fn(i);
        server
            .db
            .add_member(master_account, slave_account, settings)
            .await?;

        // Enable slave (set status to CONNECTED for trade copying)
        server
            .db
            .update_member_status(master_account, slave_account, STATUS_CONNECTED)
            .await?;
    }

    Ok(())
}

/// Register all EAs by sending heartbeats
async fn register_all_eas(
    master: &MasterEaSimulator,
    slaves: &[&SlaveEaSimulator],
) -> anyhow::Result<()> {
    master.send_heartbeat()?;
    for slave in slaves {
        slave.send_heartbeat()?;
    }
    // Wait for connections to establish.
    // The "slow joiner" problem in ZeroMQ requires sufficient time
    // for SUB subscriptions to propagate to the PUB socket.
    sleep(Duration::from_millis(500)).await;
    Ok(())
}

// =============================================================================
// Category 1: Basic Order Lifecycle Tests
// =============================================================================

/// Test basic Open -> Close cycle
#[tokio::test]
async fn test_open_close_cycle() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_OPEN_CLOSE_001";
    let slave_account = "SLAVE_OPEN_CLOSE_001";

    // Setup: Create trade group and add slave
    setup_test_scenario(&server, master_account, &[slave_account], |_| {
        default_test_slave_settings()
    })
    .await
    .expect("Failed to setup test scenario");

    // Create simulators
    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .expect("Failed to create master simulator");

    let slave = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave_account,
    )
    .expect("Failed to create slave simulator");

    // Subscribe slave to master's trade signals
    slave
        .subscribe_to_master(master_account)
        .expect("Failed to subscribe to master");

    // Register EAs
    register_all_eas(&master, &[&slave])
        .await
        .expect("Failed to register EAs");

    // Step 1: Master sends Open signal
    let open_signal = master.create_open_signal(
        12345,
        "EURUSD",
        OrderType::Buy,
        0.1,
        1.0850,
        Some(1.0800),
        Some(1.0900),
        0,
    );
    master
        .send_trade_signal(&open_signal)
        .expect("Failed to send Open signal");

    // Wait for signal to be processed
    sleep(Duration::from_millis(200)).await;

    // Step 2: Master sends Close signal
    let close_signal = master.create_close_signal(12345, "EURUSD", 0.1);
    master
        .send_trade_signal(&close_signal)
        .expect("Failed to send Close signal");

    // Wait for close signal to be processed
    sleep(Duration::from_millis(200)).await;

    // Step 3: Collect signals at slave
    let signals = slave
        .collect_trade_signals(3000, 2)
        .expect("Failed to collect signals");

    // Verify: Slave received 2 signals
    assert_eq!(signals.len(), 2, "Slave should receive 2 signals");

    // Verify Open signal
    let (topic1, sig1) = &signals[0];
    assert_eq!(topic1, master_account);
    assert!(matches!(sig1.action, TradeAction::Open));
    assert_eq!(sig1.ticket, 12345);
    assert_eq!(sig1.symbol.as_deref(), Some("EURUSD"));
    assert_eq!(sig1.lots, Some(0.1));

    // Verify Close signal
    let (topic2, sig2) = &signals[1];
    assert_eq!(topic2, master_account);
    assert!(matches!(sig2.action, TradeAction::Close));
    assert_eq!(sig2.ticket, 12345);

    println!("✅ test_open_close_cycle passed");

    server.shutdown().await;
}

/// Test Open -> Modify -> Close cycle
#[tokio::test]
async fn test_open_modify_close_cycle() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_MODIFY_001";
    let slave_account = "SLAVE_MODIFY_001";

    setup_test_scenario(&server, master_account, &[slave_account], |_| {
        default_test_slave_settings()
    })
    .await
    .expect("Failed to setup");

    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .expect("Failed to create master");

    let slave = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave_account,
    )
    .expect("Failed to create slave");

    slave.subscribe_to_master(master_account).unwrap();
    register_all_eas(&master, &[&slave]).await.unwrap();

    // Open
    let open_signal = master.create_open_signal(
        12346,
        "EURUSD",
        OrderType::Buy,
        0.1,
        1.0850,
        Some(1.0800),
        Some(1.0900),
        0,
    );
    master.send_trade_signal(&open_signal).unwrap();
    sleep(Duration::from_millis(100)).await;

    // Modify (change SL/TP)
    let modify_signal = master.create_modify_signal(12346, "EURUSD", Some(1.0750), Some(1.0950));
    master.send_trade_signal(&modify_signal).unwrap();
    sleep(Duration::from_millis(100)).await;

    // Close
    let close_signal = master.create_close_signal(12346, "EURUSD", 0.1);
    master.send_trade_signal(&close_signal).unwrap();

    sleep(Duration::from_millis(200)).await;
    let signals = slave.collect_trade_signals(3000, 3).unwrap();

    assert_eq!(signals.len(), 3, "Should receive 3 signals");

    // Verify order: Open, Modify, Close
    assert!(matches!(signals[0].1.action, TradeAction::Open));
    assert!(matches!(signals[1].1.action, TradeAction::Modify));
    assert!(matches!(signals[2].1.action, TradeAction::Close));

    // Verify Modify has updated SL/TP
    assert_eq!(signals[1].1.stop_loss, Some(1.0750));
    assert_eq!(signals[1].1.take_profit, Some(1.0950));

    println!("✅ test_open_modify_close_cycle passed");

    server.shutdown().await;
}

/// Test Close signal for non-existent position (should still be relayed)
#[tokio::test]
async fn test_close_nonexistent_position() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_NONEXIST_001";
    let slave_account = "SLAVE_NONEXIST_001";

    setup_test_scenario(&server, master_account, &[slave_account], |_| {
        default_test_slave_settings()
    })
    .await
    .unwrap();

    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .unwrap();

    let slave = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave_account,
    )
    .unwrap();

    slave.subscribe_to_master(master_account).unwrap();
    register_all_eas(&master, &[&slave]).await.unwrap();

    // Send Close for ticket that was never opened
    let close_signal = master.create_close_signal(99999, "EURUSD", 0.1);
    master.send_trade_signal(&close_signal).unwrap();

    sleep(Duration::from_millis(200)).await;
    let signals = slave.collect_trade_signals(3000, 1).unwrap();

    // Server should still relay the signal (doesn't track position state)
    assert_eq!(signals.len(), 1, "Signal should be relayed");
    assert!(matches!(signals[0].1.action, TradeAction::Close));
    assert_eq!(signals[0].1.ticket, 99999);

    println!("✅ test_close_nonexistent_position passed");

    server.shutdown().await;
}

/// Test duplicate Close signals (double close)
#[tokio::test]
async fn test_close_already_closed() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_DOUBLE_CLOSE_001";
    let slave_account = "SLAVE_DOUBLE_CLOSE_001";

    setup_test_scenario(&server, master_account, &[slave_account], |_| {
        default_test_slave_settings()
    })
    .await
    .unwrap();

    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .unwrap();

    let slave = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave_account,
    )
    .unwrap();

    slave.subscribe_to_master(master_account).unwrap();
    register_all_eas(&master, &[&slave]).await.unwrap();

    // Open
    let open_signal =
        master.create_open_signal(12347, "EURUSD", OrderType::Buy, 0.1, 1.0850, None, None, 0);
    master.send_trade_signal(&open_signal).unwrap();
    sleep(Duration::from_millis(100)).await;

    // First Close
    let close1 = master.create_close_signal(12347, "EURUSD", 0.1);
    master.send_trade_signal(&close1).unwrap();
    sleep(Duration::from_millis(100)).await;

    // Second Close (duplicate)
    let close2 = master.create_close_signal(12347, "EURUSD", 0.1);
    master.send_trade_signal(&close2).unwrap();

    sleep(Duration::from_millis(200)).await;
    let signals = slave.collect_trade_signals(3000, 3).unwrap();

    // Server doesn't deduplicate - all 3 signals should be delivered
    assert_eq!(
        signals.len(),
        3,
        "All signals should be delivered (dedup is EA's job)"
    );

    println!("✅ test_close_already_closed passed");

    server.shutdown().await;
}

// =============================================================================
// Category 5: TP/SL (Modify) Tests
// =============================================================================

/// Test Modify with SL only
#[tokio::test]
async fn test_modify_sl_only() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_SL_ONLY_001";
    let slave_account = "SLAVE_SL_ONLY_001";

    setup_test_scenario(&server, master_account, &[slave_account], |_| {
        default_test_slave_settings()
    })
    .await
    .unwrap();

    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .unwrap();

    let slave = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave_account,
    )
    .unwrap();

    slave.subscribe_to_master(master_account).unwrap();
    register_all_eas(&master, &[&slave]).await.unwrap();

    // Send Modify with SL only
    let modify_signal = master.create_modify_signal(12348, "EURUSD", Some(1.0750), None);
    master.send_trade_signal(&modify_signal).unwrap();

    sleep(Duration::from_millis(200)).await;
    let signals = slave.collect_trade_signals(3000, 1).unwrap();

    assert_eq!(signals.len(), 1);
    assert!(matches!(signals[0].1.action, TradeAction::Modify));
    assert_eq!(signals[0].1.stop_loss, Some(1.0750));
    assert_eq!(signals[0].1.take_profit, None);

    println!("✅ test_modify_sl_only passed");

    server.shutdown().await;
}

/// Test Modify with TP only
#[tokio::test]
async fn test_modify_tp_only() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_TP_ONLY_001";
    let slave_account = "SLAVE_TP_ONLY_001";

    setup_test_scenario(&server, master_account, &[slave_account], |_| {
        default_test_slave_settings()
    })
    .await
    .unwrap();

    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .unwrap();

    let slave = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave_account,
    )
    .unwrap();

    slave.subscribe_to_master(master_account).unwrap();
    register_all_eas(&master, &[&slave]).await.unwrap();

    let modify_signal = master.create_modify_signal(12349, "EURUSD", None, Some(1.0950));
    master.send_trade_signal(&modify_signal).unwrap();

    sleep(Duration::from_millis(200)).await;
    let signals = slave.collect_trade_signals(3000, 1).unwrap();

    assert_eq!(signals.len(), 1);
    assert!(matches!(signals[0].1.action, TradeAction::Modify));
    assert_eq!(signals[0].1.stop_loss, None);
    assert_eq!(signals[0].1.take_profit, Some(1.0950));

    println!("✅ test_modify_tp_only passed");

    server.shutdown().await;
}

/// Test Modify with both SL and TP
#[tokio::test]
async fn test_modify_both_sl_tp() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_BOTH_SLTP_001";
    let slave_account = "SLAVE_BOTH_SLTP_001";

    setup_test_scenario(&server, master_account, &[slave_account], |_| {
        default_test_slave_settings()
    })
    .await
    .unwrap();

    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .unwrap();

    let slave = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave_account,
    )
    .unwrap();

    slave.subscribe_to_master(master_account).unwrap();
    register_all_eas(&master, &[&slave]).await.unwrap();

    let modify_signal = master.create_modify_signal(12350, "EURUSD", Some(1.0700), Some(1.1000));
    master.send_trade_signal(&modify_signal).unwrap();

    sleep(Duration::from_millis(200)).await;
    let signals = slave.collect_trade_signals(3000, 1).unwrap();

    assert_eq!(signals.len(), 1);
    assert!(matches!(signals[0].1.action, TradeAction::Modify));
    assert_eq!(signals[0].1.stop_loss, Some(1.0700));
    assert_eq!(signals[0].1.take_profit, Some(1.1000));

    println!("✅ test_modify_both_sl_tp passed");

    server.shutdown().await;
}

/// Test multiple Modify signals in sequence
#[tokio::test]
async fn test_modify_multiple_times() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_MULTI_MODIFY_001";
    let slave_account = "SLAVE_MULTI_MODIFY_001";

    setup_test_scenario(&server, master_account, &[slave_account], |_| {
        default_test_slave_settings()
    })
    .await
    .unwrap();

    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .unwrap();

    let slave = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave_account,
    )
    .unwrap();

    slave.subscribe_to_master(master_account).unwrap();
    register_all_eas(&master, &[&slave]).await.unwrap();

    // Send 3 Modify signals with different SL/TP values
    let modify1 = master.create_modify_signal(12351, "EURUSD", Some(1.0800), Some(1.0900));
    master.send_trade_signal(&modify1).unwrap();
    sleep(Duration::from_millis(100)).await;

    let modify2 = master.create_modify_signal(12351, "EURUSD", Some(1.0750), Some(1.0950));
    master.send_trade_signal(&modify2).unwrap();
    sleep(Duration::from_millis(100)).await;

    let modify3 = master.create_modify_signal(12351, "EURUSD", Some(1.0700), Some(1.1000));
    master.send_trade_signal(&modify3).unwrap();

    sleep(Duration::from_millis(200)).await;
    let signals = slave.collect_trade_signals(3000, 3).unwrap();

    assert_eq!(signals.len(), 3, "Should receive all 3 Modify signals");

    // Verify values in order
    assert_eq!(signals[0].1.stop_loss, Some(1.0800));
    assert_eq!(signals[1].1.stop_loss, Some(1.0750));
    assert_eq!(signals[2].1.stop_loss, Some(1.0700));

    println!("✅ test_modify_multiple_times passed");

    server.shutdown().await;
}

// =============================================================================
// Category 2: Multiple Orders Tests
// =============================================================================

/// Test sequential Open signals
#[tokio::test]
async fn test_multiple_open_sequential() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_SEQ_OPEN_001";
    let slave_account = "SLAVE_SEQ_OPEN_001";

    setup_test_scenario(&server, master_account, &[slave_account], |_| {
        default_test_slave_settings()
    })
    .await
    .unwrap();

    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .unwrap();

    let slave = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave_account,
    )
    .unwrap();

    slave.subscribe_to_master(master_account).unwrap();
    register_all_eas(&master, &[&slave]).await.unwrap();

    // Send 3 Open signals sequentially
    for i in 1..=3 {
        let signal =
            master.create_open_signal(i, "EURUSD", OrderType::Buy, 0.1, 1.0850, None, None, 0);
        master.send_trade_signal(&signal).unwrap();
        sleep(Duration::from_millis(50)).await;
    }

    let signals = slave.collect_trade_signals(2000, 3).unwrap();

    assert_eq!(signals.len(), 3, "Should receive 3 signals");

    // Verify tickets in order
    assert_eq!(signals[0].1.ticket, 1);
    assert_eq!(signals[1].1.ticket, 2);
    assert_eq!(signals[2].1.ticket, 3);

    println!("✅ test_multiple_open_sequential passed");

    server.shutdown().await;
}

/// Test parallel Open signals (burst)
#[tokio::test]
async fn test_multiple_open_parallel() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_PAR_OPEN_001";
    let slave_account = "SLAVE_PAR_OPEN_001";

    setup_test_scenario(&server, master_account, &[slave_account], |_| {
        default_test_slave_settings()
    })
    .await
    .unwrap();

    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .unwrap();

    let slave = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave_account,
    )
    .unwrap();

    slave.subscribe_to_master(master_account).unwrap();
    register_all_eas(&master, &[&slave]).await.unwrap();

    // Send 5 Open signals with minimal delay
    for i in 1..=5 {
        let signal =
            master.create_open_signal(i, "EURUSD", OrderType::Buy, 0.1, 1.0850, None, None, 0);
        master.send_trade_signal(&signal).unwrap();
        sleep(Duration::from_millis(20)).await;
    }

    sleep(Duration::from_millis(300)).await;
    let signals = slave.collect_trade_signals(3000, 5).unwrap();

    assert_eq!(signals.len(), 5, "Should receive all 5 signals");

    // Verify all tickets are present (order may vary)
    let tickets: Vec<i64> = signals.iter().map(|(_, s)| s.ticket).collect();
    for i in 1..=5 {
        assert!(tickets.contains(&i), "Missing ticket {}", i);
    }

    println!("✅ test_multiple_open_parallel passed");

    server.shutdown().await;
}

/// Test sequential Close signals after Opens
#[tokio::test]
async fn test_multiple_close_sequential() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_SEQ_CLOSE_001";
    let slave_account = "SLAVE_SEQ_CLOSE_001";

    setup_test_scenario(&server, master_account, &[slave_account], |_| {
        default_test_slave_settings()
    })
    .await
    .unwrap();

    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .unwrap();

    let slave = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave_account,
    )
    .unwrap();

    slave.subscribe_to_master(master_account).unwrap();
    register_all_eas(&master, &[&slave]).await.unwrap();

    // Open 3 positions
    for i in 1..=3 {
        let signal =
            master.create_open_signal(i, "EURUSD", OrderType::Buy, 0.1, 1.0850, None, None, 0);
        master.send_trade_signal(&signal).unwrap();
        sleep(Duration::from_millis(30)).await;
    }

    // Close 3 positions
    for i in 1..=3 {
        let signal = master.create_close_signal(i, "EURUSD", 0.1);
        master.send_trade_signal(&signal).unwrap();
        sleep(Duration::from_millis(30)).await;
    }

    let signals = slave.collect_trade_signals(3000, 6).unwrap();

    assert_eq!(
        signals.len(),
        6,
        "Should receive 6 signals (3 Open + 3 Close)"
    );

    // Count Open and Close signals
    let open_count = signals
        .iter()
        .filter(|(_, s)| matches!(s.action, TradeAction::Open))
        .count();
    let close_count = signals
        .iter()
        .filter(|(_, s)| matches!(s.action, TradeAction::Close))
        .count();

    assert_eq!(open_count, 3);
    assert_eq!(close_count, 3);

    println!("✅ test_multiple_close_sequential passed");

    server.shutdown().await;
}

/// Test rapid-fire signals (stress test)
#[tokio::test]
async fn test_rapid_fire_signals() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_RAPID_001";
    let slave_account = "SLAVE_RAPID_001";

    setup_test_scenario(&server, master_account, &[slave_account], |_| {
        default_test_slave_settings()
    })
    .await
    .unwrap();

    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .unwrap();

    let slave = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave_account,
    )
    .unwrap();

    slave.subscribe_to_master(master_account).unwrap();
    register_all_eas(&master, &[&slave]).await.unwrap();

    // Send 50 signals with minimal delay
    let signal_count = 50;
    for i in 1..=signal_count {
        let signal =
            master.create_open_signal(i, "EURUSD", OrderType::Buy, 0.1, 1.0850, None, None, 0);
        master.send_trade_signal(&signal).unwrap();
        // Minimal delay to prevent message batching issues
        if i % 10 == 0 {
            sleep(Duration::from_millis(50)).await;
        }
    }

    sleep(Duration::from_millis(500)).await;
    let signals = slave
        .collect_trade_signals(8000, signal_count as usize)
        .unwrap();

    assert_eq!(
        signals.len(),
        signal_count as usize,
        "Should receive all {} signals without loss",
        signal_count
    );

    println!(
        "✅ test_rapid_fire_signals passed ({} signals)",
        signal_count
    );

    server.shutdown().await;
}

// =============================================================================
// Category 3: Multiple Masters Tests
// =============================================================================

/// Test signal isolation between different masters
#[tokio::test]
async fn test_multi_master_signal_isolation() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master1_account = "MASTER_ISO_001";
    let master2_account = "MASTER_ISO_002";
    let slave1_account = "SLAVE_ISO_001";
    let slave2_account = "SLAVE_ISO_002";

    // Setup: Master1 -> Slave1, Master2 -> Slave2
    setup_test_scenario(&server, master1_account, &[slave1_account], |_| {
        default_test_slave_settings()
    })
    .await
    .unwrap();

    setup_test_scenario(&server, master2_account, &[slave2_account], |_| {
        default_test_slave_settings()
    })
    .await
    .unwrap();

    let master1 = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master1_account,
    )
    .unwrap();

    let master2 = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master2_account,
    )
    .unwrap();

    let slave1 = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave1_account,
    )
    .unwrap();

    let slave2 = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave2_account,
    )
    .unwrap();

    // Each slave subscribes to its own master
    slave1.subscribe_to_master(master1_account).unwrap();
    slave2.subscribe_to_master(master2_account).unwrap();

    // Register all
    master1.send_heartbeat().unwrap();
    master2.send_heartbeat().unwrap();
    slave1.send_heartbeat().unwrap();
    slave2.send_heartbeat().unwrap();
    sleep(Duration::from_millis(200)).await;

    // Master1 sends ticket 100
    let sig1 =
        master1.create_open_signal(100, "EURUSD", OrderType::Buy, 0.1, 1.0850, None, None, 0);
    master1.send_trade_signal(&sig1).unwrap();

    // Master2 sends ticket 200
    let sig2 =
        master2.create_open_signal(200, "GBPUSD", OrderType::Sell, 0.2, 1.2500, None, None, 0);
    master2.send_trade_signal(&sig2).unwrap();

    sleep(Duration::from_millis(100)).await;

    let signals1 = slave1.collect_trade_signals(2000, 2).unwrap();
    let signals2 = slave2.collect_trade_signals(2000, 2).unwrap();

    // Slave1 should only receive ticket 100 from Master1
    assert_eq!(signals1.len(), 1, "Slave1 should receive only 1 signal");
    assert_eq!(signals1[0].1.ticket, 100);
    assert_eq!(signals1[0].0, master1_account);

    // Slave2 should only receive ticket 200 from Master2
    assert_eq!(signals2.len(), 1, "Slave2 should receive only 1 signal");
    assert_eq!(signals2[0].1.ticket, 200);
    assert_eq!(signals2[0].0, master2_account);

    println!("✅ test_multi_master_signal_isolation passed");

    server.shutdown().await;
}

/// Test same symbol from different masters
#[tokio::test]
async fn test_multi_master_same_symbol_open() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master1_account = "MASTER_SAME_SYM_001";
    let master2_account = "MASTER_SAME_SYM_002";
    let slave1_account = "SLAVE_SAME_SYM_001";
    let slave2_account = "SLAVE_SAME_SYM_002";

    setup_test_scenario(&server, master1_account, &[slave1_account], |_| {
        default_test_slave_settings()
    })
    .await
    .unwrap();

    setup_test_scenario(&server, master2_account, &[slave2_account], |_| {
        default_test_slave_settings()
    })
    .await
    .unwrap();

    let master1 = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master1_account,
    )
    .unwrap();

    let master2 = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master2_account,
    )
    .unwrap();

    let slave1 = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave1_account,
    )
    .unwrap();

    let slave2 = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave2_account,
    )
    .unwrap();

    slave1.subscribe_to_master(master1_account).unwrap();
    slave2.subscribe_to_master(master2_account).unwrap();

    master1.send_heartbeat().unwrap();
    master2.send_heartbeat().unwrap();
    slave1.send_heartbeat().unwrap();
    slave2.send_heartbeat().unwrap();
    sleep(Duration::from_millis(200)).await;

    // Both masters send Open for EURUSD (same symbol)
    let sig1 =
        master1.create_open_signal(100, "EURUSD", OrderType::Buy, 0.1, 1.0850, None, None, 0);
    let sig2 =
        master2.create_open_signal(200, "EURUSD", OrderType::Sell, 0.2, 1.0850, None, None, 0);

    master1.send_trade_signal(&sig1).unwrap();
    master2.send_trade_signal(&sig2).unwrap();

    sleep(Duration::from_millis(100)).await;

    let signals1 = slave1.collect_trade_signals(2000, 2).unwrap();
    let signals2 = slave2.collect_trade_signals(2000, 2).unwrap();

    // Each slave receives only its master's signal (no cross-contamination)
    assert_eq!(signals1.len(), 1);
    assert_eq!(signals1[0].1.ticket, 100);
    assert!(matches!(signals1[0].1.order_type, Some(OrderType::Buy)));

    assert_eq!(signals2.len(), 1);
    assert_eq!(signals2[0].1.ticket, 200);
    assert!(matches!(signals2[0].1.order_type, Some(OrderType::Sell)));

    println!("✅ test_multi_master_same_symbol_open passed");

    server.shutdown().await;
}

// =============================================================================
// Category 4: Multiple Slaves Tests
// =============================================================================

/// Test signal broadcast to all slaves
#[tokio::test]
async fn test_signal_broadcast_to_all_slaves() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_BROADCAST_001";
    let slave1_account = "SLAVE_BROADCAST_001";
    let slave2_account = "SLAVE_BROADCAST_002";
    let slave3_account = "SLAVE_BROADCAST_003";

    // Setup: 1 Master -> 3 Slaves
    setup_test_scenario(
        &server,
        master_account,
        &[slave1_account, slave2_account, slave3_account],
        |_| default_test_slave_settings(),
    )
    .await
    .unwrap();

    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .unwrap();

    let slave1 = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave1_account,
    )
    .unwrap();

    let slave2 = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave2_account,
    )
    .unwrap();

    let slave3 = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave3_account,
    )
    .unwrap();

    // All slaves subscribe to the same master
    slave1.subscribe_to_master(master_account).unwrap();
    slave2.subscribe_to_master(master_account).unwrap();
    slave3.subscribe_to_master(master_account).unwrap();

    master.send_heartbeat().unwrap();
    slave1.send_heartbeat().unwrap();
    slave2.send_heartbeat().unwrap();
    slave3.send_heartbeat().unwrap();
    sleep(Duration::from_millis(200)).await;

    // Master sends one signal
    let signal =
        master.create_open_signal(12345, "EURUSD", OrderType::Buy, 0.1, 1.0850, None, None, 0);
    master.send_trade_signal(&signal).unwrap();

    sleep(Duration::from_millis(100)).await;

    let signals1 = slave1.collect_trade_signals(2000, 1).unwrap();
    let signals2 = slave2.collect_trade_signals(2000, 1).unwrap();
    let signals3 = slave3.collect_trade_signals(2000, 1).unwrap();

    // All 3 slaves should receive the signal
    assert_eq!(signals1.len(), 1, "Slave1 should receive signal");
    assert_eq!(signals2.len(), 1, "Slave2 should receive signal");
    assert_eq!(signals3.len(), 1, "Slave3 should receive signal");

    // All received the same ticket
    assert_eq!(signals1[0].1.ticket, 12345);
    assert_eq!(signals2[0].1.ticket, 12345);
    assert_eq!(signals3[0].1.ticket, 12345);

    println!("✅ test_signal_broadcast_to_all_slaves passed");

    server.shutdown().await;
}

/// Test lot multiplier application
/// Note: Tests single slave lot multiplier transformation
#[tokio::test]
async fn test_slave_individual_lot_multiplier() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_LOT_MULT_001";
    let slave_account = "SLAVE_LOT_MULT_001";

    // Setup with 2x lot multiplier
    setup_test_scenario(&server, master_account, &[slave_account], |_| {
        SlaveSettings {
            lot_calculation_mode: LotCalculationMode::Multiplier,
            lot_multiplier: Some(2.0),
            reverse_trade: false,
            symbol_prefix: None,
            symbol_suffix: None,
            symbol_mappings: vec![],
            filters: TradeFilters::default(),
            config_version: 0,
            source_lot_min: None,
            source_lot_max: None,
            sync_mode: SyncMode::Skip,
            limit_order_expiry_min: None,
            market_sync_max_pips: None,
            max_slippage: None,
            copy_pending_orders: false,
            // Trade Execution defaults
            max_retries: 3,
            max_signal_delay_ms: 5000,
            use_pending_order_for_delayed: false,
        }
    })
    .await
    .unwrap();

    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .unwrap();

    let slave = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave_account,
    )
    .unwrap();

    slave.subscribe_to_master(master_account).unwrap();
    register_all_eas(&master, &[&slave]).await.unwrap();

    // Master sends 0.1 lot
    let signal =
        master.create_open_signal(12345, "EURUSD", OrderType::Buy, 0.1, 1.0850, None, None, 0);
    master.send_trade_signal(&signal).unwrap();

    sleep(Duration::from_millis(200)).await;

    let signals = slave.collect_trade_signals(3000, 1).unwrap();

    assert_eq!(signals.len(), 1, "Should receive 1 signal");

    // Verify lot multiplier applied: 0.1 * 2.0 = 0.2
    let lots = signals[0].1.lots.expect("lots should be present");
    assert!(
        (lots - 0.2).abs() < 0.001,
        "Lots should be 0.2 (0.1 * 2.0), got {}",
        lots
    );

    println!("✅ test_slave_individual_lot_multiplier passed");

    server.shutdown().await;
}

// =============================================================================
// Category 6: Latency Tests
// =============================================================================

/// Test signal latency measurement
#[tokio::test]
async fn test_signal_latency_measurement() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_LATENCY_001";
    let slave_account = "SLAVE_LATENCY_001";

    setup_test_scenario(&server, master_account, &[slave_account], |_| {
        default_test_slave_settings()
    })
    .await
    .unwrap();

    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .unwrap();

    let slave = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave_account,
    )
    .unwrap();

    slave.subscribe_to_master(master_account).unwrap();
    register_all_eas(&master, &[&slave]).await.unwrap();

    // Measure latency for 10 signals
    let mut latencies = Vec::new();

    for i in 1..=10 {
        let send_time = std::time::Instant::now();
        let signal =
            master.create_open_signal(i, "EURUSD", OrderType::Buy, 0.1, 1.0850, None, None, 0);
        master.send_trade_signal(&signal).unwrap();

        if slave.try_receive_trade_signal(1000).unwrap().is_some() {
            let latency = send_time.elapsed();
            latencies.push(latency.as_millis() as f64);
        }

        sleep(Duration::from_millis(50)).await;
    }

    let avg_latency: f64 = latencies.iter().sum::<f64>() / latencies.len() as f64;
    let max_latency = latencies.iter().cloned().fold(0.0_f64, f64::max);

    println!(
        "Latency stats: avg={:.2}ms, max={:.2}ms, samples={}",
        avg_latency,
        max_latency,
        latencies.len()
    );

    // Assert reasonable latency (should be < 50ms in local test)
    assert!(
        avg_latency < 50.0,
        "Average latency {} ms exceeds 50ms threshold",
        avg_latency
    );

    println!("✅ test_signal_latency_measurement passed");

    server.shutdown().await;
}

/// Test delayed signal (100ms old)
#[tokio::test]
async fn test_delayed_signal_immediate() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_DELAY_IMM_001";
    let slave_account = "SLAVE_DELAY_IMM_001";

    setup_test_scenario(&server, master_account, &[slave_account], |_| {
        default_test_slave_settings()
    })
    .await
    .unwrap();

    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .unwrap();

    let slave = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave_account,
    )
    .unwrap();

    slave.subscribe_to_master(master_account).unwrap();
    register_all_eas(&master, &[&slave]).await.unwrap();

    // Create signal with 100ms old timestamp
    let signal =
        master.create_open_signal(12345, "EURUSD", OrderType::Buy, 0.1, 1.0850, None, None, 0);
    let delayed_signal = master.create_delayed_signal(signal, 100);
    master.send_trade_signal(&delayed_signal).unwrap();

    sleep(Duration::from_millis(200)).await;
    let signals = slave.collect_trade_signals(3000, 1).unwrap();

    // Should still be delivered
    assert_eq!(
        signals.len(),
        1,
        "Slightly delayed signal should be delivered"
    );

    println!("✅ test_delayed_signal_immediate passed");

    server.shutdown().await;
}

/// Test delayed signal (3 seconds old)
#[tokio::test]
async fn test_delayed_signal_acceptable() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_DELAY_ACC_001";
    let slave_account = "SLAVE_DELAY_ACC_001";

    setup_test_scenario(&server, master_account, &[slave_account], |_| {
        default_test_slave_settings()
    })
    .await
    .unwrap();

    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .unwrap();

    let slave = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave_account,
    )
    .unwrap();

    slave.subscribe_to_master(master_account).unwrap();
    register_all_eas(&master, &[&slave]).await.unwrap();

    // Create signal with 3 second old timestamp
    let signal =
        master.create_open_signal(12346, "EURUSD", OrderType::Buy, 0.1, 1.0850, None, None, 0);
    let delayed_signal = master.create_delayed_signal(signal, 3000);
    master.send_trade_signal(&delayed_signal).unwrap();

    sleep(Duration::from_millis(200)).await;
    let signals = slave.collect_trade_signals(3000, 1).unwrap();

    // Server doesn't filter by timestamp - should be delivered
    assert_eq!(
        signals.len(),
        1,
        "Server should deliver signal (filtering is EA's job)"
    );

    println!("✅ test_delayed_signal_acceptable passed");

    server.shutdown().await;
}

/// Test stale signal (10+ seconds old)
#[tokio::test]
async fn test_stale_signal_too_old() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_STALE_001";
    let slave_account = "SLAVE_STALE_001";

    setup_test_scenario(&server, master_account, &[slave_account], |_| {
        default_test_slave_settings()
    })
    .await
    .unwrap();

    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .unwrap();

    let slave = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave_account,
    )
    .unwrap();

    slave.subscribe_to_master(master_account).unwrap();
    register_all_eas(&master, &[&slave]).await.unwrap();

    // Create signal with 10 second old timestamp
    let signal =
        master.create_open_signal(12347, "EURUSD", OrderType::Buy, 0.1, 1.0850, None, None, 0);
    let stale_signal = master.create_delayed_signal(signal, 10000);
    master.send_trade_signal(&stale_signal).unwrap();

    sleep(Duration::from_millis(200)).await;
    let signals = slave.collect_trade_signals(3000, 1).unwrap();

    // Server delivers all signals - EA is responsible for timestamp validation
    assert_eq!(
        signals.len(),
        1,
        "Server delivers signal (EA validates timestamp)"
    );

    // Verify timestamp is indeed old
    let now = Utc::now();
    let signal_age = now - signals[0].1.timestamp;
    assert!(
        signal_age.num_seconds() >= 10,
        "Signal should have 10+ second old timestamp"
    );

    println!("✅ test_stale_signal_too_old passed");

    server.shutdown().await;
}

/// Test partial close signal with close_ratio
/// Verifies:
/// 1. close_ratio is preserved through the relay
/// 2. Lot multiplier is NOT applied to Close signals (only Open)
#[tokio::test]
async fn test_partial_close_signal() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_PARTIAL_001";
    let slave_account = "SLAVE_PARTIAL_001";

    // Set up slave with 2x lot multiplier to verify it's not applied to Close
    setup_test_scenario(&server, master_account, &[slave_account], |_| {
        let mut settings = default_test_slave_settings();
        settings.lot_multiplier = Some(2.0); // 2x multiplier
        settings
    })
    .await
    .unwrap();

    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .unwrap();

    let slave = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave_account,
    )
    .unwrap();

    slave.subscribe_to_master(master_account).unwrap();
    register_all_eas(&master, &[&slave]).await.unwrap();

    // Step 1: Open a position (lot multiplier should apply: 1.0 * 2.0 = 2.0)
    let open_signal =
        master.create_open_signal(12345, "EURUSD", OrderType::Buy, 1.0, 1.0850, None, None, 0);
    master.send_trade_signal(&open_signal).unwrap();

    sleep(Duration::from_millis(200)).await;
    let open_signals = slave.collect_trade_signals(3000, 1).unwrap();
    assert_eq!(open_signals.len(), 1, "Should receive Open signal");
    assert_eq!(
        open_signals[0].1.lots,
        Some(2.0),
        "Lot multiplier should be applied to Open: 1.0 * 2.0 = 2.0"
    );

    // Step 2: Partial close with 50% close_ratio
    // Note: Master's lots=1.0 (original), close_ratio=0.5 means 50% closed
    // Lot multiplier should NOT be applied to Close signal
    let partial_close_signal = master.create_partial_close_signal(12345, "EURUSD", 1.0, 0.5); // 50% partial close
    master.send_trade_signal(&partial_close_signal).unwrap();

    sleep(Duration::from_millis(200)).await;
    let close_signals = slave.collect_trade_signals(3000, 1).unwrap();
    assert_eq!(close_signals.len(), 1, "Should receive Close signal");

    let received_signal = &close_signals[0].1;
    assert_eq!(received_signal.action, TradeAction::Close);
    assert_eq!(
        received_signal.close_ratio,
        Some(0.5),
        "close_ratio should be preserved: 0.5"
    );
    assert_eq!(
        received_signal.lots,
        Some(1.0),
        "Lot multiplier should NOT be applied to Close signal"
    );

    println!("✅ test_partial_close_signal passed");

    server.shutdown().await;
}

/// Test full close signal (close_ratio = None)
/// Verifies backward compatibility - close without close_ratio works as full close
#[tokio::test]
async fn test_full_close_signal_no_ratio() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_FULLCLOSE_001";
    let slave_account = "SLAVE_FULLCLOSE_001";

    setup_test_scenario(&server, master_account, &[slave_account], |_| {
        default_test_slave_settings()
    })
    .await
    .unwrap();

    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .unwrap();

    let slave = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave_account,
    )
    .unwrap();

    slave.subscribe_to_master(master_account).unwrap();
    register_all_eas(&master, &[&slave]).await.unwrap();

    // Send full close signal (no close_ratio)
    let close_signal = master.create_close_signal(12346, "GBPUSD", 0.5);
    master.send_trade_signal(&close_signal).unwrap();

    sleep(Duration::from_millis(200)).await;
    let signals = slave.collect_trade_signals(3000, 1).unwrap();
    assert_eq!(signals.len(), 1, "Should receive Close signal");

    let received_signal = &signals[0].1;
    assert_eq!(received_signal.action, TradeAction::Close);
    assert_eq!(
        received_signal.close_ratio, None,
        "close_ratio should be None for full close"
    );

    println!("✅ test_full_close_signal_no_ratio passed");

    server.shutdown().await;
}

// =============================================================================
// Symbol Transformation Tests
// =============================================================================

/// Test symbol prefix/suffix transformation
/// Master sends "pro.EURUSD.m" -> Slave receives "fx.EURUSD"
#[tokio::test]
async fn test_symbol_prefix_suffix_transformation() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_SYMBOL_001";
    let slave_account = "SLAVE_SYMBOL_001";

    // Create trade group and update master settings with prefix/suffix
    server.db.create_trade_group(master_account).await.unwrap();
    let master_settings = MasterSettings {
        symbol_prefix: Some("pro.".to_string()),
        symbol_suffix: Some(".m".to_string()),
        config_version: 0,
    };
    server
        .db
        .update_master_settings(master_account, master_settings)
        .await
        .unwrap();

    let mut settings = default_test_slave_settings();
    settings.symbol_prefix = Some("fx.".to_string()); // Slave adds "fx." prefix
    settings.symbol_suffix = None;

    server
        .db
        .add_member(master_account, slave_account, settings)
        .await
        .unwrap();
    server
        .db
        .update_member_status(master_account, slave_account, STATUS_CONNECTED)
        .await
        .unwrap();

    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .unwrap();

    let slave = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave_account,
    )
    .unwrap();

    slave.subscribe_to_master(master_account).unwrap();
    register_all_eas(&master, &[&slave]).await.unwrap();

    // Master sends signal with prefixed/suffixed symbol
    let signal = master.create_open_signal(
        12345,
        "pro.EURUSD.m", // Master's symbol with prefix/suffix
        OrderType::Buy,
        0.1,
        1.0850,
        None,
        None,
        0,
    );
    master.send_trade_signal(&signal).unwrap();

    sleep(Duration::from_millis(200)).await;
    let signals = slave.collect_trade_signals(3000, 1).unwrap();
    assert_eq!(signals.len(), 1, "Should receive signal");

    // Slave should receive transformed symbol: "fx.EURUSD"
    assert_eq!(
        signals[0].1.symbol.as_deref(),
        Some("fx.EURUSD"),
        "Symbol should be transformed: pro.EURUSD.m -> fx.EURUSD"
    );

    println!("✅ test_symbol_prefix_suffix_transformation passed");

    server.shutdown().await;
}

/// Test that Master sends ALL orders regardless of prefix/suffix matching
/// This verifies the behavior change: Master no longer filters by prefix/suffix
/// - Orders with matching prefix/suffix: transformed (prefix/suffix stripped)
/// - Orders WITHOUT matching prefix/suffix: passed through as-is
#[tokio::test]
async fn test_master_sends_all_symbols_no_filtering() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_NO_FILTER_001";
    let slave_account = "SLAVE_NO_FILTER_001";

    // Create trade group with prefix/suffix settings on Master
    server.db.create_trade_group(master_account).await.unwrap();
    let master_settings = MasterSettings {
        symbol_prefix: Some("PRO.".to_string()), // Master configured with PRO. prefix
        symbol_suffix: Some(".m".to_string()),   // and .m suffix
        config_version: 0,
    };
    server
        .db
        .update_master_settings(master_account, master_settings)
        .await
        .unwrap();

    // Slave has no prefix/suffix (receives clean symbols)
    let mut settings = default_test_slave_settings();
    settings.symbol_prefix = None;
    settings.symbol_suffix = None;

    server
        .db
        .add_member(master_account, slave_account, settings)
        .await
        .unwrap();
    server
        .db
        .update_member_status(master_account, slave_account, STATUS_CONNECTED)
        .await
        .unwrap();

    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .unwrap();

    let slave = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave_account,
    )
    .unwrap();

    slave.subscribe_to_master(master_account).unwrap();
    register_all_eas(&master, &[&slave]).await.unwrap();

    // Test 1: Symbol WITH matching prefix/suffix - should be transformed
    let signal1 = master.create_open_signal(
        10001,
        "PRO.EURUSD.m", // Matches prefix/suffix
        OrderType::Buy,
        0.1,
        1.0850,
        None,
        None,
        0,
    );
    master.send_trade_signal(&signal1).unwrap();

    sleep(Duration::from_millis(200)).await;
    let signals1 = slave.collect_trade_signals(3000, 1).unwrap();
    assert_eq!(
        signals1.len(),
        1,
        "Symbol with prefix/suffix should be received"
    );
    assert_eq!(
        signals1[0].1.symbol.as_deref(),
        Some("EURUSD"),
        "PRO.EURUSD.m should be transformed to EURUSD"
    );

    // Test 2: Symbol WITHOUT prefix but with suffix - should NOT be transformed
    // (partial match is NOT transformed, passed as-is with suffix stripped if present)
    let signal2 = master.create_open_signal(
        10002,
        "USDJPY.m", // Only suffix matches, no prefix
        OrderType::Sell,
        0.2,
        150.0,
        None,
        None,
        0,
    );
    master.send_trade_signal(&signal2).unwrap();

    sleep(Duration::from_millis(200)).await;
    let signals2 = slave.collect_trade_signals(3000, 1).unwrap();
    assert_eq!(
        signals2.len(),
        1,
        "Symbol with only suffix should be received"
    );
    assert_eq!(
        signals2[0].1.symbol.as_deref(),
        Some("USDJPY"),
        "USDJPY.m should have suffix stripped to USDJPY"
    );

    // Test 3: Symbol with NO prefix/suffix match - should be passed through as-is
    let signal3 = master.create_open_signal(
        10003,
        "GBPUSD", // No prefix/suffix at all
        OrderType::Buy,
        0.15,
        1.2500,
        None,
        None,
        0,
    );
    master.send_trade_signal(&signal3).unwrap();

    sleep(Duration::from_millis(200)).await;
    let signals3 = slave.collect_trade_signals(3000, 1).unwrap();
    assert_eq!(
        signals3.len(),
        1,
        "Symbol without prefix/suffix should be received"
    );
    assert_eq!(
        signals3[0].1.symbol.as_deref(),
        Some("GBPUSD"),
        "GBPUSD should be passed through unchanged"
    );

    // Test 4: Different broker symbol format - should be passed through
    let signal4 = master.create_open_signal(
        10004,
        "XAUUSD#", // Different format (e.g., hashtag suffix)
        OrderType::Buy,
        0.5,
        2000.0,
        None,
        None,
        0,
    );
    master.send_trade_signal(&signal4).unwrap();

    sleep(Duration::from_millis(200)).await;
    let signals4 = slave.collect_trade_signals(3000, 1).unwrap();
    assert_eq!(
        signals4.len(),
        1,
        "Symbol with different format should be received"
    );
    assert_eq!(
        signals4[0].1.symbol.as_deref(),
        Some("XAUUSD#"),
        "XAUUSD# should be passed through unchanged (no matching prefix/suffix)"
    );

    println!("✅ test_master_sends_all_symbols_no_filtering passed");
    println!("  - PRO.EURUSD.m -> EURUSD (full match transformed)");
    println!("  - USDJPY.m -> USDJPY (suffix stripped)");
    println!("  - GBPUSD -> GBPUSD (no match, passed through)");
    println!("  - XAUUSD# -> XAUUSD# (different format, passed through)");

    server.shutdown().await;
}

/// Test symbol mapping (XAUUSD -> GOLD)
#[tokio::test]
async fn test_symbol_mapping() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_MAPPING_001";
    let slave_account = "SLAVE_MAPPING_001";

    setup_test_scenario(&server, master_account, &[slave_account], |_| {
        let mut settings = default_test_slave_settings();
        settings.symbol_mappings = vec![
            SymbolMapping {
                source_symbol: "XAUUSD".to_string(),
                target_symbol: "GOLD".to_string(),
            },
            SymbolMapping {
                source_symbol: "XAGUSD".to_string(),
                target_symbol: "SILVER".to_string(),
            },
        ];
        settings
    })
    .await
    .unwrap();

    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .unwrap();

    let slave = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave_account,
    )
    .unwrap();

    slave.subscribe_to_master(master_account).unwrap();
    register_all_eas(&master, &[&slave]).await.unwrap();

    // Send XAUUSD signal
    let signal =
        master.create_open_signal(12345, "XAUUSD", OrderType::Buy, 0.1, 2000.0, None, None, 0);
    master.send_trade_signal(&signal).unwrap();

    sleep(Duration::from_millis(200)).await;
    let signals = slave.collect_trade_signals(3000, 1).unwrap();
    assert_eq!(signals.len(), 1, "Should receive signal");

    assert_eq!(
        signals[0].1.symbol.as_deref(),
        Some("GOLD"),
        "XAUUSD should be mapped to GOLD"
    );

    println!("✅ test_symbol_mapping passed");

    server.shutdown().await;
}

// =============================================================================
// Trade Filter Tests
// =============================================================================

/// Test allowed symbols filter - only specified symbols are copied
#[tokio::test]
async fn test_allowed_symbols_filter() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_ALLOWED_001";
    let slave_account = "SLAVE_ALLOWED_001";

    setup_test_scenario(&server, master_account, &[slave_account], |_| {
        let mut settings = default_test_slave_settings();
        settings.filters = TradeFilters {
            allowed_symbols: Some(vec!["EURUSD".to_string(), "GBPUSD".to_string()]),
            blocked_symbols: None,
            allowed_magic_numbers: None,
            blocked_magic_numbers: None,
        };
        settings
    })
    .await
    .unwrap();

    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .unwrap();

    let slave = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave_account,
    )
    .unwrap();

    slave.subscribe_to_master(master_account).unwrap();
    register_all_eas(&master, &[&slave]).await.unwrap();

    // Send allowed symbol - should be received
    let signal1 =
        master.create_open_signal(12345, "EURUSD", OrderType::Buy, 0.1, 1.0850, None, None, 0);
    master.send_trade_signal(&signal1).unwrap();

    sleep(Duration::from_millis(200)).await;
    let signals1 = slave.collect_trade_signals(3000, 1).unwrap();
    assert_eq!(signals1.len(), 1, "EURUSD should be received (allowed)");

    // Send non-allowed symbol - should NOT be received
    let signal2 =
        master.create_open_signal(12346, "USDJPY", OrderType::Buy, 0.1, 150.0, None, None, 0);
    master.send_trade_signal(&signal2).unwrap();

    sleep(Duration::from_millis(200)).await;
    let signals2 = slave.collect_trade_signals(500, 1).unwrap();
    assert_eq!(
        signals2.len(),
        0,
        "USDJPY should NOT be received (not in allowed list)"
    );

    println!("✅ test_allowed_symbols_filter passed");

    server.shutdown().await;
}

/// Test blocked symbols filter - specified symbols are excluded
#[tokio::test]
async fn test_blocked_symbols_filter() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_BLOCKED_001";
    let slave_account = "SLAVE_BLOCKED_001";

    setup_test_scenario(&server, master_account, &[slave_account], |_| {
        let mut settings = default_test_slave_settings();
        settings.filters = TradeFilters {
            allowed_symbols: None,
            blocked_symbols: Some(vec!["XAUUSD".to_string(), "XAGUSD".to_string()]),
            allowed_magic_numbers: None,
            blocked_magic_numbers: None,
        };
        settings
    })
    .await
    .unwrap();

    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .unwrap();

    let slave = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave_account,
    )
    .unwrap();

    slave.subscribe_to_master(master_account).unwrap();
    register_all_eas(&master, &[&slave]).await.unwrap();

    // Send non-blocked symbol - should be received
    let signal1 =
        master.create_open_signal(12345, "EURUSD", OrderType::Buy, 0.1, 1.0850, None, None, 0);
    master.send_trade_signal(&signal1).unwrap();

    sleep(Duration::from_millis(200)).await;
    let signals1 = slave.collect_trade_signals(3000, 1).unwrap();
    assert_eq!(signals1.len(), 1, "EURUSD should be received (not blocked)");

    // Send blocked symbol - should NOT be received
    let signal2 =
        master.create_open_signal(12346, "XAUUSD", OrderType::Buy, 0.1, 2000.0, None, None, 0);
    master.send_trade_signal(&signal2).unwrap();

    sleep(Duration::from_millis(200)).await;
    let signals2 = slave.collect_trade_signals(500, 1).unwrap();
    assert_eq!(signals2.len(), 0, "XAUUSD should NOT be received (blocked)");

    println!("✅ test_blocked_symbols_filter passed");

    server.shutdown().await;
}

/// Test allowed magic numbers filter
#[tokio::test]
async fn test_allowed_magic_numbers_filter() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_MAGIC_ALLOW_001";
    let slave_account = "SLAVE_MAGIC_ALLOW_001";

    setup_test_scenario(&server, master_account, &[slave_account], |_| {
        let mut settings = default_test_slave_settings();
        settings.filters = TradeFilters {
            allowed_symbols: None,
            blocked_symbols: None,
            allowed_magic_numbers: Some(vec![12345, 67890]),
            blocked_magic_numbers: None,
        };
        settings
    })
    .await
    .unwrap();

    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .unwrap();

    let slave = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave_account,
    )
    .unwrap();

    slave.subscribe_to_master(master_account).unwrap();
    register_all_eas(&master, &[&slave]).await.unwrap();

    // Send allowed magic number - should be received
    let signal1 = master.create_open_signal(
        1001,
        "EURUSD",
        OrderType::Buy,
        0.1,
        1.0850,
        None,
        None,
        12345,
    );
    master.send_trade_signal(&signal1).unwrap();

    sleep(Duration::from_millis(200)).await;
    let signals1 = slave.collect_trade_signals(3000, 1).unwrap();
    assert_eq!(
        signals1.len(),
        1,
        "Magic 12345 should be received (allowed)"
    );

    // Send non-allowed magic number - should NOT be received
    let signal2 = master.create_open_signal(
        1002,
        "EURUSD",
        OrderType::Buy,
        0.1,
        1.0850,
        None,
        None,
        99999,
    );
    master.send_trade_signal(&signal2).unwrap();

    sleep(Duration::from_millis(200)).await;
    let signals2 = slave.collect_trade_signals(500, 1).unwrap();
    assert_eq!(
        signals2.len(),
        0,
        "Magic 99999 should NOT be received (not allowed)"
    );

    println!("✅ test_allowed_magic_numbers_filter passed");

    server.shutdown().await;
}

/// Test blocked magic numbers filter
#[tokio::test]
async fn test_blocked_magic_numbers_filter() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_MAGIC_BLOCK_001";
    let slave_account = "SLAVE_MAGIC_BLOCK_001";

    setup_test_scenario(&server, master_account, &[slave_account], |_| {
        let mut settings = default_test_slave_settings();
        settings.filters = TradeFilters {
            allowed_symbols: None,
            blocked_symbols: None,
            allowed_magic_numbers: None,
            blocked_magic_numbers: Some(vec![11111, 22222]),
        };
        settings
    })
    .await
    .unwrap();

    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .unwrap();

    let slave = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave_account,
    )
    .unwrap();

    slave.subscribe_to_master(master_account).unwrap();
    register_all_eas(&master, &[&slave]).await.unwrap();

    // Send non-blocked magic number - should be received
    let signal1 = master.create_open_signal(
        1001,
        "EURUSD",
        OrderType::Buy,
        0.1,
        1.0850,
        None,
        None,
        33333,
    );
    master.send_trade_signal(&signal1).unwrap();

    sleep(Duration::from_millis(200)).await;
    let signals1 = slave.collect_trade_signals(3000, 1).unwrap();
    assert_eq!(
        signals1.len(),
        1,
        "Magic 33333 should be received (not blocked)"
    );

    // Send blocked magic number - should NOT be received
    let signal2 = master.create_open_signal(
        1002,
        "EURUSD",
        OrderType::Buy,
        0.1,
        1.0850,
        None,
        None,
        11111,
    );
    master.send_trade_signal(&signal2).unwrap();

    sleep(Duration::from_millis(200)).await;
    let signals2 = slave.collect_trade_signals(500, 1).unwrap();
    assert_eq!(
        signals2.len(),
        0,
        "Magic 11111 should NOT be received (blocked)"
    );

    println!("✅ test_blocked_magic_numbers_filter passed");

    server.shutdown().await;
}

// =============================================================================
// Reverse Trade Tests
// =============================================================================

/// Test reverse trade mode - Buy becomes Sell
#[tokio::test]
async fn test_reverse_trade_buy_to_sell() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_REVERSE_001";
    let slave_account = "SLAVE_REVERSE_001";

    setup_test_scenario(&server, master_account, &[slave_account], |_| {
        let mut settings = default_test_slave_settings();
        settings.reverse_trade = true;
        settings
    })
    .await
    .unwrap();

    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .unwrap();

    let slave = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave_account,
    )
    .unwrap();

    slave.subscribe_to_master(master_account).unwrap();
    register_all_eas(&master, &[&slave]).await.unwrap();

    // Send Buy signal - should become Sell
    let signal =
        master.create_open_signal(12345, "EURUSD", OrderType::Buy, 0.1, 1.0850, None, None, 0);
    master.send_trade_signal(&signal).unwrap();

    sleep(Duration::from_millis(200)).await;
    let signals = slave.collect_trade_signals(3000, 1).unwrap();
    assert_eq!(signals.len(), 1, "Should receive signal");

    assert_eq!(
        signals[0].1.order_type,
        Some(OrderType::Sell),
        "Buy should be reversed to Sell"
    );

    println!("✅ test_reverse_trade_buy_to_sell passed");

    server.shutdown().await;
}

/// Test reverse trade mode with pending orders
#[tokio::test]
async fn test_reverse_trade_pending_orders() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_REVERSE_PEND_001";
    let slave_account = "SLAVE_REVERSE_PEND_001";

    setup_test_scenario(&server, master_account, &[slave_account], |_| {
        let mut settings = default_test_slave_settings();
        settings.reverse_trade = true;
        settings.copy_pending_orders = true;
        settings
    })
    .await
    .unwrap();

    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .unwrap();

    let slave = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave_account,
    )
    .unwrap();

    slave.subscribe_to_master(master_account).unwrap();
    register_all_eas(&master, &[&slave]).await.unwrap();

    // Send BuyLimit - should become SellLimit
    let signal = master.create_open_signal(
        12345,
        "EURUSD",
        OrderType::BuyLimit,
        0.1,
        1.0800,
        None,
        None,
        0,
    );
    master.send_trade_signal(&signal).unwrap();

    sleep(Duration::from_millis(200)).await;
    let signals = slave.collect_trade_signals(3000, 1).unwrap();
    assert_eq!(signals.len(), 1, "Should receive signal");

    assert_eq!(
        signals[0].1.order_type,
        Some(OrderType::SellLimit),
        "BuyLimit should be reversed to SellLimit"
    );

    println!("✅ test_reverse_trade_pending_orders passed");

    server.shutdown().await;
}

// =============================================================================
// Source Lot Limits Tests
// =============================================================================

/// Test source_lot_min filter - signals below minimum are excluded
#[tokio::test]
async fn test_source_lot_min_filter() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_LOTMIN_001";
    let slave_account = "SLAVE_LOTMIN_001";

    setup_test_scenario(&server, master_account, &[slave_account], |_| {
        let mut settings = default_test_slave_settings();
        settings.source_lot_min = Some(0.5); // Minimum 0.5 lots
        settings
    })
    .await
    .unwrap();

    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .unwrap();

    let slave = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave_account,
    )
    .unwrap();

    slave.subscribe_to_master(master_account).unwrap();
    register_all_eas(&master, &[&slave]).await.unwrap();

    // Send signal above minimum - should be received
    let signal1 =
        master.create_open_signal(12345, "EURUSD", OrderType::Buy, 1.0, 1.0850, None, None, 0);
    master.send_trade_signal(&signal1).unwrap();

    sleep(Duration::from_millis(200)).await;
    let signals1 = slave.collect_trade_signals(3000, 1).unwrap();
    assert_eq!(
        signals1.len(),
        1,
        "1.0 lots should be received (>= 0.5 min)"
    );

    // Send signal below minimum - should NOT be received
    let signal2 =
        master.create_open_signal(12346, "EURUSD", OrderType::Buy, 0.1, 1.0850, None, None, 0);
    master.send_trade_signal(&signal2).unwrap();

    sleep(Duration::from_millis(200)).await;
    let signals2 = slave.collect_trade_signals(500, 1).unwrap();
    assert_eq!(
        signals2.len(),
        0,
        "0.1 lots should NOT be received (< 0.5 min)"
    );

    println!("✅ test_source_lot_min_filter passed");

    server.shutdown().await;
}

/// Test source_lot_max filter - signals above maximum are excluded
#[tokio::test]
async fn test_source_lot_max_filter() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_LOTMAX_001";
    let slave_account = "SLAVE_LOTMAX_001";

    setup_test_scenario(&server, master_account, &[slave_account], |_| {
        let mut settings = default_test_slave_settings();
        settings.source_lot_max = Some(1.0); // Maximum 1.0 lots
        settings
    })
    .await
    .unwrap();

    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .unwrap();

    let slave = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave_account,
    )
    .unwrap();

    slave.subscribe_to_master(master_account).unwrap();
    register_all_eas(&master, &[&slave]).await.unwrap();

    // Send signal below maximum - should be received
    let signal1 =
        master.create_open_signal(12345, "EURUSD", OrderType::Buy, 0.5, 1.0850, None, None, 0);
    master.send_trade_signal(&signal1).unwrap();

    sleep(Duration::from_millis(200)).await;
    let signals1 = slave.collect_trade_signals(3000, 1).unwrap();
    assert_eq!(
        signals1.len(),
        1,
        "0.5 lots should be received (<= 1.0 max)"
    );

    // Send signal above maximum - should NOT be received
    let signal2 =
        master.create_open_signal(12346, "EURUSD", OrderType::Buy, 5.0, 1.0850, None, None, 0);
    master.send_trade_signal(&signal2).unwrap();

    sleep(Duration::from_millis(200)).await;
    let signals2 = slave.collect_trade_signals(500, 1).unwrap();
    assert_eq!(
        signals2.len(),
        0,
        "5.0 lots should NOT be received (> 1.0 max)"
    );

    println!("✅ test_source_lot_max_filter passed");

    server.shutdown().await;
}

// =============================================================================
// Multiple Partial Close Tests
// =============================================================================

/// Test multiple sequential partial closes
/// 1.0 lot -> 50% close -> 0.5 lot -> 50% close -> 0.25 lot
#[tokio::test]
async fn test_multiple_sequential_partial_closes() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_MULTI_PARTIAL_001";
    let slave_account = "SLAVE_MULTI_PARTIAL_001";

    setup_test_scenario(&server, master_account, &[slave_account], |_| {
        default_test_slave_settings()
    })
    .await
    .unwrap();

    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .unwrap();

    let slave = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave_account,
    )
    .unwrap();

    slave.subscribe_to_master(master_account).unwrap();
    register_all_eas(&master, &[&slave]).await.unwrap();

    // Open position with 1.0 lots
    let open_signal =
        master.create_open_signal(12345, "EURUSD", OrderType::Buy, 1.0, 1.0850, None, None, 0);
    master.send_trade_signal(&open_signal).unwrap();

    sleep(Duration::from_millis(200)).await;
    let signals1 = slave.collect_trade_signals(3000, 1).unwrap();
    assert_eq!(signals1.len(), 1, "Should receive Open signal");

    // First partial close: 50% (1.0 -> 0.5)
    let partial1 = master.create_partial_close_signal(12345, "EURUSD", 1.0, 0.5);
    master.send_trade_signal(&partial1).unwrap();

    sleep(Duration::from_millis(200)).await;
    let signals2 = slave.collect_trade_signals(3000, 1).unwrap();
    assert_eq!(signals2.len(), 1, "Should receive first partial close");
    assert_eq!(
        signals2[0].1.close_ratio,
        Some(0.5),
        "First close_ratio should be 0.5"
    );

    // Second partial close: 50% of remaining (0.5 -> 0.25)
    let partial2 = master.create_partial_close_signal(12345, "EURUSD", 0.5, 0.5);
    master.send_trade_signal(&partial2).unwrap();

    sleep(Duration::from_millis(200)).await;
    let signals3 = slave.collect_trade_signals(3000, 1).unwrap();
    assert_eq!(signals3.len(), 1, "Should receive second partial close");
    assert_eq!(
        signals3[0].1.close_ratio,
        Some(0.5),
        "Second close_ratio should be 0.5"
    );

    println!("✅ test_multiple_sequential_partial_closes passed");

    server.shutdown().await;
}

// =============================================================================
// Disabled Slave Tests
// =============================================================================

/// Test that both enabled and disabled slaves receive signals
/// (Filtering for disabled slaves happens on Slave EA side, not relay-server)
#[tokio::test]
async fn test_disabled_slave_receives_signals() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_DISABLED_001";
    let slave_enabled = "SLAVE_ENABLED_001";
    let slave_disabled = "SLAVE_DISABLED_001";

    // Create trade group
    server.db.create_trade_group(master_account).await.unwrap();

    // Add enabled slave (status = CONNECTED)
    server
        .db
        .add_member(master_account, slave_enabled, default_test_slave_settings())
        .await
        .unwrap();
    server
        .db
        .update_member_status(master_account, slave_enabled, STATUS_CONNECTED)
        .await
        .unwrap();

    // Add disabled slave (status = 0)
    server
        .db
        .add_member(
            master_account,
            slave_disabled,
            default_test_slave_settings(),
        )
        .await
        .unwrap();
    server
        .db
        .update_member_status(master_account, slave_disabled, 0) // STATUS_DISABLED
        .await
        .unwrap();

    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .unwrap();

    let slave1 = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave_enabled,
    )
    .unwrap();

    let slave2 = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave_disabled,
    )
    .unwrap();

    slave1.subscribe_to_master(master_account).unwrap();
    slave2.subscribe_to_master(master_account).unwrap();
    register_all_eas(&master, &[&slave1, &slave2])
        .await
        .unwrap();

    // Send signal
    let signal =
        master.create_open_signal(12345, "EURUSD", OrderType::Buy, 0.1, 1.0850, None, None, 0);
    master.send_trade_signal(&signal).unwrap();

    sleep(Duration::from_millis(300)).await;

    // Enabled slave should receive
    let signals1 = slave1.collect_trade_signals(3000, 1).unwrap();
    assert_eq!(signals1.len(), 1, "Enabled slave should receive signal");

    // Disabled slave ALSO receives signal (filtering happens on Slave EA side)
    let signals2 = slave2.collect_trade_signals(3000, 1).unwrap();
    assert_eq!(
        signals2.len(),
        1,
        "Disabled slave should also receive signal (EA-side filtering)"
    );

    println!("✅ test_disabled_slave_receives_signals passed");

    server.shutdown().await;
}

// =============================================================================
// Pending Order Tests
// =============================================================================

/// Test pending order types (BuyLimit, SellLimit, BuyStop, SellStop)
#[tokio::test]
async fn test_pending_order_types() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_PENDING_001";
    let slave_account = "SLAVE_PENDING_001";

    setup_test_scenario(&server, master_account, &[slave_account], |_| {
        let mut settings = default_test_slave_settings();
        settings.copy_pending_orders = true;
        settings
    })
    .await
    .unwrap();

    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .unwrap();

    let slave = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave_account,
    )
    .unwrap();

    slave.subscribe_to_master(master_account).unwrap();
    register_all_eas(&master, &[&slave]).await.unwrap();

    // Test BuyLimit
    let buy_limit = master.create_open_signal(
        1001,
        "EURUSD",
        OrderType::BuyLimit,
        0.1,
        1.0800,
        Some(1.0750),
        Some(1.0900),
        0,
    );
    master.send_trade_signal(&buy_limit).unwrap();

    sleep(Duration::from_millis(200)).await;
    let signals1 = slave.collect_trade_signals(3000, 1).unwrap();
    assert_eq!(signals1.len(), 1, "BuyLimit should be received");
    assert_eq!(signals1[0].1.order_type, Some(OrderType::BuyLimit));

    // Test SellLimit
    let sell_limit = master.create_open_signal(
        1002,
        "EURUSD",
        OrderType::SellLimit,
        0.1,
        1.0900,
        Some(1.0950),
        Some(1.0800),
        0,
    );
    master.send_trade_signal(&sell_limit).unwrap();

    sleep(Duration::from_millis(200)).await;
    let signals2 = slave.collect_trade_signals(3000, 1).unwrap();
    assert_eq!(signals2.len(), 1, "SellLimit should be received");
    assert_eq!(signals2[0].1.order_type, Some(OrderType::SellLimit));

    // Test BuyStop
    let buy_stop = master.create_open_signal(
        1003,
        "EURUSD",
        OrderType::BuyStop,
        0.1,
        1.0900,
        Some(1.0850),
        Some(1.1000),
        0,
    );
    master.send_trade_signal(&buy_stop).unwrap();

    sleep(Duration::from_millis(200)).await;
    let signals3 = slave.collect_trade_signals(3000, 1).unwrap();
    assert_eq!(signals3.len(), 1, "BuyStop should be received");
    assert_eq!(signals3[0].1.order_type, Some(OrderType::BuyStop));

    // Test SellStop
    let sell_stop = master.create_open_signal(
        1004,
        "EURUSD",
        OrderType::SellStop,
        0.1,
        1.0800,
        Some(1.0850),
        Some(1.0700),
        0,
    );
    master.send_trade_signal(&sell_stop).unwrap();

    sleep(Duration::from_millis(200)).await;
    let signals4 = slave.collect_trade_signals(3000, 1).unwrap();
    assert_eq!(signals4.len(), 1, "SellStop should be received");
    assert_eq!(signals4[0].1.order_type, Some(OrderType::SellStop));

    println!("✅ test_pending_order_types passed");

    server.shutdown().await;
}

/// Test copy_pending_orders = false - pending orders should not be copied
#[tokio::test]
async fn test_pending_orders_disabled() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_NO_PENDING_001";
    let slave_account = "SLAVE_NO_PENDING_001";

    setup_test_scenario(&server, master_account, &[slave_account], |_| {
        let mut settings = default_test_slave_settings();
        settings.copy_pending_orders = false; // Disabled
        settings
    })
    .await
    .unwrap();

    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .unwrap();

    let slave = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        &server.zmq_pub_trade_address(),
        slave_account,
    )
    .unwrap();

    slave.subscribe_to_master(master_account).unwrap();
    register_all_eas(&master, &[&slave]).await.unwrap();

    // Market order should be received
    let market_order =
        master.create_open_signal(1001, "EURUSD", OrderType::Buy, 0.1, 1.0850, None, None, 0);
    master.send_trade_signal(&market_order).unwrap();

    sleep(Duration::from_millis(200)).await;
    let signals1 = slave.collect_trade_signals(3000, 1).unwrap();
    assert_eq!(signals1.len(), 1, "Market order (Buy) should be received");

    // Pending order should NOT be received
    let pending_order = master.create_open_signal(
        1002,
        "EURUSD",
        OrderType::BuyLimit,
        0.1,
        1.0800,
        None,
        None,
        0,
    );
    master.send_trade_signal(&pending_order).unwrap();

    sleep(Duration::from_millis(200)).await;
    let signals2 = slave.collect_trade_signals(500, 1).unwrap();
    assert_eq!(
        signals2.len(),
        0,
        "Pending order (BuyLimit) should NOT be received when copy_pending_orders=false"
    );

    println!("✅ test_pending_orders_disabled passed");

    server.shutdown().await;
}
