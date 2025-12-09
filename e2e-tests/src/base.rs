// e2e-tests/src/base.rs
//
// EaSimulatorBase - Common ZMQ connection management for EA Simulators.
//
// This module provides:
// - ZMQ context and socket creation/connection
// - Topic subscription
// - Raw message send/receive primitives
//
// The actual OnTimer loop logic is implemented in slave.rs and master.rs,
// matching the MQL5 EA implementation exactly.

use anyhow::Result;
use chrono::Utc;
use sankey_copier_zmq::ffi::{
    build_config_topic, zmq_context_create, zmq_context_destroy, zmq_socket_connect,
    zmq_socket_create, zmq_socket_destroy, zmq_socket_receive, zmq_socket_send_binary,
    zmq_socket_subscribe, ZMQ_PUSH, ZMQ_SUB,
};
use sankey_copier_zmq::HeartbeatMessage;
use std::ffi::c_char;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::types::{EaType, HeartbeatParams, BUFFER_SIZE, TOPIC_BUFFER_SIZE};

/// Base structure for EA Simulator ZMQ connection management.
///
/// Manages ZMQ context and sockets. The OnTimer loop is implemented
/// separately in MasterEaSimulator and SlaveEaSimulator to match
/// the MQL5 EA implementation exactly.
///
/// ## MQL5 Socket Architecture (SankeyCopierSlave.mq5)
/// - `g_zmq_socket` (PUSH): Send heartbeat, trade signals to server
/// - `g_zmq_config_socket` (SUB): Receive config/vlogs_config/PositionSnapshot
/// - `g_zmq_trade_socket` (SUB): Receive trade signals (Slave only)
///
/// Note: Master EA only has PUSH + config_socket (no trade_socket).
pub struct EaSimulatorBase {
    context_handle: i32,
    /// PUSH socket for sending messages to server (heartbeat, trade signals, etc.)
    /// MQL5: g_zmq_socket
    pub(crate) push_socket_handle: i32,
    /// SUB socket for receiving config messages from server
    /// MQL5: g_zmq_config_socket - receives config/{account_id}, config/global (vlogs), PositionSnapshot
    pub(crate) config_socket_handle: i32,
    /// SUB socket for receiving trade signals (Slave only)
    /// MQL5: g_zmq_trade_socket - receives trade/{master}/{slave}
    /// This is None for Master EA (Master doesn't receive trade signals)
    pub(crate) trade_socket_handle: Option<i32>,
    account_id: String,
    pub(crate) ea_type: EaType,
    /// Auto-trading state (simulates TERMINAL_TRADE_ALLOWED in MQL5)
    /// MQL5: g_last_trade_allowed initialized to false
    is_trade_allowed: Arc<AtomicBool>,
    /// Flag to signal background thread to stop
    pub(crate) shutdown_flag: Arc<AtomicBool>,
    /// Heartbeat parameters (balance, equity, version, etc.)
    pub(crate) heartbeat_params: HeartbeatParams,
}

impl EaSimulatorBase {
    /// Create a new EA simulator base with ZMQ connections.
    ///
    /// This sets up the ZMQ context and sockets, subscribes to the config topic,
    /// but does NOT start any background threads. Call start() on the derived
    /// simulator (MasterEaSimulator or SlaveEaSimulator) to begin the OnTimer loop.
    ///
    /// ## Parameters
    /// - `push_address`: ZMQ PUSH address (EA -> Server)
    /// - `config_address`: ZMQ SUB address for config (Server -> EA)
    /// - `trade_address`: ZMQ SUB address for trade signals (Slave only, None for Master)
    /// - `account_id`: EA account identifier
    /// - `ea_type`: Master or Slave
    pub fn new(
        push_address: &str,
        config_address: &str,
        trade_address: Option<&str>,
        account_id: &str,
        ea_type: EaType,
    ) -> Result<Self> {
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

        // Create trade socket for Slave EA (MQL5: g_zmq_trade_socket)
        // Master EA doesn't have trade_socket (only sends, doesn't receive trade signals)
        let trade_socket_handle = if trade_address.is_some() {
            let handle = zmq_socket_create(context_handle, ZMQ_SUB);
            if handle < 0 {
                zmq_socket_destroy(config_socket_handle);
                zmq_socket_destroy(push_socket_handle);
                zmq_context_destroy(context_handle);
                anyhow::bail!("Failed to create trade SUB socket");
            }
            Some(handle)
        } else {
            None
        };

        let push_addr_utf16: Vec<u16> = push_address.encode_utf16().chain(Some(0)).collect();
        let config_addr_utf16: Vec<u16> = config_address.encode_utf16().chain(Some(0)).collect();

        // Build config topic using mt-bridge FFI (same as actual EA)
        let account_id_utf16: Vec<u16> = account_id.encode_utf16().chain(Some(0)).collect();
        let mut topic_buffer = vec![0u16; TOPIC_BUFFER_SIZE as usize];
        let topic_len = unsafe {
            build_config_topic(
                account_id_utf16.as_ptr(),
                topic_buffer.as_mut_ptr(),
                TOPIC_BUFFER_SIZE,
            )
        };
        if topic_len <= 0 {
            zmq_socket_destroy(config_socket_handle);
            zmq_socket_destroy(push_socket_handle);
            zmq_context_destroy(context_handle);
            anyhow::bail!("Failed to build config topic");
        }

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

            if zmq_socket_subscribe(config_socket_handle, topic_buffer.as_ptr()) != 1 {
                if let Some(ts) = trade_socket_handle {
                    zmq_socket_destroy(ts);
                }
                zmq_socket_destroy(config_socket_handle);
                zmq_socket_destroy(push_socket_handle);
                zmq_context_destroy(context_handle);
                anyhow::bail!("Failed to subscribe to config topic");
            }

            // Connect trade socket (Slave only)
            // MQL5: g_zmq_trade_socket connects to same unified PUB address as config
            if let (Some(ts), Some(addr)) = (trade_socket_handle, trade_address) {
                let trade_addr_utf16: Vec<u16> = addr.encode_utf16().chain(Some(0)).collect();
                if zmq_socket_connect(ts, trade_addr_utf16.as_ptr()) != 1 {
                    zmq_socket_destroy(ts);
                    zmq_socket_destroy(config_socket_handle);
                    zmq_socket_destroy(push_socket_handle);
                    zmq_context_destroy(context_handle);
                    anyhow::bail!("Failed to connect trade SUB socket");
                }
                // Note: Trade topic subscription is done dynamically after config is received
                // (see ProcessConfigMessage in MQL5: SubscribeToTradeTopic)
            }
        }

        // Set heartbeat params based on EA type
        let heartbeat_params = match ea_type {
            EaType::Master => HeartbeatParams::master_default(),
            EaType::Slave => HeartbeatParams::slave_default(),
        };

        Ok(Self {
            context_handle,
            push_socket_handle,
            config_socket_handle,
            trade_socket_handle,
            account_id: account_id.to_string(),
            ea_type,
            // MQL5: g_last_trade_allowed = false (初期値)
            is_trade_allowed: Arc::new(AtomicBool::new(false)),
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            heartbeat_params,
        })
    }

    pub fn new_without_zmq(account_id: &str, ea_type: EaType) -> Result<Self> {
        let heartbeat_params = match ea_type {
            EaType::Master => HeartbeatParams::master_default(),
            EaType::Slave => HeartbeatParams::slave_default(),
        };

        Ok(Self {
            context_handle: -1,
            push_socket_handle: -1,
            config_socket_handle: -1,
            trade_socket_handle: None,
            account_id: account_id.to_string(),
            ea_type,
            is_trade_allowed: Arc::new(AtomicBool::new(false)),
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            heartbeat_params,
        })
    }

    /// Get account ID
    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    /// Get is_trade_allowed Arc for sharing with OnTimer thread
    pub(crate) fn is_trade_allowed_arc(&self) -> Arc<AtomicBool> {
        self.is_trade_allowed.clone()
    }

    /// Set is_trade_allowed state (simulates MT4/MT5 auto-trading toggle)
    ///
    /// In MQL5, this corresponds to TerminalInfoInteger(TERMINAL_TRADE_ALLOWED).
    /// Changing this value triggers a heartbeat on the next OnTimer cycle.
    pub fn set_trade_allowed(&self, allowed: bool) {
        self.is_trade_allowed.store(allowed, Ordering::SeqCst);
    }

    /// Get is_trade_allowed state
    pub fn is_trade_allowed(&self) -> bool {
        self.is_trade_allowed.load(Ordering::SeqCst)
    }

    /// Send heartbeat message (called from OnTimer loop)
    ///
    /// MQL5: SendHeartbeatMessage() in Messages.mqh
    #[allow(dead_code)]
    pub(crate) fn send_heartbeat(&self) -> Result<()> {
        self.send_heartbeat_with_options(self.is_trade_allowed.load(Ordering::SeqCst))
    }

    /// Send heartbeat with specific is_trade_allowed value
    #[allow(dead_code)]
    pub(crate) fn send_heartbeat_with_options(&self, is_trade_allowed: bool) -> Result<()> {
        let msg = HeartbeatMessage {
            message_type: "Heartbeat".to_string(),
            account_id: self.account_id.clone(),
            balance: self.heartbeat_params.balance,
            equity: self.heartbeat_params.equity,
            open_positions: 0,
            timestamp: Utc::now().to_rfc3339(),
            version: self.heartbeat_params.version.clone(),
            ea_type: self.ea_type.as_str().to_string(),
            platform: "MT5".to_string(),
            account_number: self.heartbeat_params.account_number,
            broker: "TestBroker".to_string(),
            account_name: self.heartbeat_params.account_name.clone(),
            server: "TestServer".to_string(),
            currency: "USD".to_string(),
            leverage: self.heartbeat_params.leverage,
            is_trade_allowed,
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

    /// Try to receive raw bytes from config socket (non-blocking, single attempt)
    ///
    /// MQL5: zmq_socket_receive(g_zmq_config_socket, ...) in OnTimer
    /// Returns (topic, payload) if data is available, None otherwise.
    pub(crate) fn try_receive_raw_nonblocking(&self) -> Result<Option<(String, Vec<u8>)>> {
        Self::receive_from_socket(self.config_socket_handle)
    }

    /// Internal helper to receive from any socket
    fn receive_from_socket(socket_handle: i32) -> Result<Option<(String, Vec<u8>)>> {
        let mut buffer = vec![0u8; BUFFER_SIZE];

        let received_bytes = unsafe {
            zmq_socket_receive(
                socket_handle,
                buffer.as_mut_ptr() as *mut c_char,
                BUFFER_SIZE as i32,
            )
        };

        if received_bytes > 0 {
            let bytes = &buffer[..received_bytes as usize];

            // Parse topic + space + payload format (MQL5: space_pos detection)
            if let Some(space_pos) = bytes.iter().position(|&b| b == b' ') {
                let topic = String::from_utf8_lossy(&bytes[..space_pos]).to_string();
                let payload = bytes[space_pos + 1..].to_vec();
                return Ok(Some((topic, payload)));
            } else {
                // No space separator, return empty topic with raw bytes
                return Ok(Some((String::new(), bytes.to_vec())));
            }
        }

        Ok(None)
    }

    /// Try to receive raw bytes with timeout (blocking with polling)
    ///
    /// This is a convenience method for tests that need to wait for messages.
    /// The actual OnTimer loop uses try_receive_raw_nonblocking().
    pub fn try_receive_raw(&self, timeout_ms: i32) -> Result<Option<(String, Vec<u8>)>> {
        let start = std::time::Instant::now();
        let timeout_duration = std::time::Duration::from_millis(timeout_ms as u64);

        loop {
            if let Some(result) = self.try_receive_raw_nonblocking()? {
                return Ok(Some(result));
            }

            if start.elapsed() >= timeout_duration {
                return Ok(None);
            }

            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}

impl Drop for EaSimulatorBase {
    fn drop(&mut self) {
        // Signal any background threads to stop
        self.shutdown_flag.store(true, Ordering::SeqCst);

        // Clean up ZMQ resources
        if let Some(ts) = self.trade_socket_handle {
            if ts >= 0 {
                zmq_socket_destroy(ts);
            }
        }
        if self.config_socket_handle >= 0 {
            zmq_socket_destroy(self.config_socket_handle);
        }
        if self.push_socket_handle >= 0 {
            zmq_socket_destroy(self.push_socket_handle);
        }
        if self.context_handle >= 0 {
            zmq_context_destroy(self.context_handle);
        }
    }
}
