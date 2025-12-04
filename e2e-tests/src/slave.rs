// e2e-tests/src/slave.rs
//
// Slave EA Simulator - MQL5 SankeyCopierSlave.mq5 完全準拠実装
//
// このSimulatorはMQL5 EAの実装を忠実に再現します:
// - OnInit(): ZMQ接続、トピック購読
// - OnTimer(): Heartbeat判定、RequestConfig送信、Config受信
// - グローバル変数: g_initialized, g_last_heartbeat, g_config_requested, etc.
//
// 外部からの操作は禁止。読み取り専用の観測のみ許可。
// - get_status(), wait_for_status(), has_received_config()

use anyhow::Result;
use chrono::Utc;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Instant;

use crate::base::EaSimulatorBase;
use crate::types::{
    EaType, PositionSnapshotMessage, RequestConfigMessage, SlaveConfig, SyncRequestMessage,
    TradeSignalMessage, VLogsConfigMessage, HEARTBEAT_INTERVAL_SECONDS, ONTIMER_INTERVAL_MS,
    STATUS_NO_CONFIG,
};

// =============================================================================
// Slave EA Simulator
// =============================================================================

/// Slave EA Simulator - MQL5 SankeyCopierSlave.mq5 完全準拠
///
/// ## MQL5グローバル変数対応表
/// | MQL5 | Rust | 説明 |
/// |------|------|------|
/// | g_initialized | ontimer_thread.is_some() | 初期化完了 |
/// | g_last_heartbeat | g_last_heartbeat | 最後のHeartbeat送信時刻 |
/// | g_config_requested | g_config_requested | Config要求送信済み |
/// | g_last_trade_allowed | g_last_trade_allowed | 前回のauto-trading状態 |
/// | g_has_received_config | g_has_received_config | Config受信済み |
/// | g_configs[] | g_configs | 受信したConfig配列 |
///
/// ## ライフサイクル
/// 1. new() - ZMQ接続 (OnInit相当)
/// 2. set_trade_allowed(true) - auto-trading ON
/// 3. start() - OnTimerスレッド開始 (EventSetMillisecondTimer相当)
/// 4. drop() - クリーンアップ (OnDeinit相当)
pub struct SlaveEaSimulator {
    base: EaSimulatorBase,
    master_account: String,

    // =========================================================================
    // MQL5 グローバル変数 (L48-67)
    // =========================================================================
    /// MQL5: g_last_heartbeat (datetime → Instant)
    g_last_heartbeat: Arc<Mutex<Option<Instant>>>,

    /// MQL5: g_config_requested
    g_config_requested: Arc<AtomicBool>,

    /// MQL5: g_last_trade_allowed (初期値 false)
    g_last_trade_allowed: Arc<AtomicBool>,

    /// MQL5: g_has_received_config
    g_has_received_config: Arc<AtomicBool>,

    /// MQL5: g_configs[] - 受信したSlaveConfig (最新のもの)
    g_configs: Arc<Mutex<Vec<SlaveConfig>>>,

    /// 最後に受信したステータス (STATUS_NO_CONFIG, DISABLED, ENABLED, CONNECTED)
    last_received_status: Arc<AtomicI32>,

    /// OnTimerスレッドハンドル
    ontimer_thread: Option<JoinHandle<()>>,

    /// 購読済みMasterアカウント (trade topic動的購読用)
    subscribed_masters: Arc<Mutex<Vec<String>>>,

    /// 受信したTradeSignalキュー (テスト用)
    received_trade_signals: Arc<Mutex<Vec<TradeSignalMessage>>>,

    /// 受信したPositionSnapshotキュー (テスト用)
    /// MQL5: OnTimerでconfig_socketから受信し、ProcessPositionSnapshot()で処理
    received_position_snapshots: Arc<Mutex<Vec<PositionSnapshotMessage>>>,

    /// 受信したVLogsConfigキュー (テスト用)
    /// MQL5: OnTimerでconfig_socketから受信し、ProcessVLogsConfig()で処理
    received_vlogs_configs: Arc<Mutex<Vec<VLogsConfigMessage>>>,
}

impl SlaveEaSimulator {
    /// Create a new Slave EA simulator (OnInit相当)
    ///
    /// ZMQ接続を確立し、config topicを購読します。
    /// OnTimerスレッドはまだ開始されません。start()を呼び出してください。
    ///
    /// ## MQL5 Socket Architecture
    /// - `g_zmq_trade_socket` (SUB): Trade signals from server
    /// - `g_zmq_config_socket` (SUB): Config/VLogs/PositionSnapshot from server
    /// Both connect to the same unified PUB address (port 5556).
    pub fn new(
        push_address: &str,
        config_address: &str,
        trade_address: &str,
        account_id: &str,
        master_account: &str,
    ) -> Result<Self> {
        // Slave EA has both config_socket and trade_socket
        // MQL5: g_zmq_config_socket + g_zmq_trade_socket
        let base = EaSimulatorBase::new(
            push_address,
            config_address,
            Some(trade_address),
            account_id,
            EaType::Slave,
        )?;

        Ok(Self {
            base,
            master_account: master_account.to_string(),
            g_last_heartbeat: Arc::new(Mutex::new(None)),
            g_config_requested: Arc::new(AtomicBool::new(false)),
            g_last_trade_allowed: Arc::new(AtomicBool::new(false)), // MQL5と同じ初期値
            g_has_received_config: Arc::new(AtomicBool::new(false)),
            g_configs: Arc::new(Mutex::new(Vec::new())),
            last_received_status: Arc::new(AtomicI32::new(STATUS_NO_CONFIG)),
            ontimer_thread: None,
            subscribed_masters: Arc::new(Mutex::new(Vec::new())),
            received_trade_signals: Arc::new(Mutex::new(Vec::new())),
            received_position_snapshots: Arc::new(Mutex::new(Vec::new())),
            received_vlogs_configs: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// Start the OnTimer loop (EventSetMillisecondTimer相当)
    ///
    /// MQL5: EventSetMillisecondTimer(SignalPollingIntervalMs) at L186
    /// バックグラウンドスレッドでOnTimerループを開始します。
    pub fn start(&mut self) -> Result<()> {
        if self.ontimer_thread.is_some() {
            return Ok(()); // Already started
        }

        // Clone Arcs for the thread
        let push_socket = self.base.push_socket_handle;
        let config_socket = self.base.config_socket_handle;
        // MQL5: g_zmq_trade_socket - Slave EA always has trade_socket
        let trade_socket = self
            .base
            .trade_socket_handle()
            .expect("Slave EA must have trade_socket");
        let account_id = self.base.account_id().to_string();
        let _master_account = self.master_account.clone(); // Reserved for future use
        let shutdown_flag = self.base.shutdown_flag.clone();
        let is_trade_allowed = self.base.is_trade_allowed_arc();
        let heartbeat_params = self.base.heartbeat_params.clone();
        let ea_type = self.base.ea_type;

        let g_last_heartbeat = self.g_last_heartbeat.clone();
        let g_config_requested = self.g_config_requested.clone();
        let g_last_trade_allowed = self.g_last_trade_allowed.clone();
        let g_has_received_config = self.g_has_received_config.clone();
        let g_configs = self.g_configs.clone();
        let last_received_status = self.last_received_status.clone();
        let subscribed_masters = self.subscribed_masters.clone();
        let received_trade_signals = self.received_trade_signals.clone();
        let received_position_snapshots = self.received_position_snapshots.clone();
        let received_vlogs_configs = self.received_vlogs_configs.clone();

        let handle = std::thread::spawn(move || {
            // MQL5: OnTimer() loop
            while !shutdown_flag.load(Ordering::SeqCst) {
                // Sleep for ONTIMER_INTERVAL_MS (100ms)
                std::thread::sleep(std::time::Duration::from_millis(ONTIMER_INTERVAL_MS));

                if shutdown_flag.load(Ordering::SeqCst) {
                    break;
                }

                // =============================================================
                // MQL5 OnTimer() L234-418 準拠
                // =============================================================

                // 1. ProcessTradeSignals() (MQL5 L244)
                // MQL5: zmq_socket_receive(g_zmq_trade_socket, trade_buffer, ...)
                // Trade signals are received on trade_socket and queued for test access
                {
                    let mut buffer = vec![0u8; crate::types::BUFFER_SIZE];
                    let trade_bytes = unsafe {
                        sankey_copier_zmq::ffi::zmq_socket_receive(
                            trade_socket,
                            buffer.as_mut_ptr() as *mut std::ffi::c_char,
                            crate::types::BUFFER_SIZE as i32,
                        )
                    };

                    if trade_bytes > 0 {
                        let bytes = &buffer[..trade_bytes as usize];
                        if let Some(space_pos) = bytes.iter().position(|&b| b == b' ') {
                            let topic = String::from_utf8_lossy(&bytes[..space_pos]).to_string();
                            let payload = &bytes[space_pos + 1..];

                            // Only process trade signals (MQL5: ProcessTradeSignal)
                            if topic.starts_with("trade/") {
                                if let Ok(signal) =
                                    rmp_serde::from_slice::<TradeSignalMessage>(payload)
                                {
                                    let mut signals = received_trade_signals.lock().unwrap();
                                    signals.push(signal);
                                }
                            }
                        }
                    }
                }

                // 2. Auto-trading状態変化の検出 (MQL5 L246-248)
                let current_trade_allowed = is_trade_allowed.load(Ordering::SeqCst);
                let last_trade_allowed = g_last_trade_allowed.load(Ordering::SeqCst);
                let trade_state_changed = current_trade_allowed != last_trade_allowed;

                // 3. Heartbeat判定 (MQL5 L251-252)
                let now = Instant::now();
                let last_hb = g_last_heartbeat.lock().unwrap().clone();
                let should_send_heartbeat = match last_hb {
                    None => true, // 最初のHeartbeat
                    Some(last) => {
                        now.duration_since(last).as_secs() >= HEARTBEAT_INTERVAL_SECONDS
                            || trade_state_changed
                    }
                };

                // 4. Heartbeat送信 (MQL5 L254-317)
                if should_send_heartbeat {
                    // SendHeartbeatMessage (MQL5 L257)
                    let msg = sankey_copier_zmq::HeartbeatMessage {
                        message_type: "Heartbeat".to_string(),
                        account_id: account_id.clone(),
                        balance: heartbeat_params.balance,
                        equity: heartbeat_params.equity,
                        open_positions: 0,
                        timestamp: Utc::now().to_rfc3339(),
                        version: heartbeat_params.version.clone(),
                        ea_type: ea_type.as_str().to_string(),
                        platform: "MT5".to_string(),
                        account_number: heartbeat_params.account_number,
                        broker: "TestBroker".to_string(),
                        account_name: heartbeat_params.account_name.clone(),
                        server: "TestServer".to_string(),
                        currency: "USD".to_string(),
                        leverage: heartbeat_params.leverage,
                        is_trade_allowed: current_trade_allowed,
                        symbol_prefix: None,
                        symbol_suffix: None,
                        symbol_map: None,
                    };

                    if let Ok(bytes) = rmp_serde::to_vec_named(&msg) {
                        let heartbeat_sent = unsafe {
                            sankey_copier_zmq::ffi::zmq_socket_send_binary(
                                push_socket,
                                bytes.as_ptr(),
                                bytes.len() as i32,
                            ) == 1
                        };

                        if heartbeat_sent {
                            // 更新 g_last_heartbeat (MQL5 L262)
                            *g_last_heartbeat.lock().unwrap() = Some(Instant::now());

                            // 状態変化時の処理 (MQL5 L265-293)
                            if trade_state_changed {
                                g_last_trade_allowed.store(current_trade_allowed, Ordering::SeqCst);

                                // Auto-trading有効化時にconfig要求 (MQL5 L285-293)
                                if current_trade_allowed
                                    && !g_config_requested.load(Ordering::SeqCst)
                                {
                                    let req_msg = RequestConfigMessage {
                                        message_type: "RequestConfig".to_string(),
                                        account_id: account_id.clone(),
                                        ea_type: "Slave".to_string(),
                                        timestamp: Utc::now().to_rfc3339(),
                                    };
                                    if let Ok(req_bytes) = rmp_serde::to_vec_named(&req_msg) {
                                        unsafe {
                                            if sankey_copier_zmq::ffi::zmq_socket_send_binary(
                                                push_socket,
                                                req_bytes.as_ptr(),
                                                req_bytes.len() as i32,
                                            ) == 1
                                            {
                                                g_config_requested.store(true, Ordering::SeqCst);
                                            }
                                        }
                                    }
                                }
                            } else {
                                // 通常heartbeat時にconfig要求 (MQL5 L297-309)
                                if !g_config_requested.load(Ordering::SeqCst) {
                                    let req_msg = RequestConfigMessage {
                                        message_type: "RequestConfig".to_string(),
                                        account_id: account_id.clone(),
                                        ea_type: "Slave".to_string(),
                                        timestamp: Utc::now().to_rfc3339(),
                                    };
                                    if let Ok(req_bytes) = rmp_serde::to_vec_named(&req_msg) {
                                        unsafe {
                                            if sankey_copier_zmq::ffi::zmq_socket_send_binary(
                                                push_socket,
                                                req_bytes.as_ptr(),
                                                req_bytes.len() as i32,
                                            ) == 1
                                            {
                                                g_config_requested.store(true, Ordering::SeqCst);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // 5. Config受信 (MQL5 L320-418) - Non-blocking
                let mut buffer = vec![0u8; crate::types::BUFFER_SIZE];
                let received_bytes = unsafe {
                    sankey_copier_zmq::ffi::zmq_socket_receive(
                        config_socket,
                        buffer.as_mut_ptr() as *mut std::ffi::c_char,
                        crate::types::BUFFER_SIZE as i32,
                    )
                };

                if received_bytes > 0 {
                    let bytes = &buffer[..received_bytes as usize];

                    // Parse topic + space + payload (MQL5 L326-337)
                    if let Some(space_pos) = bytes.iter().position(|&b| b == b' ') {
                        let _topic = String::from_utf8_lossy(&bytes[..space_pos]).to_string();
                        let payload = &bytes[space_pos + 1..];

                        // Try to parse as SlaveConfig (MQL5 L365-377)
                        if let Ok(config) = rmp_serde::from_slice::<SlaveConfig>(payload) {
                            // Update g_configs (MQL5: ProcessConfigMessage)
                            {
                                let mut configs = g_configs.lock().unwrap();
                                // Replace or add config for this master
                                if let Some(existing) = configs
                                    .iter_mut()
                                    .find(|c| c.master_account == config.master_account)
                                {
                                    *existing = config.clone();
                                } else {
                                    configs.push(config.clone());
                                }
                            }

                            // Update status
                            last_received_status.store(config.status, Ordering::SeqCst);
                            g_has_received_config.store(true, Ordering::SeqCst);

                            // 動的にtrade topicを購読 (ProcessConfigMessage内の動作)
                            // MQL5: SubscribeToTradeTopic(g_zmq_trade_socket, trade_topic)
                            // Trade topic is subscribed on trade_socket (not config_socket)
                            let master_acc = &config.master_account;
                            let mut subscribed = subscribed_masters.lock().unwrap();
                            if !subscribed.contains(master_acc) {
                                // Use FFI build_trade_topic (same as MQL5 EA)
                                let master_utf16: Vec<u16> =
                                    master_acc.encode_utf16().chain(Some(0)).collect();
                                let slave_utf16: Vec<u16> =
                                    account_id.encode_utf16().chain(Some(0)).collect();
                                let mut topic_utf16 = vec![0u16; 256];
                                unsafe {
                                    sankey_copier_zmq::ffi::build_trade_topic(
                                        master_utf16.as_ptr(),
                                        slave_utf16.as_ptr(),
                                        topic_utf16.as_mut_ptr(),
                                        256,
                                    );
                                }
                                unsafe {
                                    // Subscribe on trade_socket (MQL5: g_zmq_trade_socket)
                                    sankey_copier_zmq::ffi::zmq_socket_subscribe(
                                        trade_socket,
                                        topic_utf16.as_ptr(),
                                    );
                                }
                                subscribed.push(master_acc.clone());
                            }
                        } else if let Ok(snapshot) =
                            rmp_serde::from_slice::<PositionSnapshotMessage>(payload)
                        {
                            // MQL5: ProcessPositionSnapshot() - キューに格納
                            let mut snapshots = received_position_snapshots.lock().unwrap();
                            snapshots.push(snapshot);
                        } else if let Ok(vlogs_config) =
                            rmp_serde::from_slice::<VLogsConfigMessage>(payload)
                        {
                            // MQL5: ProcessVLogsConfig() - キューに格納
                            let mut configs = received_vlogs_configs.lock().unwrap();
                            configs.push(vlogs_config);
                        }
                    }
                }
            }
        });

        self.ontimer_thread = Some(handle);
        Ok(())
    }

    // =========================================================================
    // Read-only observation methods (外部から呼び出し可能)
    // =========================================================================

    /// Get the last received status from server config
    ///
    /// Returns STATUS_NO_CONFIG (-1) if no config has been received yet.
    /// MQL5: g_configs[].status
    pub fn get_status(&self) -> i32 {
        self.last_received_status.load(Ordering::SeqCst)
    }

    /// Check if at least one config has been received from server
    ///
    /// MQL5: g_has_received_config
    pub fn has_received_config(&self) -> bool {
        self.g_has_received_config.load(Ordering::SeqCst)
    }

    /// Get account ID
    pub fn account_id(&self) -> &str {
        self.base.account_id()
    }

    /// Get master account
    pub fn master_account(&self) -> &str {
        &self.master_account
    }

    /// Set is_trade_allowed state (simulates MT4/MT5 auto-trading toggle)
    ///
    /// MQL5: TerminalInfoInteger(TERMINAL_TRADE_ALLOWED)
    /// Changing this value triggers a heartbeat on the next OnTimer cycle.
    pub fn set_trade_allowed(&self, allowed: bool) {
        self.base.set_trade_allowed(allowed);
    }

    /// Get is_trade_allowed state
    pub fn is_trade_allowed(&self) -> bool {
        self.base.is_trade_allowed()
    }

    /// Wait for config with specific status value (test helper)
    ///
    /// This polls the internal status at short intervals until the expected
    /// status is received or timeout occurs.
    pub fn wait_for_status(
        &self,
        expected_status: i32,
        timeout_ms: i32,
    ) -> Result<Option<SlaveConfig>> {
        let start = std::time::Instant::now();
        let timeout_duration = std::time::Duration::from_millis(timeout_ms as u64);

        while start.elapsed() < timeout_duration {
            let current_status = self.last_received_status.load(Ordering::SeqCst);
            if current_status == expected_status {
                // Return the latest config
                let configs = self.g_configs.lock().unwrap();
                return Ok(configs.last().cloned());
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        Ok(None)
    }

    /// Wait for any config to be received (test helper)
    pub fn wait_for_config(&self, timeout_ms: i32) -> Result<Option<SlaveConfig>> {
        let start = std::time::Instant::now();
        let timeout_duration = std::time::Duration::from_millis(timeout_ms as u64);

        while start.elapsed() < timeout_duration {
            if self.g_has_received_config.load(Ordering::SeqCst) {
                let configs = self.g_configs.lock().unwrap();
                return Ok(configs.last().cloned());
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        Ok(None)
    }

    /// Get all received configs (test helper)
    pub fn get_configs(&self) -> Vec<SlaveConfig> {
        self.g_configs.lock().unwrap().clone()
    }

    // =========================================================================
    // Trade Signal Reception (OnTick相当 - テスト用外部メソッド)
    // =========================================================================

    /// Try to receive a trade signal with timeout (test helper)
    ///
    /// Trade signals are received by the OnTimer thread via ProcessTradeSignals()
    /// and queued in received_trade_signals. This method reads from that queue.
    ///
    /// MQL5: ProcessTradeSignals() receives on g_zmq_trade_socket
    pub fn try_receive_trade_signal(&self, timeout_ms: i32) -> Result<Option<TradeSignalMessage>> {
        let start = std::time::Instant::now();
        let timeout_duration = std::time::Duration::from_millis(timeout_ms as u64);

        while start.elapsed() < timeout_duration {
            // Check the queue for signals received by OnTimer thread
            {
                let mut signals = self.received_trade_signals.lock().unwrap();
                if !signals.is_empty() {
                    return Ok(Some(signals.remove(0)));
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        Ok(None)
    }

    /// Wait for a specific trade signal action (test helper)
    pub fn wait_for_trade_action(
        &self,
        expected_action: &str,
        timeout_ms: i32,
    ) -> Result<Option<TradeSignalMessage>> {
        let start = std::time::Instant::now();
        let timeout_duration = std::time::Duration::from_millis(timeout_ms as u64);

        while start.elapsed() < timeout_duration {
            if let Some(signal) = self.try_receive_trade_signal(100)? {
                if signal.action == expected_action {
                    return Ok(Some(signal));
                }
            }
        }
        Ok(None)
    }

    /// Get all received trade signals (for debugging/testing)
    pub fn get_received_trade_signals(&self) -> Vec<TradeSignalMessage> {
        self.received_trade_signals.lock().unwrap().clone()
    }

    // =========================================================================
    // Position Sync methods (テスト用)
    // =========================================================================

    /// Send a SyncRequest message to request position sync from master
    pub fn send_sync_request(&self, last_sync_time: Option<String>) -> Result<()> {
        let msg = SyncRequestMessage {
            message_type: "SyncRequest".to_string(),
            slave_account: self.base.account_id().to_string(),
            master_account: self.master_account.clone(),
            last_sync_time,
            timestamp: Utc::now().to_rfc3339(),
        };

        let bytes = rmp_serde::to_vec_named(&msg)?;
        self.base.send_binary(&bytes)
    }

    /// Try to receive a PositionSnapshot message with timeout
    ///
    /// PositionSnapshots are received by OnTimer thread on config_socket and
    /// queued in received_position_snapshots. This method reads from that queue.
    ///
    /// MQL5: ProcessPositionSnapshot() receives on g_zmq_config_socket
    pub fn try_receive_position_snapshot(
        &self,
        timeout_ms: i32,
    ) -> Result<Option<PositionSnapshotMessage>> {
        let start = std::time::Instant::now();
        let timeout_duration = std::time::Duration::from_millis(timeout_ms as u64);

        while start.elapsed() < timeout_duration {
            // Check the queue for snapshots received by OnTimer thread
            {
                let mut snapshots = self.received_position_snapshots.lock().unwrap();
                if !snapshots.is_empty() {
                    return Ok(Some(snapshots.remove(0)));
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        Ok(None)
    }

    /// Subscribe to position snapshots from master
    pub fn subscribe_to_position_snapshots(&self) -> Result<()> {
        let sync_topic = format!("sync/{}/{}", self.master_account, self.base.account_id());
        self.base.subscribe_to_topic(&sync_topic)
    }

    // =========================================================================
    // VLogs Config methods (テスト用)
    // =========================================================================

    /// Subscribe to global config topic for VLogs configuration
    pub fn subscribe_to_global_config(&self) -> Result<()> {
        self.base.subscribe_to_global_config()
    }

    /// Try to receive a VLogsConfigMessage from the global config topic
    ///
    /// VLogsConfigs are received by OnTimer thread on config_socket and
    /// queued in received_vlogs_configs. This method reads from that queue.
    ///
    /// MQL5: ProcessVLogsConfig() receives on g_zmq_config_socket
    pub fn try_receive_vlogs_config(&self, timeout_ms: i32) -> Result<Option<VLogsConfigMessage>> {
        let start = std::time::Instant::now();
        let timeout_duration = std::time::Duration::from_millis(timeout_ms as u64);

        while start.elapsed() < timeout_duration {
            // Check the queue for vlogs configs received by OnTimer thread
            {
                let mut configs = self.received_vlogs_configs.lock().unwrap();
                if !configs.is_empty() {
                    return Ok(Some(configs.remove(0)));
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        Ok(None)
    }

    // =========================================================================
    // Legacy methods for backward compatibility during migration
    // =========================================================================

    /// Subscribe to trade signals for a specific Master account
    ///
    /// Note: In the new implementation, this is done automatically when
    /// config is received. This method is kept for backward compatibility.
    #[deprecated(note = "Trade topic subscription is now automatic on config reception")]
    pub fn subscribe_to_master(&self, master_account: &str) -> Result<()> {
        // Use FFI build_trade_topic (same as MQL5 EA)
        let master_utf16: Vec<u16> = master_account.encode_utf16().chain(Some(0)).collect();
        let slave_utf16: Vec<u16> = self
            .base
            .account_id()
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let mut topic_buffer = vec![0u16; 256];
        let topic_len = unsafe {
            sankey_copier_zmq::ffi::build_trade_topic(
                master_utf16.as_ptr(),
                slave_utf16.as_ptr(),
                topic_buffer.as_mut_ptr(),
                256,
            )
        };
        if topic_len <= 0 {
            anyhow::bail!("Failed to build trade topic");
        }
        let trade_topic = String::from_utf16_lossy(&topic_buffer[..topic_len as usize]);
        self.base.subscribe_to_topic(&trade_topic)
    }
}

impl Drop for SlaveEaSimulator {
    fn drop(&mut self) {
        // Signal OnTimer thread to stop
        self.base.shutdown_flag.store(true, Ordering::SeqCst);

        // Wait for thread to finish
        if let Some(handle) = self.ontimer_thread.take() {
            let _ = handle.join();
        }
    }
}
