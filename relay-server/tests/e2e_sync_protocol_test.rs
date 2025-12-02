// relay-server/tests/e2e_sync_protocol_test.rs
//
// E2E integration tests for Position Sync Protocol.
// Tests the complete flow of position synchronization:
// - Master EA sends PositionSnapshot after restart
// - Slave EA sends SyncRequest when needing sync
// - Relay Server routes messages appropriately
//
// Categories covered:
// 1. PositionSnapshot distribution (Master → Slaves)
// 2. SyncRequest routing (Slave → Master)
// 3. Full sync cycle (Slave request → Master response)

mod test_server;

use chrono::Utc;
use sankey_copier_relay_server::models::{LotCalculationMode, SlaveSettings, SyncMode};
use sankey_copier_zmq::ffi::{
    zmq_context_create, zmq_context_destroy, zmq_socket_connect, zmq_socket_create,
    zmq_socket_destroy, zmq_socket_receive, zmq_socket_send_binary, zmq_socket_subscribe, ZMQ_PUSH,
    ZMQ_SUB,
};
use sankey_copier_zmq::{
    HeartbeatMessage, PositionInfo, PositionSnapshotMessage, SyncRequestMessage, TradeFilters,
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
// Master EA Simulator (with Sync support)
// =============================================================================

/// Master EA Simulator with Position Sync support
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
        let config_topic = format!("config/{}", account_id);
        let topic_utf16: Vec<u16> = config_topic.encode_utf16().chain(Some(0)).collect();

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

    /// Send PositionSnapshot message
    fn send_position_snapshot(&self, positions: Vec<PositionInfo>) -> anyhow::Result<()> {
        let msg = PositionSnapshotMessage {
            message_type: "PositionSnapshot".to_string(),
            source_account: self.account_id.clone(),
            positions,
            timestamp: Utc::now().to_rfc3339(),
        };

        let bytes = rmp_serde::to_vec_named(&msg)?;
        unsafe {
            if zmq_socket_send_binary(self.push_socket_handle, bytes.as_ptr(), bytes.len() as i32)
                != 1
            {
                anyhow::bail!("Failed to send PositionSnapshot");
            }
        }
        Ok(())
    }

    /// Create a test position
    fn create_test_position(
        ticket: i64,
        symbol: &str,
        order_type: &str,
        lots: f64,
        open_price: f64,
    ) -> PositionInfo {
        PositionInfo {
            ticket,
            symbol: symbol.to_string(),
            order_type: order_type.to_string(),
            lots,
            open_price,
            open_time: Utc::now().to_rfc3339(),
            stop_loss: None,
            take_profit: None,
            magic_number: None,
            comment: None,
        }
    }

    /// Try to receive a SyncRequest with timeout
    fn try_receive_sync_request(
        &self,
        timeout_ms: i32,
    ) -> anyhow::Result<Option<SyncRequestMessage>> {
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

                // Parse topic + space + payload format
                let space_pos = bytes
                    .iter()
                    .position(|&b| b == b' ')
                    .ok_or_else(|| anyhow::anyhow!("Invalid message format: no space separator"))?;

                let payload = &bytes[space_pos + 1..];

                // Try to parse as SyncRequest
                match rmp_serde::from_slice::<SyncRequestMessage>(payload) {
                    Ok(req) if req.message_type == "SyncRequest" => {
                        return Ok(Some(req));
                    }
                    _ => {
                        // Not a SyncRequest, continue waiting
                        if start.elapsed() >= timeout_duration {
                            return Ok(None);
                        }
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                }
            } else if received_bytes == 0 {
                if start.elapsed() >= timeout_duration {
                    return Ok(None);
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            } else {
                return Err(anyhow::anyhow!("Failed to receive message"));
            }
        }
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
// Slave EA Simulator (with Sync support)
// =============================================================================

/// Slave EA Simulator with Position Sync support
struct SlaveEaSimulator {
    context_handle: i32,
    push_socket_handle: i32,
    config_socket_handle: i32,
    account_id: String,
    master_account: String,
}

impl SlaveEaSimulator {
    /// Create a new Slave EA simulator
    fn new(
        push_address: &str,
        config_address: &str,
        account_id: &str,
        master_account: &str,
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
            anyhow::bail!("Failed to create SUB socket");
        }

        let push_addr_utf16: Vec<u16> = push_address.encode_utf16().chain(Some(0)).collect();
        let config_addr_utf16: Vec<u16> = config_address.encode_utf16().chain(Some(0)).collect();
        let config_topic = format!("config/{}", account_id);
        let topic_utf16: Vec<u16> = config_topic.encode_utf16().chain(Some(0)).collect();

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

            // Subscribe to config messages for this slave account
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
            master_account: master_account.to_string(),
        })
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

    /// Send SyncRequest message
    fn send_sync_request(&self, last_sync_time: Option<String>) -> anyhow::Result<()> {
        let msg = SyncRequestMessage {
            message_type: "SyncRequest".to_string(),
            slave_account: self.account_id.clone(),
            master_account: self.master_account.clone(),
            last_sync_time,
            timestamp: Utc::now().to_rfc3339(),
        };

        let bytes = rmp_serde::to_vec_named(&msg)?;
        unsafe {
            if zmq_socket_send_binary(self.push_socket_handle, bytes.as_ptr(), bytes.len() as i32)
                != 1
            {
                anyhow::bail!("Failed to send SyncRequest");
            }
        }
        Ok(())
    }

    /// Try to receive a PositionSnapshot with timeout
    fn try_receive_position_snapshot(
        &self,
        timeout_ms: i32,
    ) -> anyhow::Result<Option<PositionSnapshotMessage>> {
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

                // Parse topic + space + payload format
                let space_pos = bytes
                    .iter()
                    .position(|&b| b == b' ')
                    .ok_or_else(|| anyhow::anyhow!("Invalid message format: no space separator"))?;

                let payload = &bytes[space_pos + 1..];

                // Try to parse as PositionSnapshot
                match rmp_serde::from_slice::<PositionSnapshotMessage>(payload) {
                    Ok(snapshot) if snapshot.message_type == "PositionSnapshot" => {
                        return Ok(Some(snapshot));
                    }
                    _ => {
                        // Not a PositionSnapshot, continue waiting
                        if start.elapsed() >= timeout_duration {
                            return Ok(None);
                        }
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                }
            } else if received_bytes == 0 {
                if start.elapsed() >= timeout_duration {
                    return Ok(None);
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            } else {
                return Err(anyhow::anyhow!("Failed to receive message"));
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

async fn set_member_status(
    server: &TestServer,
    master_account: &str,
    slave_account: &str,
    status: i32,
) -> anyhow::Result<()> {
    server
        .db
        .update_member_enabled_flag(master_account, slave_account, status > 0)
        .await?;
    server
        .db
        .update_member_runtime_status(master_account, slave_account, status)
        .await?;
    Ok(())
}

/// Setup test scenario with master and slaves
async fn setup_test_scenario(
    server: &TestServer,
    master_account: &str,
    slave_accounts: &[&str],
) -> anyhow::Result<()> {
    // Create trade group for master
    server.db.create_trade_group(master_account).await?;

    // Add slaves with default settings
    for slave_account in slave_accounts {
        let settings = default_test_slave_settings();
        server
            .db
            .add_member(master_account, slave_account, settings, 0)
            .await?;

        // Enable slave (set status to CONNECTED for trade copying)
        set_member_status(server, master_account, slave_account, STATUS_CONNECTED).await?;
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
    // Wait for connections to establish (slow joiner problem)
    sleep(Duration::from_millis(500)).await;
    Ok(())
}

// =============================================================================
// Category 1: PositionSnapshot Distribution Tests
// =============================================================================

/// Test: Master sends PositionSnapshot → Single Slave receives it
#[tokio::test]
async fn test_position_snapshot_single_slave() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_SYNC_001";
    let slave_account = "SLAVE_SYNC_001";

    // Setup scenario
    setup_test_scenario(&server, master_account, &[slave_account])
        .await
        .expect("Failed to setup scenario");

    // Create simulators
    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .expect("Failed to create master");

    let slave = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        slave_account,
        master_account,
    )
    .expect("Failed to create slave");

    // Register EAs
    register_all_eas(&master, &[&slave])
        .await
        .expect("Failed to register EAs");

    // Master sends PositionSnapshot with test positions
    let positions = vec![
        MasterEaSimulator::create_test_position(1001, "EURUSD", "Buy", 0.5, 1.0850),
        MasterEaSimulator::create_test_position(1002, "GBPUSD", "Sell", 0.3, 1.2650),
    ];

    master
        .send_position_snapshot(positions)
        .expect("Failed to send snapshot");

    // Give time for message routing
    sleep(Duration::from_millis(200)).await;

    // Slave should receive the PositionSnapshot
    let received = slave
        .try_receive_position_snapshot(1000)
        .expect("Failed to receive");

    assert!(received.is_some(), "Slave should receive PositionSnapshot");

    let snapshot = received.unwrap();
    assert_eq!(snapshot.source_account, master_account);
    assert_eq!(snapshot.positions.len(), 2);
    assert_eq!(snapshot.positions[0].ticket, 1001);
    assert_eq!(snapshot.positions[0].symbol, "EURUSD");
    assert_eq!(snapshot.positions[1].ticket, 1002);
    assert_eq!(snapshot.positions[1].symbol, "GBPUSD");
}

/// Test: Master sends PositionSnapshot → Multiple Slaves receive it
#[tokio::test]
async fn test_position_snapshot_multiple_slaves() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_SYNC_002";
    let slave_accounts = ["SLAVE_SYNC_002A", "SLAVE_SYNC_002B"];

    // Setup scenario
    setup_test_scenario(&server, master_account, &slave_accounts)
        .await
        .expect("Failed to setup scenario");

    // Create simulators
    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .expect("Failed to create master");

    let slave1 = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        slave_accounts[0],
        master_account,
    )
    .expect("Failed to create slave1");

    let slave2 = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        slave_accounts[1],
        master_account,
    )
    .expect("Failed to create slave2");

    // Register EAs
    register_all_eas(&master, &[&slave1, &slave2])
        .await
        .expect("Failed to register EAs");

    // Master sends PositionSnapshot
    let positions = vec![MasterEaSimulator::create_test_position(
        2001, "USDJPY", "Buy", 1.0, 149.50,
    )];

    master
        .send_position_snapshot(positions)
        .expect("Failed to send snapshot");

    // Give time for message routing
    sleep(Duration::from_millis(200)).await;

    // Both slaves should receive the PositionSnapshot
    let received1 = slave1
        .try_receive_position_snapshot(1000)
        .expect("Failed to receive on slave1");

    let received2 = slave2
        .try_receive_position_snapshot(1000)
        .expect("Failed to receive on slave2");

    assert!(
        received1.is_some(),
        "Slave1 should receive PositionSnapshot"
    );
    assert!(
        received2.is_some(),
        "Slave2 should receive PositionSnapshot"
    );

    let snapshot1 = received1.unwrap();
    let snapshot2 = received2.unwrap();

    assert_eq!(snapshot1.source_account, master_account);
    assert_eq!(snapshot2.source_account, master_account);
    assert_eq!(snapshot1.positions.len(), 1);
    assert_eq!(snapshot2.positions.len(), 1);
}

/// Test: Empty PositionSnapshot (Master has no positions)
#[tokio::test]
async fn test_position_snapshot_empty() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_SYNC_003";
    let slave_account = "SLAVE_SYNC_003";

    // Setup scenario
    setup_test_scenario(&server, master_account, &[slave_account])
        .await
        .expect("Failed to setup scenario");

    // Create simulators
    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .expect("Failed to create master");

    let slave = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        slave_account,
        master_account,
    )
    .expect("Failed to create slave");

    // Register EAs
    register_all_eas(&master, &[&slave])
        .await
        .expect("Failed to register EAs");

    // Master sends empty PositionSnapshot
    master
        .send_position_snapshot(vec![])
        .expect("Failed to send empty snapshot");

    // Give time for message routing
    sleep(Duration::from_millis(200)).await;

    // Slave should receive the empty snapshot
    let received = slave
        .try_receive_position_snapshot(1000)
        .expect("Failed to receive");

    assert!(received.is_some(), "Slave should receive empty snapshot");

    let snapshot = received.unwrap();
    assert_eq!(snapshot.positions.len(), 0);
}

// =============================================================================
// Category 2: SyncRequest Routing Tests
// =============================================================================

/// Test: Slave sends SyncRequest → Master receives it
#[tokio::test]
async fn test_sync_request_to_master() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_SYNC_004";
    let slave_account = "SLAVE_SYNC_004";

    // Setup scenario
    setup_test_scenario(&server, master_account, &[slave_account])
        .await
        .expect("Failed to setup scenario");

    // Create simulators
    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .expect("Failed to create master");

    let slave = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        slave_account,
        master_account,
    )
    .expect("Failed to create slave");

    // Register EAs
    register_all_eas(&master, &[&slave])
        .await
        .expect("Failed to register EAs");

    // Slave sends SyncRequest
    slave
        .send_sync_request(None)
        .expect("Failed to send SyncRequest");

    // Give time for message routing
    sleep(Duration::from_millis(200)).await;

    // Master should receive the SyncRequest
    let received = master
        .try_receive_sync_request(1000)
        .expect("Failed to receive");

    assert!(received.is_some(), "Master should receive SyncRequest");

    let request = received.unwrap();
    assert_eq!(request.slave_account, slave_account);
    assert_eq!(request.master_account, master_account);
    assert!(request.last_sync_time.is_none());
}

/// Test: SyncRequest with last_sync_time
#[tokio::test]
async fn test_sync_request_with_last_sync_time() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_SYNC_005";
    let slave_account = "SLAVE_SYNC_005";

    // Setup scenario
    setup_test_scenario(&server, master_account, &[slave_account])
        .await
        .expect("Failed to setup scenario");

    // Create simulators
    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .expect("Failed to create master");

    let slave = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        slave_account,
        master_account,
    )
    .expect("Failed to create slave");

    // Register EAs
    register_all_eas(&master, &[&slave])
        .await
        .expect("Failed to register EAs");

    // Slave sends SyncRequest with last_sync_time
    let last_sync = Utc::now().to_rfc3339();
    slave
        .send_sync_request(Some(last_sync.clone()))
        .expect("Failed to send SyncRequest");

    // Give time for message routing
    sleep(Duration::from_millis(200)).await;

    // Master should receive the SyncRequest
    let received = master
        .try_receive_sync_request(1000)
        .expect("Failed to receive");

    assert!(received.is_some(), "Master should receive SyncRequest");

    let request = received.unwrap();
    assert!(request.last_sync_time.is_some());
    assert_eq!(request.last_sync_time.unwrap(), last_sync);
}

/// Test: SyncRequest from non-member slave should be rejected
#[tokio::test]
async fn test_sync_request_non_member_rejected() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_SYNC_006";
    let slave_account = "SLAVE_SYNC_006";
    let non_member_slave = "NON_MEMBER_SLAVE";

    // Setup scenario - only add slave_account as member
    setup_test_scenario(&server, master_account, &[slave_account])
        .await
        .expect("Failed to setup scenario");

    // Create simulators
    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .expect("Failed to create master");

    // Create non-member slave (not registered in trade group)
    let non_member = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        non_member_slave,
        master_account,
    )
    .expect("Failed to create non-member slave");

    // Register EAs
    master.send_heartbeat().expect("Failed to send heartbeat");
    non_member
        .send_heartbeat()
        .expect("Failed to send heartbeat");
    sleep(Duration::from_millis(500)).await;

    // Non-member sends SyncRequest
    non_member
        .send_sync_request(None)
        .expect("Failed to send SyncRequest");

    // Give time for message processing
    sleep(Duration::from_millis(200)).await;

    // Master should NOT receive the SyncRequest (rejected by relay)
    let received = master
        .try_receive_sync_request(500)
        .expect("Failed to receive");

    assert!(
        received.is_none(),
        "Non-member SyncRequest should be rejected"
    );
}

// =============================================================================
// Category 3: Full Sync Cycle Tests
// =============================================================================

/// Test: Full sync cycle - Slave requests → Master responds with snapshot
#[tokio::test]
async fn test_full_sync_cycle() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_SYNC_007";
    let slave_account = "SLAVE_SYNC_007";

    // Setup scenario
    setup_test_scenario(&server, master_account, &[slave_account])
        .await
        .expect("Failed to setup scenario");

    // Create simulators
    let master = MasterEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        master_account,
    )
    .expect("Failed to create master");

    let slave = SlaveEaSimulator::new(
        &server.zmq_pull_address(),
        &server.zmq_pub_config_address(),
        slave_account,
        master_account,
    )
    .expect("Failed to create slave");

    // Register EAs
    register_all_eas(&master, &[&slave])
        .await
        .expect("Failed to register EAs");

    // Step 1: Slave sends SyncRequest
    slave
        .send_sync_request(None)
        .expect("Failed to send SyncRequest");

    // Give time for message routing
    sleep(Duration::from_millis(200)).await;

    // Step 2: Master receives SyncRequest
    let request = master
        .try_receive_sync_request(1000)
        .expect("Failed to receive SyncRequest")
        .expect("Should receive SyncRequest");

    assert_eq!(request.slave_account, slave_account);

    // Step 3: Master responds with PositionSnapshot
    let positions = vec![
        MasterEaSimulator::create_test_position(7001, "EURUSD", "Buy", 0.5, 1.0850),
        MasterEaSimulator::create_test_position(7002, "AUDUSD", "Sell", 0.2, 0.6520),
    ];

    master
        .send_position_snapshot(positions)
        .expect("Failed to send snapshot");

    // Give time for message routing
    sleep(Duration::from_millis(200)).await;

    // Step 4: Slave receives PositionSnapshot
    let snapshot = slave
        .try_receive_position_snapshot(1000)
        .expect("Failed to receive snapshot")
        .expect("Should receive PositionSnapshot");

    assert_eq!(snapshot.source_account, master_account);
    assert_eq!(snapshot.positions.len(), 2);

    // Verify position details
    assert_eq!(snapshot.positions[0].ticket, 7001);
    assert_eq!(snapshot.positions[0].symbol, "EURUSD");
    assert_eq!(snapshot.positions[0].order_type, "Buy");
    assert!((snapshot.positions[0].lots - 0.5).abs() < 0.0001);

    assert_eq!(snapshot.positions[1].ticket, 7002);
    assert_eq!(snapshot.positions[1].symbol, "AUDUSD");
    assert_eq!(snapshot.positions[1].order_type, "Sell");
}
