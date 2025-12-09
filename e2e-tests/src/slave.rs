// e2e-tests/src/slave.rs
//
// Slave EA Simulator - MQL5 SankeyCopierSlave.mq5 完全準拠実装
//
// Refactored to use EaContext via FFI.
// Now using the Platform Simulator infrastructure.

use anyhow::Result;
use chrono::Utc;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Instant;

use sankey_copier_zmq::ffi::{
    build_trade_topic, ea_connect, ea_context_free, ea_context_mark_config_requested,
    ea_context_should_request_config, ea_init, ea_receive_config, ea_send_heartbeat, ea_send_push,
    ea_send_register, ea_send_sync_request, ea_send_unregister, ea_subscribe_config,
};
use sankey_copier_zmq::EaContext;

use crate::base::EaSimulatorBase;
use crate::platform::runner::PlatformRunner;
use crate::platform::traits::ExpertAdvisor;
use crate::platform::types::{ENUM_DEINIT_REASON, ENUM_INIT_RETCODE};
use crate::types::{
    EaType, PositionSnapshotMessage, RequestConfigMessage, SlaveConfig, TradeAction, TradeSignal,
    UnregisterMessage, VLogsConfigMessage, HEARTBEAT_INTERVAL_SECONDS, ONTIMER_INTERVAL_MS,
    STATUS_NO_CONFIG,
};

// Wrapper for thread-safe passing of EaContext pointer
struct ContextWrapper(pub *mut EaContext);
unsafe impl Send for ContextWrapper {}
unsafe impl Sync for ContextWrapper {}

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

    g_last_heartbeat: Arc<Mutex<Option<Instant>>>,
    g_config_requested: Arc<AtomicBool>,
    g_last_trade_allowed: Arc<AtomicBool>,
    g_has_received_config: Arc<AtomicBool>,
    g_configs: Arc<Mutex<Vec<SlaveConfig>>>,
    last_received_status: Arc<AtomicI32>,

    g_register_sent: Arc<AtomicBool>,
    received_trade_signals: Arc<Mutex<Vec<TradeSignal>>>,
    received_position_snapshots: Arc<Mutex<Vec<PositionSnapshotMessage>>>,
    received_vlogs_configs: Arc<Mutex<Vec<VLogsConfigMessage>>>,

    subscribed_masters: Arc<Mutex<Vec<String>>>,
    pending_subscriptions: Arc<Mutex<Vec<String>>>,
    context: Arc<Mutex<Option<ContextWrapper>>>,

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

        let ctx = unsafe {
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

        if ctx.is_null() {
            eprintln!("Failed to initialize EA context!");
            return ENUM_INIT_RETCODE::INIT_FAILED;
        }

        {
            let mut guard = self.context.lock().unwrap();
            *guard = Some(ContextWrapper(ctx));
        }

        let push_u16 = to_u16(&self.push_address);
        let sub_u16 = to_u16(&self.config_address);

        unsafe {
            if ea_connect(ctx, push_u16.as_ptr(), sub_u16.as_ptr()) != 1 {
                eprintln!("Failed to connect EA context!");
                return ENUM_INIT_RETCODE::INIT_FAILED;
            }
        }

        ENUM_INIT_RETCODE::INIT_SUCCEEDED
    }

    fn on_deinit(&mut self, _reason: ENUM_DEINIT_REASON) {
        let mut guard = self.context.lock().unwrap();
        if let Some(wrapper) = guard.take() {
            let ctx = wrapper.0;
            let mut buffer = vec![0u8; 1024];
            let len = unsafe {
                ea_send_unregister(ctx, buffer.as_mut_ptr(), buffer.len() as i32)
            };
            if len > 0 {
                unsafe {
                    ea_send_push(ctx, buffer.as_ptr(), len);
                }
            }
            unsafe {
                ea_context_free(ctx);
            }
        }
    }

    fn on_timer(&mut self) {
        let guard = self.context.lock().unwrap();
        let ctx = match guard.as_ref() {
            Some(w) => w.0,
            None => return,
        };
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

        // Register (Once)
        if !self.g_register_sent.load(Ordering::SeqCst) {
            let mut buffer = vec![0u8; 1024];
            let len = unsafe { ea_send_register(ctx, buffer.as_mut_ptr(), buffer.len() as i32) };
            if len > 0 {
                unsafe {
                    ea_send_push(ctx, buffer.as_ptr(), len);
                }
                self.g_register_sent.store(true, Ordering::SeqCst);
            }
        }

        let current_trade_allowed = self.is_trade_allowed.load(Ordering::SeqCst);
        let last_trade_allowed_val = self.g_last_trade_allowed.load(Ordering::SeqCst);
        let trade_state_changed = current_trade_allowed != last_trade_allowed_val;

        let now = Instant::now();
        let last_hb = *self.g_last_heartbeat.lock().unwrap();
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
                    self.heartbeat_params.balance,
                    self.heartbeat_params.equity,
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

                *self.g_last_heartbeat.lock().unwrap() = Some(Instant::now());
                if trade_state_changed {
                    self.g_last_trade_allowed
                        .store(current_trade_allowed, Ordering::SeqCst);
                }

                let should_request = unsafe {
                    ea_context_should_request_config(ctx, if current_trade_allowed { 1 } else { 0 })
                };

                if should_request == 1 {
                    let req_msg = RequestConfigMessage {
                        message_type: "RequestConfig".to_string(),
                        account_id: self.account_id.clone(),
                        ea_type: "Slave".to_string(),
                        timestamp: Utc::now().to_rfc3339(),
                    };
                    if let Ok(req_bytes) = rmp_serde::to_vec_named(&req_msg) {
                        unsafe {
                            ea_send_push(ctx, req_bytes.as_ptr(), req_bytes.len() as i32);
                        }
                        self.g_config_requested.store(true, Ordering::SeqCst);
                    }
                }
            }
        }

        // Receive Loop (Config & Trade & Sync)
        loop {
            let mut buffer = vec![0u8; crate::types::BUFFER_SIZE];

            let received_bytes = unsafe {
                ea_receive_config(ctx, buffer.as_mut_ptr(), crate::types::BUFFER_SIZE as i32)
            };

            if received_bytes <= 0 {
                break;
            }

            let bytes = &buffer[..received_bytes as usize];
            if let Some(space_pos) = bytes.iter().position(|&b| b == b' ') {
                let topic = String::from_utf8_lossy(&bytes[..space_pos]).to_string();
                let payload = &bytes[space_pos + 1..];

                if topic.starts_with("trade/") {
                    match rmp_serde::from_slice::<TradeSignal>(payload) {
                        Ok(signal) => {
                            self.received_trade_signals.lock().unwrap().push(signal);
                        }
                        Err(e) => {
                            eprintln!("Failed to deserialize TradeSignal: {}", e);
                            eprintln!("Payload len: {}, bytes: {:?}", payload.len(), payload);
                        }
                    }
                } else if topic.starts_with("sync/") {
                    if let Ok(snapshot) = rmp_serde::from_slice::<PositionSnapshotMessage>(payload)
                    {
                        self.received_position_snapshots
                            .lock()
                            .unwrap()
                            .push(snapshot);
                    }
                } else if topic.starts_with("config/") {
                    if let Ok(config) = rmp_serde::from_slice::<SlaveConfig>(payload) {
                        {
                            let mut configs = self.g_configs.lock().unwrap();
                            if let Some(existing) = configs
                                .iter_mut()
                                .find(|c| c.master_account == config.master_account)
                            {
                                *existing = config.clone();
                            } else {
                                configs.push(config.clone());
                            }
                        }
                        self.last_received_status
                            .store(config.status, Ordering::SeqCst);
                        self.g_has_received_config.store(true, Ordering::SeqCst);
                        unsafe {
                            ea_context_mark_config_requested(ctx);
                        }

                        // Dynamic Trade/Sync Subscription logic
                        let master_acc = &config.master_account;
                        let mut subscribed = self.subscribed_masters.lock().unwrap();
                        if !subscribed.contains(master_acc) {
                            let master_u16 = to_u16(master_acc);
                            let slave_u16 = to_u16(&self.account_id);
                            let mut topic_buf = vec![0u16; 256];

                            unsafe {
                                build_trade_topic(
                                    master_u16.as_ptr(),
                                    slave_u16.as_ptr(),
                                    topic_buf.as_mut_ptr(),
                                    256,
                                );
                                ea_subscribe_config(ctx, topic_buf.as_ptr());
                            }

                            let sync_topic =
                                format!("sync/{}/{}", master_acc, self.account_id);
                            let sync_topic_u16 = to_u16(&sync_topic);
                            unsafe {
                                ea_subscribe_config(ctx, sync_topic_u16.as_ptr());
                            }

                            subscribed.push(master_acc.clone());
                        }
                    } else if let Ok(vlogs) = rmp_serde::from_slice::<VLogsConfigMessage>(payload) {
                        self.received_vlogs_configs.lock().unwrap().push(vlogs);
                    }
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
    received_vlogs_configs: Arc<Mutex<Vec<VLogsConfigMessage>>>,

    // --- Subscription Management ---
    subscribed_masters: Arc<Mutex<Vec<String>>>,
    pending_subscriptions: Arc<Mutex<Vec<String>>>,

    // --- Context (Managed in OnTimer thread) ---
    context: Arc<Mutex<Option<ContextWrapper>>>,

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
    ) -> Result<Self> {
        let base = EaSimulatorBase::new_without_zmq(account_id, EaType::Slave)?;

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
            g_last_heartbeat: self.g_last_heartbeat.clone(),
            g_config_requested: self.g_config_requested.clone(),
            g_last_trade_allowed: self.g_last_trade_allowed.clone(),
            g_has_received_config: self.g_has_received_config.clone(),
            g_configs: self.g_configs.clone(),
            last_received_status: self.last_received_status.clone(),
            g_register_sent: self.g_register_sent.clone(),
            received_trade_signals: self.received_trade_signals.clone(),
            received_position_snapshots: self.received_position_snapshots.clone(),
            received_vlogs_configs: self.received_vlogs_configs.clone(),
            subscribed_masters: self.subscribed_masters.clone(),
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
            timestamp: Utc::now().to_rfc3339(),
            ea_type: Some("Slave".to_string()),
        };
        let bytes = rmp_serde::to_vec_named(&msg)?;
        self.send_raw_bytes(&bytes)
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
        let ctx = match guard.as_ref() {
            Some(w) => w.0,
            None => return Err(anyhow::anyhow!("Context not initialized")),
        };

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
