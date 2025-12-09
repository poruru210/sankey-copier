// e2e-tests/src/master.rs
//
// Master EA Simulator - MQL5 SankeyCopierMaster.mq5 完全準拠実装
//
// Refactored to use EaContext via FFI, demonstrating strict encapsulation and Strategy Pattern.

#![allow(unused_imports)]

use anyhow::Result;
use chrono::Utc;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Instant;

use sankey_copier_zmq::ffi::{
    ea_connect, ea_context_free, ea_context_mark_config_requested,
    ea_context_should_request_config, ea_init, ea_receive_config, ea_send_close_signal,
    ea_send_heartbeat, ea_send_modify_signal, ea_send_open_signal, ea_send_push, ea_send_register,
    ea_subscribe_config,
};
use sankey_copier_zmq::EaContext;

use crate::base::EaSimulatorBase;
use crate::types::{
    EaType, MasterConfigMessage, OrderType, PositionInfo, PositionSnapshotMessage,
    SyncRequestMessage, TradeAction, TradeSignal, VLogsConfigMessage, HEARTBEAT_INTERVAL_SECONDS,
    ONTIMER_INTERVAL_MS, STATUS_NO_CONFIG,
};

// Wrapper for thread-safe passing of EaContext pointer (which is !Send !Sync by default)
struct ContextWrapper(pub *mut EaContext);
unsafe impl Send for ContextWrapper {}
unsafe impl Sync for ContextWrapper {}

// =============================================================================
// Master EA Simulator
// =============================================================================

pub struct MasterEaSimulator {
    base: EaSimulatorBase,

    // --- MQL5 Global Variables ---
    g_last_heartbeat: Arc<Mutex<Option<Instant>>>,
    g_config_requested: Arc<AtomicBool>,
    g_last_trade_allowed: Arc<AtomicBool>,
    g_server_status: Arc<AtomicI32>,
    g_symbol_prefix: Arc<Mutex<String>>,
    g_symbol_suffix: Arc<Mutex<String>>,

    // --- Received Data Queues (Verification) ---
    received_sync_requests: Arc<Mutex<Vec<SyncRequestMessage>>>,
    received_vlogs_configs: Arc<Mutex<Vec<VLogsConfigMessage>>>,
    received_config: Arc<Mutex<Option<MasterConfigMessage>>>, // For test verification

    // --- State ---
    g_register_sent: Arc<AtomicBool>,
    ontimer_thread: Option<JoinHandle<()>>,

    // --- Context (Managed in OnTimer thread, accessible via FFI wrapper) ---
    context: Arc<Mutex<Option<ContextWrapper>>>,

    // Connection Params (Passed to Init/Connect)
    push_address: String,
    config_address: String,

    // Pending Subscriptions (Thread-safe queue)
    pending_subscriptions: Arc<Mutex<Vec<String>>>,
}

impl MasterEaSimulator {
    pub fn new(push_address: &str, config_address: &str, account_id: &str) -> Result<Self> {
        // Use new_without_zmq to avoid creating raw sockets in base
        let base = EaSimulatorBase::new_without_zmq(account_id, EaType::Master)?;

        Ok(Self {
            base,
            g_last_heartbeat: Arc::new(Mutex::new(None)),
            g_config_requested: Arc::new(AtomicBool::new(false)),
            g_last_trade_allowed: Arc::new(AtomicBool::new(false)),
            g_server_status: Arc::new(AtomicI32::new(STATUS_NO_CONFIG)),
            g_symbol_prefix: Arc::new(Mutex::new(String::new())),
            g_symbol_suffix: Arc::new(Mutex::new(String::new())),
            received_sync_requests: Arc::new(Mutex::new(Vec::new())),
            received_vlogs_configs: Arc::new(Mutex::new(Vec::new())),
            received_config: Arc::new(Mutex::new(None)),
            g_register_sent: Arc::new(AtomicBool::new(false)),
            ontimer_thread: None,
            context: Arc::new(Mutex::new(None)),
            push_address: push_address.to_string(),
            config_address: config_address.to_string(),
            pending_subscriptions: Arc::new(Mutex::new(Vec::new())),
        })
    }

    pub fn start(&mut self) -> Result<()> {
        if self.ontimer_thread.is_some() {
            return Ok(());
        }

        let account_id = self.base.account_id().to_string();
        let shutdown_flag = self.base.shutdown_flag.clone();
        let is_trade_allowed = self.base.is_trade_allowed_arc();
        let heartbeat_params = self.base.heartbeat_params.clone();
        let ea_type_val = self.base.ea_type;

        let g_last_heartbeat = self.g_last_heartbeat.clone();
        let g_config_requested = self.g_config_requested.clone();
        let g_last_trade_allowed = self.g_last_trade_allowed.clone();
        let g_server_status = self.g_server_status.clone();
        let g_symbol_prefix = self.g_symbol_prefix.clone();
        let g_symbol_suffix = self.g_symbol_suffix.clone();
        let received_sync_requests = self.received_sync_requests.clone();
        let received_vlogs_configs = self.received_vlogs_configs.clone();
        let received_config = self.received_config.clone();
        let g_register_sent = self.g_register_sent.clone();
        let context_mutex = self.context.clone();
        let push_addr = self.push_address.clone();
        let config_addr = self.config_address.clone();
        let pending_subs = self.pending_subscriptions.clone();

        let handle = std::thread::spawn(move || {
            let to_u16 = |s: &str| -> Vec<u16> { s.encode_utf16().chain(Some(0)).collect() };

            // 1. ea_init
            let acc_id_u16 = to_u16(&account_id);
            let ea_type_u16 = to_u16(ea_type_val.as_str());
            let platform_u16 = to_u16("MT5");
            let broker_u16 = to_u16("TestBroker");
            let acc_name_u16 = to_u16(&heartbeat_params.account_name);
            let server_u16 = to_u16("TestServer");
            let currency_u16 = to_u16("USD");

            let ctx = unsafe {
                ea_init(
                    acc_id_u16.as_ptr(),
                    ea_type_u16.as_ptr(),
                    platform_u16.as_ptr(),
                    heartbeat_params.account_number,
                    broker_u16.as_ptr(),
                    acc_name_u16.as_ptr(),
                    server_u16.as_ptr(),
                    currency_u16.as_ptr(),
                    heartbeat_params.leverage,
                )
            };

            if ctx.is_null() {
                eprintln!("Failed to initialize EA context!");
                return;
            }

            {
                let mut guard = context_mutex.lock().unwrap();
                *guard = Some(ContextWrapper(ctx));
            }

            // 2. ea_connect
            let push_u16 = to_u16(&push_addr);
            let sub_u16 = to_u16(&config_addr);
            // trade_u16 not used in ea_connect signature anymore (Strategy handles sub reuse)

            unsafe {
                if ea_connect(ctx, push_u16.as_ptr(), sub_u16.as_ptr()) != 1 {
                    eprintln!("Failed to connect EA context!");
                    return;
                }
            }

            // 3. OnTimer Loop
            while !shutdown_flag.load(Ordering::SeqCst) {
                // Process pending subscriptions
                {
                    let mut subs = pending_subs.lock().unwrap();
                    if !subs.is_empty() {
                        for topic in subs.iter() {
                            let topic_u16 = to_u16(topic);
                            unsafe {
                                ea_subscribe_config(ctx, topic_u16.as_ptr());
                            }
                        }
                        subs.clear();
                    }
                }

                std::thread::sleep(std::time::Duration::from_millis(ONTIMER_INTERVAL_MS));
                if shutdown_flag.load(Ordering::SeqCst) {
                    break;
                }

                // P. Register
                if !g_register_sent.load(Ordering::SeqCst) {
                    let mut buffer = vec![0u8; 1024];
                    let len =
                        unsafe { ea_send_register(ctx, buffer.as_mut_ptr(), buffer.len() as i32) };
                    if len > 0 {
                        unsafe {
                            ea_send_push(ctx, buffer.as_ptr(), len);
                        }
                        g_register_sent.store(true, Ordering::SeqCst);
                    }
                }

                let current_trade_allowed = is_trade_allowed.load(Ordering::SeqCst);
                let last_trade_allowed_val = g_last_trade_allowed.load(Ordering::SeqCst);
                let trade_state_changed = current_trade_allowed != last_trade_allowed_val;

                let now = Instant::now();
                let last_hb = *g_last_heartbeat.lock().unwrap();
                let should_send_heartbeat = match last_hb {
                    None => true,
                    Some(last) => {
                        now.duration_since(last).as_secs() >= HEARTBEAT_INTERVAL_SECONDS
                            || trade_state_changed
                    }
                };

                if should_send_heartbeat {
                    let mut buffer = vec![0u8; 1024];
                    let len = unsafe {
                        ea_send_heartbeat(
                            ctx,
                            heartbeat_params.balance,
                            heartbeat_params.equity,
                            0, // open_positions
                            if current_trade_allowed { 1 } else { 0 },
                            buffer.as_mut_ptr(),
                            buffer.len() as i32,
                        )
                    };

                    if len > 0 {
                        unsafe {
                            ea_send_push(ctx, buffer.as_ptr(), len);
                        }

                        *g_last_heartbeat.lock().unwrap() = Some(Instant::now());
                        if trade_state_changed {
                            g_last_trade_allowed.store(current_trade_allowed, Ordering::SeqCst);
                        }

                        let should_request = unsafe {
                            ea_context_should_request_config(
                                ctx,
                                if current_trade_allowed { 1 } else { 0 },
                            )
                        };

                        if should_request == 1 {
                            unsafe {
                                sankey_copier_zmq::ffi::ea_send_request_config(ctx, 0);
                            }
                            g_config_requested.store(true, Ordering::SeqCst);
                        }
                    }
                }

                // Config Receive Loop
                loop {
                    let mut buffer = vec![0u8; crate::types::BUFFER_SIZE];

                    let received_bytes = unsafe {
                        ea_receive_config(
                            ctx,
                            buffer.as_mut_ptr(),
                            crate::types::BUFFER_SIZE as i32,
                        )
                    };

                    if received_bytes <= 0 {
                        break;
                    }

                    let bytes = &buffer[..received_bytes as usize];
                    if let Some(space_pos) = bytes.iter().position(|&b| b == b' ') {
                        let topic = String::from_utf8_lossy(&bytes[..space_pos]).to_string();
                        let payload = &bytes[space_pos + 1..];

                        if topic.starts_with("sync/") {
                            if let Ok(msg) = rmp_serde::from_slice::<SyncRequestMessage>(payload) {
                                received_sync_requests.lock().unwrap().push(msg);
                            }
                        } else if topic.starts_with("config/") {
                            if let Ok(config) =
                                rmp_serde::from_slice::<MasterConfigMessage>(payload)
                            {
                                g_server_status.store(config.status, Ordering::SeqCst);
                                if let Some(prefix) = &config.symbol_prefix {
                                    *g_symbol_prefix.lock().unwrap() = prefix.clone();
                                }
                                if let Some(suffix) = &config.symbol_suffix {
                                    *g_symbol_suffix.lock().unwrap() = suffix.clone();
                                }
                                unsafe {
                                    ea_context_mark_config_requested(ctx);
                                }
                                *received_config.lock().unwrap() = Some(config);
                            } else if let Ok(vlogs) =
                                rmp_serde::from_slice::<VLogsConfigMessage>(payload)
                            {
                                received_vlogs_configs.lock().unwrap().push(vlogs);
                            }
                        }
                    }
                }
            } // End While

            let mut buffer = vec![0u8; 1024];
            let len = unsafe {
                sankey_copier_zmq::ffi::ea_send_unregister(
                    ctx,
                    buffer.as_mut_ptr(),
                    buffer.len() as i32,
                )
            };
            if len > 0 {
                unsafe {
                    ea_send_push(ctx, buffer.as_ptr(), len);
                }
            }

            {
                let mut guard = context_mutex.lock().unwrap();
                *guard = None;
            }
            unsafe {
                ea_context_free(ctx);
            }
        });

        self.ontimer_thread = Some(handle);

        // Wait for initialization
        let start = std::time::Instant::now();
        while start.elapsed().as_secs() < 5 {
            if self.context.lock().unwrap().is_some() {
                return Ok(());
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        Err(anyhow::anyhow!(
            "Timed out waiting for EA context initialization"
        ))
    }

    // =========================================================================
    // Helpers
    // =========================================================================
    pub fn account_id(&self) -> &str {
        self.base.account_id()
    }

    pub fn set_trade_allowed(&self, allowed: bool) {
        self.base.set_trade_allowed(allowed);
    }

    pub fn wait_for_status(
        &self,
        expected: i32,
        timeout_ms: i32,
    ) -> Result<Option<MasterConfigMessage>> {
        let start = std::time::Instant::now();
        while start.elapsed().as_millis() < timeout_ms as u128 {
            if self.g_server_status.load(Ordering::SeqCst) == expected {
                return Ok(self.received_config.lock().unwrap().clone());
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        Ok(None)
    }

    pub fn try_receive_sync_request(&self, timeout_ms: i32) -> Result<Option<SyncRequestMessage>> {
        let start = std::time::Instant::now();
        while start.elapsed().as_millis() < timeout_ms as u128 {
            let mut lock = self.received_sync_requests.lock().unwrap();
            if !lock.is_empty() {
                return Ok(Some(lock.remove(0)));
            }
            drop(lock);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        Ok(None)
    }

    // For test compatibility - returns config if received, but does NOT consume from socket directly
    pub fn try_receive_master_config(
        &self,
        timeout_ms: i32,
    ) -> Result<Option<MasterConfigMessage>> {
        let start = std::time::Instant::now();
        while start.elapsed().as_millis() < timeout_ms as u128 {
            let cfg = self.received_config.lock().unwrap().clone();
            if let Some(c) = cfg {
                return Ok(Some(c));
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        Ok(None)
    }

    pub fn send_trade_signal(&self, signal: &TradeSignal) -> Result<()> {
        let guard = self.context.lock().unwrap();
        let ctx = match guard.as_ref() {
            Some(w) => w.0,
            None => return Err(anyhow::anyhow!("Context not initialized")),
        };

        let to_u16 = |s: &str| -> Vec<u16> { s.encode_utf16().chain(Some(0)).collect() };

        // If signal has a custom timestamp (simulated delay), use raw bytes to bypass FFI timestamp generation
        if (Utc::now() - signal.timestamp).num_seconds().abs() > 1 {
            let bytes = rmp_serde::to_vec_named(signal)?;
            let ret = unsafe { ea_send_push(ctx, bytes.as_ptr(), bytes.len() as i32) };
            if ret == 1 {
                return Ok(());
            } else {
                return Err(anyhow::anyhow!(
                    "Failed to send push (raw timestamp bypass)"
                ));
            }
        }

        match signal.action {
            TradeAction::Open => {
                let symbol = to_u16(signal.symbol.as_deref().unwrap_or(""));
                let ot_str = signal
                    .order_type
                    .as_ref()
                    .map(|ot| format!("{:?}", ot))
                    .unwrap_or_default();
                let order_type = to_u16(&ot_str);
                let comment = to_u16(signal.comment.as_deref().unwrap_or(""));

                let ret = unsafe {
                    ea_send_open_signal(
                        ctx,
                        signal.ticket,
                        symbol.as_ptr(),
                        order_type.as_ptr(),
                        signal.lots.unwrap_or(0.0),
                        signal.open_price.unwrap_or(0.0),
                        signal.stop_loss.unwrap_or(0.0),
                        signal.take_profit.unwrap_or(0.0),
                        signal.magic_number.unwrap_or(0),
                        comment.as_ptr(),
                    )
                };
                if ret != 1 {
                    return Err(anyhow::anyhow!("Failed to send open signal"));
                }
            }
            TradeAction::Close => {
                let ret = unsafe {
                    ea_send_close_signal(
                        ctx,
                        signal.ticket,
                        signal.lots.unwrap_or(0.0),
                        signal.close_ratio.unwrap_or(1.0), // Default to full close if not specified
                    )
                };
                if ret != 1 {
                    return Err(anyhow::anyhow!("Failed to send close signal"));
                }
            }
            TradeAction::Modify => {
                let ret = unsafe {
                    ea_send_modify_signal(
                        ctx,
                        signal.ticket,
                        signal.stop_loss.unwrap_or(0.0),
                        signal.take_profit.unwrap_or(0.0),
                    )
                };
                if ret != 1 {
                    return Err(anyhow::anyhow!("Failed to send modify signal"));
                }
            }
        }

        Ok(())
    }

    fn send_raw_bytes(&self, data: &[u8]) -> Result<()> {
        let guard = self.context.lock().unwrap();
        if let Some(wrapper) = guard.as_ref() {
            let ret = unsafe { ea_send_push(wrapper.0, data.as_ptr(), data.len() as i32) };
            if ret == 1 {
                Ok(())
            } else {
                Err(anyhow::anyhow!("Failed to send push"))
            }
        } else {
            Err(anyhow::anyhow!("Context not initialized"))
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_open_signal(
        &self,
        ticket: i64,
        symbol: &str,
        order_type: OrderType,
        lots: f64,
        price: f64,
        sl: Option<f64>,
        tp: Option<f64>,
        magic: i64,
    ) -> TradeSignal {
        TradeSignal {
            action: TradeAction::Open,
            ticket,
            symbol: Some(symbol.to_string()),
            order_type: Some(order_type),
            lots: Some(lots),
            open_price: Some(price),
            stop_loss: sl,
            take_profit: tp,
            magic_number: Some(magic),
            comment: Some("E2E Test".to_string()),
            timestamp: Utc::now(),
            source_account: self.base.account_id().to_string(),
            close_ratio: None,
        }
    }

    pub fn create_close_signal(&self, ticket: i64, symbol: &str, lots: f64) -> TradeSignal {
        TradeSignal {
            action: TradeAction::Close,
            ticket,
            symbol: Some(symbol.to_string()),
            order_type: None,
            lots: Some(lots),
            open_price: None,
            stop_loss: None,
            take_profit: None,
            magic_number: Some(0),
            comment: Some("E2E Test Close".to_string()),
            timestamp: Utc::now(),
            source_account: self.base.account_id().to_string(),
            close_ratio: None,
        }
    }

    pub fn create_partial_close_signal(
        &self,
        ticket: i64,
        symbol: &str,
        lots: f64,
        close_ratio: f64,
    ) -> TradeSignal {
        TradeSignal {
            action: TradeAction::Close,
            ticket,
            symbol: Some(symbol.to_string()),
            order_type: None,
            lots: Some(lots),
            open_price: None,
            stop_loss: None,
            take_profit: None,
            magic_number: Some(0),
            comment: Some("E2E Test Partial Close".to_string()),
            timestamp: Utc::now(),
            source_account: self.base.account_id().to_string(),
            close_ratio: Some(close_ratio),
        }
    }

    pub fn create_modify_signal(
        &self,
        ticket: i64,
        symbol: &str,
        sl: Option<f64>,
        tp: Option<f64>,
    ) -> TradeSignal {
        TradeSignal {
            action: TradeAction::Modify,
            ticket,
            symbol: Some(symbol.to_string()),
            order_type: None,
            lots: None,
            open_price: None,
            stop_loss: sl,
            take_profit: tp,
            magic_number: None,
            comment: Some("E2E Test Modify".to_string()),
            timestamp: Utc::now(),
            source_account: self.base.account_id().to_string(),
            close_ratio: None,
        }
    }

    pub fn create_delayed_signal(&self, mut signal: TradeSignal, delay_ms: i64) -> TradeSignal {
        let past_time = Utc::now() - chrono::Duration::milliseconds(delay_ms);
        signal.timestamp = past_time;
        signal
    }

    pub fn send_position_snapshot(&self, positions: Vec<PositionInfo>) -> Result<()> {
        let msg = PositionSnapshotMessage {
            message_type: "PositionSnapshot".to_string(),
            source_account: self.base.account_id().to_string(),
            positions,
            timestamp: Utc::now().to_rfc3339(),
        };
        let bytes = rmp_serde::to_vec_named(&msg)?;
        self.send_raw_bytes(&bytes)
    }

    pub fn create_test_position(
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
            stop_loss: None,
            take_profit: None,
            magic_number: Some(0),
            comment: Some("E2E Test Position".to_string()),
            open_time: Utc::now().to_rfc3339(),
        }
    }

    pub fn subscribe_to_sync_requests(&self) -> Result<()> {
        let topic = format!("sync/{}/", self.base.account_id());
        self.pending_subscriptions.lock().unwrap().push(topic);
        Ok(())
    }

    pub fn subscribe_to_global_config(&self) -> Result<()> {
        self.pending_subscriptions
            .lock()
            .unwrap()
            .push("config/global".to_string());
        Ok(())
    }

    pub fn try_receive_vlogs_config(&self, timeout_ms: i32) -> Result<Option<VLogsConfigMessage>> {
        let start = std::time::Instant::now();
        while start.elapsed().as_millis() < timeout_ms as u128 {
            let mut lock = self.received_vlogs_configs.lock().unwrap();
            if !lock.is_empty() {
                return Ok(Some(lock.remove(0)));
            }
            drop(lock);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        Ok(None)
    }
}

impl Drop for MasterEaSimulator {
    fn drop(&mut self) {
        self.base.shutdown_flag.store(true, Ordering::SeqCst);
        if let Some(handle) = self.ontimer_thread.take() {
            let _ = handle.join();
        }
    }
}
