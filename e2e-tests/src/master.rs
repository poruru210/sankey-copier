// e2e-tests/src/master.rs
//
// Master EA Simulator - MQL5 SankeyCopierMaster.mq5 完全準拠実装
//
// このSimulatorはMQL5 EAの実装を忠実に再現します:
// - OnInit(): ZMQ接続、トピック購読
// - OnTimer(): Heartbeat判定、RequestConfig送信、Config受信
// - グローバル変数: g_initialized, g_last_heartbeat, g_config_requested, etc.
//
// Trade送信メソッド (send_trade_signal, send_position_snapshot) は
// OnTick/OnTradeTransaction相当として外部から呼び出し可能。

use anyhow::Result;
use chrono::Utc;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Instant;

use crate::base::EaSimulatorBase;
use crate::types::{
    EaType, MasterConfigMessage, PositionInfo, PositionSnapshotMessage, RequestConfigMessage,
    SyncRequestMessage, TradeSignalMessage, VLogsConfigMessage, HEARTBEAT_INTERVAL_SECONDS,
    ONTIMER_INTERVAL_MS, STATUS_NO_CONFIG,
};

// =============================================================================
// Master EA Simulator
// =============================================================================

/// Master EA Simulator - MQL5 SankeyCopierMaster.mq5 完全準拠
///
/// ## MQL5グローバル変数対応表
/// | MQL5 | Rust | 説明 |
/// |------|------|------|
/// | g_initialized | ontimer_thread.is_some() | 初期化完了 |
/// | g_last_heartbeat | g_last_heartbeat | 最後のHeartbeat送信時刻 |
/// | g_config_requested | g_config_requested | Config要求送信済み |
/// | g_last_trade_allowed | g_last_trade_allowed | 前回のauto-trading状態 |
/// | g_server_status | g_server_status | サーバーからのステータス |
/// | g_symbol_prefix | g_symbol_prefix | シンボルプレフィックス |
/// | g_symbol_suffix | g_symbol_suffix | シンボルサフィックス |
pub struct MasterEaSimulator {
    base: EaSimulatorBase,

    // =========================================================================
    // MQL5 グローバル変数 (L46-67)
    // =========================================================================
    /// MQL5: g_last_heartbeat
    g_last_heartbeat: Arc<Mutex<Option<Instant>>>,

    /// MQL5: g_config_requested
    g_config_requested: Arc<AtomicBool>,

    /// MQL5: g_last_trade_allowed (初期値 false)
    g_last_trade_allowed: Arc<AtomicBool>,

    /// MQL5: g_server_status (STATUS_NO_CONFIG, DISABLED, CONNECTED)
    g_server_status: Arc<AtomicI32>,

    /// MQL5: g_symbol_prefix
    g_symbol_prefix: Arc<Mutex<String>>,

    /// MQL5: g_symbol_suffix
    g_symbol_suffix: Arc<Mutex<String>>,

    /// MQL5: g_config_version
    #[allow(dead_code)]
    g_config_version: Arc<Mutex<u32>>,

    /// 受信したSyncRequestキュー (テスト用)
    /// MQL5: OnTimerでconfig_socketから受信し、ProcessSyncRequest()で処理
    received_sync_requests: Arc<Mutex<Vec<SyncRequestMessage>>>,

    /// 受信したVLogsConfigキュー (テスト用)
    /// MQL5: OnTimerでconfig_socketから受信し、ProcessVLogsConfig()で処理
    received_vlogs_configs: Arc<Mutex<Vec<VLogsConfigMessage>>>,

    /// OnTimerスレッドハンドル
    ontimer_thread: Option<JoinHandle<()>>,
}

impl MasterEaSimulator {
    /// Create a new Master EA simulator (OnInit相当)
    ///
    /// ## MQL5 Socket Architecture (SankeyCopierMaster.mq5)
    /// - `g_zmq_socket` (PUSH): Send heartbeat, trade signals to server
    /// - `g_zmq_config_socket` (SUB): Receive config/vlogs_config from server
    /// - Master EA does NOT have trade_socket (only Slave receives trade signals)
    pub fn new(push_address: &str, config_address: &str, account_id: &str) -> Result<Self> {
        // Master EA: No trade_socket (only PUSH + config_socket)
        // MQL5: g_zmq_socket + g_zmq_config_socket
        let base = EaSimulatorBase::new(
            push_address,
            config_address,
            None, // No trade_address for Master EA
            account_id,
            EaType::Master,
        )?;

        Ok(Self {
            base,
            g_last_heartbeat: Arc::new(Mutex::new(None)),
            g_config_requested: Arc::new(AtomicBool::new(false)),
            g_last_trade_allowed: Arc::new(AtomicBool::new(false)), // MQL5と同じ初期値
            g_server_status: Arc::new(AtomicI32::new(STATUS_NO_CONFIG)),
            g_symbol_prefix: Arc::new(Mutex::new(String::new())),
            g_symbol_suffix: Arc::new(Mutex::new(String::new())),
            g_config_version: Arc::new(Mutex::new(0)),
            received_sync_requests: Arc::new(Mutex::new(Vec::new())),
            received_vlogs_configs: Arc::new(Mutex::new(Vec::new())),
            ontimer_thread: None,
        })
    }

    /// Start the OnTimer loop (EventSetTimer相当)
    ///
    /// MQL5: EventSetTimer(1) at L182 (1 second interval)
    /// Note: Simulator uses 100ms for faster test execution
    pub fn start(&mut self) -> Result<()> {
        if self.ontimer_thread.is_some() {
            return Ok(()); // Already started
        }

        // Clone Arcs for the thread
        let push_socket = self.base.push_socket_handle;
        let config_socket = self.base.config_socket_handle;
        let account_id = self.base.account_id().to_string();
        let shutdown_flag = self.base.shutdown_flag.clone();
        let is_trade_allowed = self.base.is_trade_allowed_arc();
        let heartbeat_params = self.base.heartbeat_params.clone();
        let ea_type = self.base.ea_type;

        let g_last_heartbeat = self.g_last_heartbeat.clone();
        let g_config_requested = self.g_config_requested.clone();
        let g_last_trade_allowed = self.g_last_trade_allowed.clone();
        let g_server_status = self.g_server_status.clone();
        let g_symbol_prefix = self.g_symbol_prefix.clone();
        let g_symbol_suffix = self.g_symbol_suffix.clone();
        let received_sync_requests = self.received_sync_requests.clone();
        let received_vlogs_configs = self.received_vlogs_configs.clone();

        let handle = std::thread::spawn(move || {
            // MQL5: OnTimer() loop
            while !shutdown_flag.load(Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_millis(ONTIMER_INTERVAL_MS));

                if shutdown_flag.load(Ordering::SeqCst) {
                    break;
                }

                // =============================================================
                // MQL5 OnTimer() L225-343 準拠
                // =============================================================

                // 1. Auto-trading状態変化の検出 (MQL5 L230-232)
                let current_trade_allowed = is_trade_allowed.load(Ordering::SeqCst);
                let last_trade_allowed = g_last_trade_allowed.load(Ordering::SeqCst);
                let trade_state_changed = current_trade_allowed != last_trade_allowed;

                // 2. Heartbeat判定 (MQL5 L235-236)
                let now = Instant::now();
                let last_hb = g_last_heartbeat.lock().unwrap().clone();
                let should_send_heartbeat = match last_hb {
                    None => true,
                    Some(last) => {
                        now.duration_since(last).as_secs() >= HEARTBEAT_INTERVAL_SECONDS
                            || trade_state_changed
                    }
                };

                // 3. Heartbeat送信 (MQL5 L238-269)
                if should_send_heartbeat {
                    let prefix = g_symbol_prefix.lock().unwrap().clone();
                    let suffix = g_symbol_suffix.lock().unwrap().clone();

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
                        symbol_prefix: if prefix.is_empty() {
                            None
                        } else {
                            Some(prefix)
                        },
                        symbol_suffix: if suffix.is_empty() {
                            None
                        } else {
                            Some(suffix)
                        },
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
                            *g_last_heartbeat.lock().unwrap() = Some(Instant::now());

                            if trade_state_changed {
                                g_last_trade_allowed.store(current_trade_allowed, Ordering::SeqCst);
                            }

                            // RequestConfig送信 (MQL5 L253-267)
                            // current_trade_allowed AND !g_config_requested の時のみ
                            if !g_config_requested.load(Ordering::SeqCst) && current_trade_allowed {
                                let req_msg = RequestConfigMessage {
                                    message_type: "RequestConfig".to_string(),
                                    account_id: account_id.clone(),
                                    ea_type: "Master".to_string(),
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

                // 4. Config受信 (MQL5 L271-343) - Non-blocking
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

                    if let Some(space_pos) = bytes.iter().position(|&b| b == b' ') {
                        let _topic = String::from_utf8_lossy(&bytes[..space_pos]).to_string();
                        let payload = &bytes[space_pos + 1..];

                        // Try to parse as MasterConfig (MQL5 L305-323)
                        if let Ok(config) = rmp_serde::from_slice::<MasterConfigMessage>(payload) {
                            // Update status and symbol settings
                            g_server_status.store(config.status, Ordering::SeqCst);
                            if let Some(prefix) = &config.symbol_prefix {
                                *g_symbol_prefix.lock().unwrap() = prefix.clone();
                            }
                            if let Some(suffix) = &config.symbol_suffix {
                                *g_symbol_suffix.lock().unwrap() = suffix.clone();
                            }
                        } else if let Ok(sync_req) =
                            rmp_serde::from_slice::<SyncRequestMessage>(payload)
                        {
                            // MQL5: ProcessSyncRequest() - キューに格納 (L325-340)
                            let mut requests = received_sync_requests.lock().unwrap();
                            requests.push(sync_req);
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
    // Read-only observation methods
    // =========================================================================

    /// Get account ID
    pub fn account_id(&self) -> &str {
        self.base.account_id()
    }

    /// Get server status
    pub fn get_server_status(&self) -> i32 {
        self.g_server_status.load(Ordering::SeqCst)
    }

    /// Set is_trade_allowed state
    pub fn set_trade_allowed(&self, allowed: bool) {
        self.base.set_trade_allowed(allowed);
    }

    /// Get is_trade_allowed state
    pub fn is_trade_allowed(&self) -> bool {
        self.base.is_trade_allowed()
    }

    /// Wait for server status (test helper)
    pub fn wait_for_status(
        &self,
        expected_status: i32,
        timeout_ms: i32,
    ) -> Result<Option<MasterConfigMessage>> {
        let start = std::time::Instant::now();
        let timeout_duration = std::time::Duration::from_millis(timeout_ms as u64);

        while start.elapsed() < timeout_duration {
            let current_status = self.g_server_status.load(Ordering::SeqCst);
            if current_status == expected_status {
                return Ok(Some(MasterConfigMessage {
                    account_id: self.base.account_id().to_string(),
                    status: current_status,
                    symbol_prefix: Some(self.g_symbol_prefix.lock().unwrap().clone()),
                    symbol_suffix: Some(self.g_symbol_suffix.lock().unwrap().clone()),
                    config_version: 0,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    warning_codes: vec![],
                }));
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        Ok(None)
    }

    // =========================================================================
    // TradeSignal methods (OnTick/OnTradeTransaction相当)
    // =========================================================================

    /// Send a TradeSignal message
    pub fn send_trade_signal(&self, signal: &TradeSignalMessage) -> Result<()> {
        let bytes = rmp_serde::to_vec_named(signal)?;
        self.base.send_binary(&bytes)
    }

    /// Create an Open signal
    #[allow(clippy::too_many_arguments)]
    pub fn create_open_signal(
        &self,
        ticket: i64,
        symbol: &str,
        order_type: &str,
        lots: f64,
        price: f64,
        sl: Option<f64>,
        tp: Option<f64>,
        magic: i64,
    ) -> TradeSignalMessage {
        TradeSignalMessage {
            action: "Open".to_string(),
            ticket,
            symbol: Some(symbol.to_string()),
            order_type: Some(order_type.to_string()),
            lots: Some(lots),
            open_price: Some(price),
            stop_loss: sl,
            take_profit: tp,
            magic_number: Some(magic),
            comment: Some("E2E Test".to_string()),
            timestamp: Utc::now().to_rfc3339(),
            source_account: self.base.account_id().to_string(),
            close_ratio: None,
        }
    }

    /// Create a Close signal (full close)
    pub fn create_close_signal(&self, ticket: i64, symbol: &str, lots: f64) -> TradeSignalMessage {
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
            source_account: self.base.account_id().to_string(),
            close_ratio: None,
        }
    }

    /// Create a Partial Close signal
    pub fn create_partial_close_signal(
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
            source_account: self.base.account_id().to_string(),
            close_ratio: Some(close_ratio),
        }
    }

    /// Create a Modify signal for SL/TP changes
    pub fn create_modify_signal(
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
            magic_number: None,
            comment: Some("E2E Test Modify".to_string()),
            timestamp: Utc::now().to_rfc3339(),
            source_account: self.base.account_id().to_string(),
            close_ratio: None,
        }
    }

    /// Create a delayed signal by setting timestamp to the past
    pub fn create_delayed_signal(
        &self,
        mut signal: TradeSignalMessage,
        delay_ms: i64,
    ) -> TradeSignalMessage {
        use chrono::Duration as ChronoDuration;
        let past_time = Utc::now() - ChronoDuration::milliseconds(delay_ms);
        signal.timestamp = past_time.to_rfc3339();
        signal
    }

    // =========================================================================
    // Position Sync methods
    // =========================================================================

    /// Send a PositionSnapshot message
    pub fn send_position_snapshot(&self, positions: Vec<PositionInfo>) -> Result<()> {
        let msg = PositionSnapshotMessage {
            message_type: "PositionSnapshot".to_string(),
            source_account: self.base.account_id().to_string(),
            positions,
            timestamp: Utc::now().to_rfc3339(),
        };

        let bytes = rmp_serde::to_vec_named(&msg)?;
        self.base.send_binary(&bytes)
    }

    /// Create a test position for PositionSnapshot
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

    /// Subscribe to sync requests for this master
    pub fn subscribe_to_sync_requests(&self) -> Result<()> {
        let sync_topic = format!("sync/{}", self.base.account_id());
        self.base.subscribe_to_topic(&sync_topic)
    }

    /// Try to receive a SyncRequest message with timeout
    ///
    /// SyncRequests are received by OnTimer thread on config_socket and
    /// queued in received_sync_requests. This method reads from that queue.
    ///
    /// MQL5: ProcessSyncRequest() receives on g_zmq_config_socket
    pub fn try_receive_sync_request(&self, timeout_ms: i32) -> Result<Option<SyncRequestMessage>> {
        let start = std::time::Instant::now();
        let timeout_duration = std::time::Duration::from_millis(timeout_ms as u64);

        while start.elapsed() < timeout_duration {
            // Check the queue for sync requests received by OnTimer thread
            {
                let mut requests = self.received_sync_requests.lock().unwrap();
                if !requests.is_empty() {
                    return Ok(Some(requests.remove(0)));
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        Ok(None)
    }

    // =========================================================================
    // Config methods (test helpers)
    // =========================================================================

    /// Try to receive a MasterConfigMessage with timeout (test helper)
    pub fn try_receive_master_config(
        &self,
        timeout_ms: i32,
    ) -> Result<Option<MasterConfigMessage>> {
        let start = std::time::Instant::now();
        let timeout_duration = std::time::Duration::from_millis(timeout_ms as u64);

        while start.elapsed() < timeout_duration {
            if let Some((_, payload)) = self.base.try_receive_raw_nonblocking()? {
                if let Ok(config) = rmp_serde::from_slice::<MasterConfigMessage>(&payload) {
                    return Ok(Some(config));
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        Ok(None)
    }

    // =========================================================================
    // VLogs Config methods
    // =========================================================================

    /// Subscribe to global config topic for VLogs configuration
    pub fn subscribe_to_global_config(&self) -> Result<()> {
        self.base.subscribe_to_global_config()
    }

    /// Try to receive a VLogsConfigMessage with timeout
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
}

impl Drop for MasterEaSimulator {
    fn drop(&mut self) {
        self.base.shutdown_flag.store(true, Ordering::SeqCst);

        if let Some(handle) = self.ontimer_thread.take() {
            let _ = handle.join();
        }
    }
}
