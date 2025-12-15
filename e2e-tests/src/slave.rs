// e2e-tests/src/slave.rs
//
// Slave EA Simulator - MQL5 SankeyCopierSlave.mq5 完全準拠実装
//
// Refactored to use EaContext via FFI.
// Now using the Platform Simulator infrastructure and EaContextWrapper.

use anyhow::Result;
use chrono::Utc;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Instant;

use sankey_copier_zmq::ea_context::{EaCommand, EaCommandType};
use sankey_copier_zmq::ffi::*; // Use all FFI functions available

use crate::base::EaSimulatorBase;
use crate::platform::context::SlaveContextWrapper;
use crate::platform::runner::PlatformRunner;
use crate::platform::traits::ExpertAdvisor;
use crate::platform::types::{ENUM_DEINIT_REASON, ENUM_INIT_RETCODE};
use crate::types::{
    EaType, GlobalConfigMessage, PositionSnapshotMessage, SlaveConfig, TradeAction, TradeSignal,
    UnregisterMessage, ONTIMER_INTERVAL_MS, STATUS_NO_CONFIG,
};

// =============================================================================
// Slave EA Core (MQL5 Logic)
// =============================================================================

#[derive(Clone)]
struct SlaveEaCore {
    // Shared state from SlaveEaSimulator
    account_id: String,
    ea_type: EaType,
    heartbeat_params: crate::types::HeartbeatParams,
    _shutdown_flag: Arc<AtomicBool>,
    is_trade_allowed: Arc<AtomicBool>,
    _master_account: String,

    _g_last_heartbeat: Arc<Mutex<Option<Instant>>>,
    _g_config_requested: Arc<AtomicBool>,
    _g_last_trade_allowed: Arc<AtomicBool>,
    _g_has_received_config: Arc<AtomicBool>,
    g_configs: Arc<Mutex<Vec<SlaveConfig>>>,
    last_received_status: Arc<AtomicI32>,

    _g_register_sent: Arc<AtomicBool>,
    received_trade_signals: Arc<Mutex<Vec<TradeSignal>>>,
    received_position_snapshots: Arc<Mutex<Vec<PositionSnapshotMessage>>>,
    _received_vlogs_configs: Arc<Mutex<Vec<GlobalConfigMessage>>>,

    _subscribed_masters: Arc<Mutex<Vec<String>>>,
    pending_subscriptions: Arc<Mutex<Vec<String>>>,
    context: Arc<Mutex<Option<SlaveContextWrapper>>>,

    push_address: String,
    config_address: String,
}

impl ExpertAdvisor for SlaveEaCore {
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
            *guard = Some(SlaveContextWrapper::new(ctx_ptr));
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
            let len = unsafe { ea_send_unregister(ctx, buffer.as_mut_ptr(), buffer.len() as i32) };
            if len > 0 {
                unsafe {
                    ea_send_push(ctx, buffer.as_ptr(), len);
                }
            }
            wrapper.free(); // Safely calls ea_context_free
        }
    }

    fn on_timer(&mut self) {
        let guard = self.context.lock().unwrap();
        let wrapper = match guard.as_ref() {
            Some(w) => w,
            None => return,
        };
        let ctx = wrapper.raw();

        // Helper for string conversion
        let to_u16 = |s: &str| -> Vec<u16> { s.encode_utf16().chain(Some(0)).collect() };

        // Process pending subscriptions
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

        // 1. Rust Manager Tick
        let status = unsafe {
            ea_manager_tick(
                ctx,
                self.heartbeat_params.balance,
                self.heartbeat_params.equity,
                0,
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
                    EaCommandType::UpdateUi => {
                        // Config updated. Retrieve via safe wrapper.
                        if let Some(cfg) = wrapper.get_slave_config() {
                            // Update verification queues
                            {
                                let mut configs = self.g_configs.lock().unwrap();
                                if let Some(existing) = configs
                                    .iter_mut()
                                    .find(|c| c.master_account == cfg.master_account)
                                {
                                    *existing = cfg.clone();
                                } else {
                                    configs.push(cfg.clone());
                                }
                            }
                            self.last_received_status
                                .store(cfg.status, Ordering::SeqCst);
                            unsafe { ea_context_mark_config_requested(ctx) };
                        }
                        // Also check for global config updates
                        if let Some(cfg) = wrapper.get_global_config() {
                            self._received_vlogs_configs.lock().unwrap().push(cfg);
                        }
                    }
                    EaCommandType::ProcessSnapshot => {
                        let snapshots = wrapper.get_position_snapshot();
                        let source = wrapper.get_position_snapshot_source_account();
                        // Always process snapshot command, even if empty positions
                        let snap = PositionSnapshotMessage {
                            message_type: "PositionSnapshot".to_string(),
                            source_account: if source.is_empty() {
                                "Unknown".to_string()
                            } else {
                                source
                            },
                            positions: snapshots,
                            timestamp: Utc::now().to_rfc3339(),
                        };
                        self.received_position_snapshots.lock().unwrap().push(snap);
                    }
                    EaCommandType::Open | EaCommandType::Close | EaCommandType::Modify => {
                        // Reconstruct TradeSignal for verification
                        let sym_bytes = &cmd.symbol;
                        let end = sym_bytes
                            .iter()
                            .position(|&x| x == 0)
                            .unwrap_or(sym_bytes.len());
                        let symbol = String::from_utf8_lossy(&sym_bytes[..end]).to_string();

                        let action = match cmd_type {
                            EaCommandType::Open => TradeAction::Open,
                            EaCommandType::Close => TradeAction::Close,
                            EaCommandType::Modify => TradeAction::Modify,
                            _ => TradeAction::Open,
                        };

                        let comment_raw = &cmd.comment;
                        let end_comment = comment_raw
                            .iter()
                            .position(|&x| x == 0)
                            .unwrap_or(comment_raw.len());
                        let comment_str =
                            String::from_utf8_lossy(&comment_raw[..end_comment]).to_string();
                        let comment = if comment_str.is_empty() {
                            None
                        } else {
                            Some(comment_str)
                        };

                        // New: Source Account from EaCommand
                        let src_bytes = &cmd.source_account;
                        let end_src = src_bytes
                            .iter()
                            .position(|&x| x == 0)
                            .unwrap_or(src_bytes.len());
                        let source_account =
                            String::from_utf8_lossy(&src_bytes[..end_src]).to_string();

                        let signal = TradeSignal {
                            action,
                            ticket: cmd.ticket,
                            symbol: Some(symbol),
                            order_type: crate::types::OrderType::from_mql(cmd.order_type),
                            lots: Some(cmd.volume),
                            open_price: Some(cmd.price),
                            stop_loss: if cmd.sl.abs() < 1e-6 {
                                None
                            } else {
                                Some(cmd.sl)
                            },
                            take_profit: if cmd.tp.abs() < 1e-6 {
                                None
                            } else {
                                Some(cmd.tp)
                            },
                            magic_number: Some(cmd.magic),
                            comment,
                            timestamp: chrono::DateTime::from_timestamp(cmd.timestamp, 0)
                                .unwrap_or_else(chrono::Utc::now),
                            source_account, // Used new field
                            close_ratio: if cmd.close_ratio.abs() < 1e-6 {
                                None
                            } else {
                                Some(cmd.close_ratio)
                            },
                        };
                        self.received_trade_signals.lock().unwrap().push(signal);
                    }
                    _ => {}
                }
            }
        }
    }
}

// =============================================================================
// Slave EA Simulator (Facade)
// =============================================================================

pub struct SlaveEaSimulator {
    base: EaSimulatorBase,
    master_account: String,

    // --- MQL5 Global Variables ---
    g_last_heartbeat: Arc<Mutex<Option<Instant>>>,
    g_config_requested: Arc<AtomicBool>,
    g_last_trade_allowed: Arc<AtomicBool>,
    g_has_received_config: Arc<AtomicBool>,
    g_configs: Arc<Mutex<Vec<SlaveConfig>>>,
    last_received_status: Arc<AtomicI32>,

    // --- State ---
    g_register_sent: Arc<AtomicBool>,

    // Platform Runner & Timer Thread
    runner: Option<PlatformRunner>,
    timer_thread: Option<JoinHandle<()>>,

    // --- Received Data Queues (Verification) ---
    received_trade_signals: Arc<Mutex<Vec<TradeSignal>>>,
    received_position_snapshots: Arc<Mutex<Vec<PositionSnapshotMessage>>>,
    received_vlogs_configs: Arc<Mutex<Vec<GlobalConfigMessage>>>,

    // --- Subscription Management ---
    subscribed_masters: Arc<Mutex<Vec<String>>>,
    pending_subscriptions: Arc<Mutex<Vec<String>>>,

    // --- Context (Managed in OnTimer thread) ---
    context: Arc<Mutex<Option<SlaveContextWrapper>>>,

    // Connection Params
    push_address: String,
    config_address: String,
}

impl SlaveEaSimulator {
    pub fn new(
        push_address: &str,
        config_address: &str,
        _trade_address: &str,
        account_id: &str,
        master_account: &str,
        is_trade_allowed: bool,
    ) -> Result<Self> {
        let base = EaSimulatorBase::new_without_zmq(account_id, EaType::Slave, is_trade_allowed)?;

        Ok(Self {
            base,
            master_account: master_account.to_string(),
            g_last_heartbeat: Arc::new(Mutex::new(None)),
            g_config_requested: Arc::new(AtomicBool::new(false)),
            g_last_trade_allowed: Arc::new(AtomicBool::new(false)),
            g_has_received_config: Arc::new(AtomicBool::new(false)),
            g_configs: Arc::new(Mutex::new(Vec::new())),
            last_received_status: Arc::new(AtomicI32::new(STATUS_NO_CONFIG)),
            runner: None,
            timer_thread: None,
            subscribed_masters: Arc::new(Mutex::new(Vec::new())),
            received_trade_signals: Arc::new(Mutex::new(Vec::new())),
            received_position_snapshots: Arc::new(Mutex::new(Vec::new())),
            received_vlogs_configs: Arc::new(Mutex::new(Vec::new())),
            g_register_sent: Arc::new(AtomicBool::new(false)),
            pending_subscriptions: Arc::new(Mutex::new(Vec::new())),
            context: Arc::new(Mutex::new(None)),
            push_address: push_address.to_string(),
            config_address: config_address.to_string(),
        })
    }

    pub fn start(&mut self) -> Result<()> {
        if self.runner.is_some() {
            return Ok(());
        }

        let core = SlaveEaCore {
            account_id: self.base.account_id().to_string(),
            ea_type: self.base.ea_type,
            heartbeat_params: self.base.heartbeat_params.clone(),
            _shutdown_flag: self.base.shutdown_flag.clone(),
            is_trade_allowed: self.base.is_trade_allowed_arc(),
            _master_account: self.master_account.clone(),
            _g_last_heartbeat: self.g_last_heartbeat.clone(),
            _g_config_requested: self.g_config_requested.clone(),
            _g_last_trade_allowed: self.g_last_trade_allowed.clone(),
            _g_has_received_config: self.g_has_received_config.clone(),
            g_configs: self.g_configs.clone(),
            last_received_status: self.last_received_status.clone(),
            _g_register_sent: self.g_register_sent.clone(),
            received_trade_signals: self.received_trade_signals.clone(),
            received_position_snapshots: self.received_position_snapshots.clone(),
            _received_vlogs_configs: self.received_vlogs_configs.clone(),
            _subscribed_masters: self.subscribed_masters.clone(),
            pending_subscriptions: self.pending_subscriptions.clone(),
            context: self.context.clone(),
            push_address: self.push_address.clone(),
            config_address: self.config_address.clone(),
        };

        let runner = PlatformRunner::new(core);
        let sender = runner.get_sender();

        self.runner = Some(runner);

        // Wait for initialization
        let start = std::time::Instant::now();
        while start.elapsed().as_secs() < 5 {
            if self.context.lock().unwrap().is_some() {
                // Initialized. Start timer thread.
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

    pub fn stop(&mut self) -> Result<()> {
        self.base.shutdown_flag.store(true, Ordering::SeqCst);
        if let Some(mut runner) = self.runner.take() {
            runner.stop(ENUM_DEINIT_REASON::REASON_REMOVE);
        }
        if let Some(handle) = self.timer_thread.take() {
            let _ = handle.join();
        }
        Ok(())
    }

    // =========================================================================
    // Helpers & Test Methods
    // =========================================================================

    pub fn send_unregister(&self) -> Result<()> {
        let msg = UnregisterMessage {
            message_type: "Unregister".to_string(),
            account_id: self.base.account_id().to_string(),
            timestamp: Utc::now().timestamp_millis(),
            ea_type: Some("Slave".to_string()),
        };
        let bytes = rmp_serde::to_vec_named(&msg)?;
        self.send_raw_bytes(&bytes)
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
            // Fallback if context not initialized (e.g. tests calling before start usually fail, but for completeness)
            Err(anyhow::anyhow!("Context not initialized"))
        }
    }

    pub fn get_status(&self) -> i32 {
        self.last_received_status.load(Ordering::SeqCst)
    }

    pub fn has_received_config(&self) -> bool {
        self.g_has_received_config.load(Ordering::SeqCst)
    }

    pub fn account_id(&self) -> &str {
        self.base.account_id()
    }

    pub fn master_account(&self) -> &str {
        &self.master_account
    }

    pub fn set_trade_allowed(&self, allowed: bool) {
        self.base.set_trade_allowed(allowed);
    }

    pub fn is_trade_allowed(&self) -> bool {
        self.base.is_trade_allowed()
    }

    pub fn wait_for_status(&self, expected: i32, timeout_ms: i32) -> Result<Option<SlaveConfig>> {
        let start = std::time::Instant::now();
        while start.elapsed().as_millis() < timeout_ms as u128 {
            if self.last_received_status.load(Ordering::SeqCst) == expected {
                let configs = self.g_configs.lock().unwrap();
                return Ok(configs.last().cloned());
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        Ok(None)
    }

    pub fn wait_for_config(&self, timeout_ms: i32) -> Result<Option<SlaveConfig>> {
        let start = std::time::Instant::now();
        while start.elapsed().as_millis() < timeout_ms as u128 {
            if self.g_has_received_config.load(Ordering::SeqCst) {
                let configs = self.g_configs.lock().unwrap();
                return Ok(configs.last().cloned());
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        Ok(None)
    }

    pub fn get_configs(&self) -> Vec<SlaveConfig> {
        self.g_configs.lock().unwrap().clone()
    }

    pub fn try_receive_trade_signal(&self, timeout_ms: i32) -> Result<Option<TradeSignal>> {
        let start = std::time::Instant::now();
        while start.elapsed().as_millis() < timeout_ms as u128 {
            let mut lock = self.received_trade_signals.lock().unwrap();
            if !lock.is_empty() {
                return Ok(Some(lock.remove(0)));
            }
            drop(lock);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        Ok(None)
    }

    pub fn try_receive_specific_signal(
        &self,
        expected: TradeAction,
        timeout_ms: i32,
    ) -> Result<Option<TradeSignal>> {
        let start = std::time::Instant::now();
        while start.elapsed().as_millis() < timeout_ms as u128 {
            // Instead of calling try_receive which consumes, we might peeking?
            // But simple implementation consumes. If multiple signals arrive, this might skip mismatching ones.
            // For test simplicity, assume sequential checks.
            if let Some(signal) = self.try_receive_trade_signal(10)? {
                if signal.action == expected {
                    return Ok(Some(signal));
                }
                // Discard mismatching signal in this simple implementation
            }
        }
        Ok(None)
    }

    pub fn wait_for_trade_action(
        &self,
        expected: TradeAction,
        timeout_ms: i32,
    ) -> Result<Option<TradeSignal>> {
        let start = std::time::Instant::now();
        while start.elapsed().as_millis() < timeout_ms as u128 {
            // Instead of calling try_receive which consumes, we might peeking?
            // But simple implementation consumes. If multiple signals arrive, this might skip mismatching ones.
            // For test simplicity, assume sequential checks.
            if let Some(signal) = self.try_receive_trade_signal(10)? {
                if signal.action == expected {
                    return Ok(Some(signal));
                }
                // Discard mismatching signal in this simple implementation
            }
        }
        Ok(None)
    }

    pub fn get_received_trade_signals(&self) -> Vec<TradeSignal> {
        self.received_trade_signals.lock().unwrap().clone()
    }

    pub fn send_sync_request(&self, last_sync_time: Option<String>) -> Result<()> {
        let guard = self.context.lock().unwrap();
        let wrapper = match guard.as_ref() {
            Some(w) => w,
            None => return Err(anyhow::anyhow!("Context not initialized")),
        };
        let ctx = wrapper.raw();

        let to_u16 = |s: &str| -> Vec<u16> { s.encode_utf16().chain(Some(0)).collect() };

        let master_u16 = to_u16(&self.master_account);
        // Handle optional last_sync_time
        let last_sync_u16_vec = last_sync_time.as_ref().map(|s| to_u16(s));
        let last_sync_ptr = match last_sync_u16_vec.as_ref() {
            Some(v) => v.as_ptr(),
            None => std::ptr::null(),
        };

        let ret = unsafe { ea_send_sync_request(ctx, master_u16.as_ptr(), last_sync_ptr) };

        if ret != 1 {
            return Err(anyhow::anyhow!("Failed to send sync request"));
        }

        Ok(())
    }

    pub fn try_receive_position_snapshot(
        &self,
        timeout_ms: i32,
    ) -> Result<Option<PositionSnapshotMessage>> {
        let start = std::time::Instant::now();
        while start.elapsed().as_millis() < timeout_ms as u128 {
            let mut lock = self.received_position_snapshots.lock().unwrap();
            if !lock.is_empty() {
                return Ok(Some(lock.remove(0)));
            }
            drop(lock);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        Ok(None)
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

    pub fn subscribe_to_sync_topic(&self) -> Result<()> {
        let sync_topic = format!("sync/{}/{}", self.master_account, self.base.account_id());
        self.pending_subscriptions.lock().unwrap().push(sync_topic);
        Ok(())
    }

    #[deprecated(note = "Trade topic subscription is now automatic on config reception")]
    pub fn subscribe_to_master(&self, master_account: &str) -> Result<()> {
        // Manual subscription helper for backward compatibility
        // We construct the topic and push to pending
        let to_u16 = |s: &str| -> Vec<u16> { s.encode_utf16().chain(Some(0)).collect() };
        let master_u16 = to_u16(master_account);
        let slave_u16 = to_u16(self.account_id());
        let mut topic_buf = vec![0u16; 256];
        unsafe {
            build_trade_topic(
                master_u16.as_ptr(),
                slave_u16.as_ptr(),
                topic_buf.as_mut_ptr(),
                256,
            );
        }
        let trade_topic = String::from_utf16_lossy(&topic_buf)
            .trim_end_matches('\0')
            .to_string();
        self.pending_subscriptions.lock().unwrap().push(trade_topic);
        Ok(())
    }
}

impl Drop for SlaveEaSimulator {
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
