// e2e-tests/src/master.rs
//
// Master EA Simulator - MQL5 SankeyCopierMaster.mq5 完全準拠実装
//
// Refactored to use EaContext via FFI, demonstrating strict encapsulation and Strategy Pattern.
// Now using the Platform Simulator infrastructure and EaContextWrapper.

#![allow(unused_imports)]

use anyhow::Result;
use chrono::Utc;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Instant;

use sankey_copier_zmq::ea_context::{EaCommand, EaCommandType};
use sankey_copier_zmq::ffi::*; // Use all FFI functions available
use sankey_copier_zmq::EaContext;

use crate::base::EaSimulatorBase;
use crate::platform::context::MasterContextWrapper;
use crate::platform::runner::PlatformRunner;
use crate::platform::traits::ExpertAdvisor;
use crate::platform::types::{ENUM_DEINIT_REASON, ENUM_INIT_RETCODE};
use crate::types::{
    EaType, GlobalConfigMessage, MasterConfigMessage, OrderType, PositionInfo,
    PositionSnapshotMessage, SyncRequestMessage, TradeAction, TradeSignal,
    HEARTBEAT_INTERVAL_SECONDS, ONTIMER_INTERVAL_MS, STATUS_NO_CONFIG,
};

// =============================================================================
// Master EA Core (MQL5 Logic)
// =============================================================================

#[derive(Clone)]
struct MasterEaCore {
    // Shared state from MasterEaSimulator
    account_id: String,
    ea_type: EaType,
    heartbeat_params: crate::types::HeartbeatParams,
    _shutdown_flag: Arc<AtomicBool>,
    is_trade_allowed: Arc<AtomicBool>,

    _g_last_heartbeat: Arc<Mutex<Option<Instant>>>,
    _g_config_requested: Arc<AtomicBool>,
    _g_last_trade_allowed: Arc<AtomicBool>,
    g_server_status: Arc<AtomicI32>,
    g_symbol_prefix: Arc<Mutex<String>>,
    g_symbol_suffix: Arc<Mutex<String>>,

    received_sync_requests: Arc<Mutex<Vec<SyncRequestMessage>>>,
    _received_vlogs_configs: Arc<Mutex<Vec<GlobalConfigMessage>>>,
    received_config: Arc<Mutex<Option<MasterConfigMessage>>>,

    _g_register_sent: Arc<AtomicBool>,
    context: Arc<Mutex<Option<MasterContextWrapper>>>,
    push_address: String,
    config_address: String,
    pending_subscriptions: Arc<Mutex<Vec<String>>>,
}

impl ExpertAdvisor for MasterEaCore {
    fn on_init(&mut self) -> ENUM_INIT_RETCODE {
        let to_u16 = |s: &str| -> Vec<u16> { s.encode_utf16().chain(Some(0)).collect() };

        let acc_id_u16 = to_u16(&self.account_id);
        let ea_type_u16 = to_u16(self.ea_type.as_str());
        let platform_u16 = to_u16("MT5");
        let broker_u16 = to_u16("TestBroker");
        let acc_name_u16 = to_u16(&self.heartbeat_params.account_name);
        let server_u16 = to_u16("TestServer");
        let currency_u16 = to_u16("USD");

        let ctx_ptr = unsafe {
            ea_init(
                acc_id_u16.as_ptr(),
                ea_type_u16.as_ptr(),
                platform_u16.as_ptr(),
                self.heartbeat_params.account_number,
                broker_u16.as_ptr(),
                acc_name_u16.as_ptr(),
                server_u16.as_ptr(),
                currency_u16.as_ptr(),
                self.heartbeat_params.leverage,
            )
        };

        if ctx_ptr.is_null() {
            eprintln!("Failed to initialize EA context!");
            return ENUM_INIT_RETCODE::INIT_FAILED;
        }

        {
            let mut guard = self.context.lock().unwrap();
            *guard = Some(MasterContextWrapper::new(ctx_ptr));
        }

        let push_u16 = to_u16(&self.push_address);
        let sub_u16 = to_u16(&self.config_address);

        unsafe {
            if ea_connect(ctx_ptr, push_u16.as_ptr(), sub_u16.as_ptr()) != 1 {
                eprintln!("Failed to connect EA context!");
                return ENUM_INIT_RETCODE::INIT_FAILED;
            }
        }

        ENUM_INIT_RETCODE::INIT_SUCCEEDED
    }

    fn on_deinit(&mut self, _reason: ENUM_DEINIT_REASON) {
        let mut guard = self.context.lock().unwrap();
        if let Some(wrapper) = guard.take() {
            let ctx = wrapper.raw();
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
            wrapper.free(); // Safely calls ea_context_free
        }
    }

    fn on_timer(&mut self) {
        // Retrieve context
        let guard = self.context.lock().unwrap();
        let wrapper = match guard.as_ref() {
            Some(w) => w,
            None => return,
        };
        let ctx = wrapper.raw();

        let to_u16 = |s: &str| -> Vec<u16> { s.encode_utf16().chain(Some(0)).collect() };

        // Process pending subscriptions (One-off or manual subscriptions still valid)
        {
            let mut subs = self.pending_subscriptions.lock().unwrap();
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

        let current_trade_allowed = self.is_trade_allowed.load(Ordering::SeqCst);

        // 1. Rust Manager Tick (Handles Heartbeat, IO polling)
        // Return value: 1 if commands pending, 0 otherwise
        let status = unsafe {
            ea_manager_tick(
                ctx,
                self.heartbeat_params.balance,
                self.heartbeat_params.equity,
                0, // open_positions (TODO: track if needed)
                if current_trade_allowed { 1 } else { 0 },
            )
        };

        // 2. Process Commands
        if status == 1 {
            let mut cmd = EaCommand::default();
            while unsafe { ea_get_command(ctx, &mut cmd) } == 1 {
                let cmd_type =
                    unsafe { std::mem::transmute::<i32, EaCommandType>(cmd.command_type) };
                match cmd_type {
                    EaCommandType::SendSnapshot => {
                        // Slave requested sync.
                        // Use safe wrapper to get full SyncRequestMessage
                        if let Some(req) = wrapper.get_sync_request() {
                            // Correct timestamp to be now for new request if needed, or keep original?
                            // Here we just clone/store it as received
                            self.received_sync_requests.lock().unwrap().push(req);
                        }
                    }
                    EaCommandType::UpdateUi => {
                        // Config updated. Retrieve it via safe wrapper.
                        if let Some(cfg) = wrapper.get_master_config() {
                            // Update internal state
                            self.g_server_status.store(cfg.status, Ordering::SeqCst);
                            if let Some(prefix) = &cfg.symbol_prefix {
                                *self.g_symbol_prefix.lock().unwrap() = prefix.clone();
                            }
                            if let Some(suffix) = &cfg.symbol_suffix {
                                *self.g_symbol_suffix.lock().unwrap() = suffix.clone();
                            }
                            // Push to received queue for verification
                            *self.received_config.lock().unwrap() = Some(cfg);
                        }
                        // Also check for global config updates
                        if let Some(cfg) = wrapper.get_global_config() {
                            self._received_vlogs_configs.lock().unwrap().push(cfg);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

// =============================================================================
// Master EA Simulator (Facade)
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
    received_vlogs_configs: Arc<Mutex<Vec<GlobalConfigMessage>>>,
    received_config: Arc<Mutex<Option<MasterConfigMessage>>>,

    // --- State ---
    g_register_sent: Arc<AtomicBool>,

    // Platform Runner & Timer Thread
    runner: Option<PlatformRunner>,
    timer_thread: Option<JoinHandle<()>>,

    // --- Context (Managed in OnTimer thread, accessible via FFI wrapper) ---
    context: Arc<Mutex<Option<MasterContextWrapper>>>,

    // Connection Params (Passed to Init/Connect)
    push_address: String,
    config_address: String,

    // Pending Subscriptions (Thread-safe queue)
    pending_subscriptions: Arc<Mutex<Vec<String>>>,
}

impl MasterEaSimulator {
    pub fn new(
        ini_path: &std::path::Path,
        account_id: &str,
        is_trade_allowed: bool,
    ) -> Result<Self> {
        let base = EaSimulatorBase::new_without_zmq(account_id, EaType::Master, is_trade_allowed)?;

        // Load INI config
        let ini_conf = crate::ini_config::EaIniConfig::load_from_file(ini_path)
            .map_err(|e| anyhow::anyhow!("Failed to load INI config: {}", e))?;

        let push_address = format!("tcp://127.0.0.1:{}", ini_conf.receiver_port);
        let config_address = format!("tcp://127.0.0.1:{}", ini_conf.publisher_port);

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
            runner: None,
            timer_thread: None,
            context: Arc::new(Mutex::new(None)),
            push_address,
            config_address,
            pending_subscriptions: Arc::new(Mutex::new(Vec::new())),
        })
    }

    pub fn start(&mut self) -> Result<()> {
        if self.runner.is_some() {
            return Ok(());
        }

        let core = MasterEaCore {
            account_id: self.base.account_id().to_string(),
            ea_type: self.base.ea_type,
            heartbeat_params: self.base.heartbeat_params.clone(),
            _shutdown_flag: self.base.shutdown_flag.clone(),
            is_trade_allowed: self.base.is_trade_allowed_arc(),
            _g_last_heartbeat: self.g_last_heartbeat.clone(),
            _g_config_requested: self.g_config_requested.clone(),
            _g_last_trade_allowed: self.g_last_trade_allowed.clone(),
            g_server_status: self.g_server_status.clone(),
            g_symbol_prefix: self.g_symbol_prefix.clone(),
            g_symbol_suffix: self.g_symbol_suffix.clone(),
            received_sync_requests: self.received_sync_requests.clone(),
            _received_vlogs_configs: self.received_vlogs_configs.clone(),
            received_config: self.received_config.clone(),
            _g_register_sent: self.g_register_sent.clone(),
            context: self.context.clone(),
            push_address: self.push_address.clone(),
            config_address: self.config_address.clone(),
            pending_subscriptions: self.pending_subscriptions.clone(),
        };

        // Initialize Platform Runner
        let runner = PlatformRunner::new(core);
        let sender = runner.get_sender();

        self.runner = Some(runner);

        // Wait for initialization
        let start = std::time::Instant::now();
        while start.elapsed().as_secs() < 5 {
            if self.context.lock().unwrap().is_some() {
                // Initialized. Now start timer thread.
                let shutdown_flag = self.base.shutdown_flag.clone();
                let timer_sender = sender.clone();

                let handle = std::thread::spawn(move || {
                    while !shutdown_flag.load(Ordering::SeqCst) {
                        std::thread::sleep(std::time::Duration::from_millis(ONTIMER_INTERVAL_MS));
                        if shutdown_flag.load(Ordering::SeqCst) {
                            break;
                        }
                        let _ = timer_sender.send(crate::platform::runner::PlatformEvent::Timer);
                    }
                });

                self.timer_thread = Some(handle);
                return Ok(());
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        Err(anyhow::anyhow!(
            "Timed out waiting for EA context initialization"
        ))
    }

    // ... Helpers ... (rest of the file)
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
        let wrapper = match guard.as_ref() {
            Some(w) => w,
            None => return Err(anyhow::anyhow!("Context not initialized")),
        };
        let ctx = wrapper.raw();

        let to_u16 = |s: &str| -> Vec<u16> { s.encode_utf16().chain(Some(0)).collect() };

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
                        signal.close_ratio.unwrap_or(1.0),
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
            let ret = unsafe { ea_send_push(wrapper.raw(), data.as_ptr(), data.len() as i32) };
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

    pub fn try_receive_vlogs_config(&self, timeout_ms: i32) -> Result<Option<GlobalConfigMessage>> {
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
        if let Some(mut runner) = self.runner.take() {
            runner.stop(ENUM_DEINIT_REASON::REASON_REMOVE);
        }
        if let Some(handle) = self.timer_thread.take() {
            let _ = handle.join();
        }
    }
}
