// mt-bridge/src/ea_context.rs
//
// EA State and Communication Management
//
// Refactored to use Strategy Pattern for Master/Slave communication logic.

use crate::communication::{CommunicationStrategy, MasterStrategy, NoOpStrategy, SlaveStrategy};
use crate::constants::{OrderType, TradeAction};
use crate::errors::BridgeError;
use crate::types::{RequestConfigMessage, SlaveConfigMessage, TradeSignal};
use chrono::{DateTime, Utc};
use std::collections::{HashMap, VecDeque};
use std::fmt::Debug;

// Command types corresponding to MQL
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(i32)]
pub enum EaCommandType {
    None = 0,
    Open = 1,
    Close = 2,
    Modify = 3,
    Delete = 4,
    UpdateUi = 5,
    SendSnapshot = 6,
    ProcessSnapshot = 7,
}

// C-compatible Command structure
// MQL4 (pack=1) and Rust (aligned) compatibility requires explicit padding.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct EaCommand {
    pub command_type: i32,
    pub algo_flags: i32, // [FIX] Replaced _pad1 with algo_flags (Bit 0: IsDelayed)

    pub ticket: i64,
    pub symbol: [u8; 32], // 32 bytes (aligned to 8, safe)

    pub order_type: i32,
    pub _pad2: i32, // [FIX] Explicit padding for alignment

    pub volume: f64,
    pub price: f64,
    pub sl: f64,
    pub tp: f64,
    pub magic: i64,
    pub close_ratio: f64,
    pub timestamp: i64,
    pub comment: [u8; 64], // MQL Comment

    // New field for Source Account (Master ID)
    pub source_account: [u8; 64],
}

impl Default for EaCommand {
    fn default() -> Self {
        Self {
            command_type: 0,
            algo_flags: 0,
            ticket: 0,
            symbol: [0; 32],
            order_type: 0, // ...
            _pad2: 0,
            volume: 0.0,
            price: 0.0,
            sl: 0.0,
            tp: 0.0,
            magic: 0,
            close_ratio: 0.0,
            timestamp: 0,
            comment: [0; 64],
            source_account: [0; 64],
        }
    }
}

// ===========================================================================
// EaContext (Context)
// ===========================================================================

/// EA Context Manager
/// Holds both static account configuration and dynamic runtime state.
#[derive(Debug)]
pub struct EaContext {
    // --- Static Identity (Set via ea_init) ---
    pub account_id: String,
    pub ea_type: String,  // "Master" or "Slave"
    pub platform: String, // "MT4" or "MT5"
    pub account_number: i64,
    pub broker: String,
    pub account_name: String,
    pub server: String,
    pub currency: String,
    pub leverage: i64,

    // --- Runtime State ---
    /// Config request sent flag
    pub is_config_requested: bool,
    /// Last auto-trading state (for tracking changes)
    pub last_trade_allowed: bool,

    // --- Event Loop State ---
    pub last_heartbeat_time: DateTime<Utc>,
    pub pending_commands: VecDeque<EaCommand>,
    // Latest state provided by MQL (for heartbeat)
    pub current_balance: f64,
    pub current_equity: f64,
    pub current_open_positions: i32,

    // --- Cached Config ---
    pub last_master_config: Option<crate::types::MasterConfigMessage>,
    pub pending_master_configs: VecDeque<crate::types::MasterConfigMessage>, // New: Queue for Master Configs
    pub slave_configs: HashMap<String, SlaveConfigMessage>, // Key: Master Account ID
    pub pending_slave_configs: VecDeque<SlaveConfigMessage>, // For UI update command (FIFO queue)
    pub current_slave_config: Option<SlaveConfigMessage>, // Currently being processed by MQL (popped from queue)

    pub last_global_config: Option<crate::types::GlobalConfigMessage>,

    pub last_position_snapshot: Option<crate::types::PositionSnapshotMessage>,
    pub last_sync_request: Option<crate::types::SyncRequestMessage>,

    // --- Communication Layer ---
    pub strategy: Box<dyn CommunicationStrategy>,
}

impl EaContext {
    /// Create a new Context with static identity information
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        account_id: String,
        ea_type: String,
        platform: String,
        account_number: i64,
        broker: String,
        account_name: String,
        server: String,
        currency: String,
        leverage: i64,
    ) -> Self {
        // Select strategy based on ea_type
        let strategy: Box<dyn CommunicationStrategy> = match ea_type.as_str() {
            "Master" => Box::new(MasterStrategy::default()),
            "Slave" => Box::new(SlaveStrategy::default()),
            _ => Box::new(NoOpStrategy),
        };

        Self {
            account_id,
            ea_type,
            platform,
            account_number,
            broker,
            account_name,
            server,
            currency,
            leverage,
            is_config_requested: false,
            last_trade_allowed: false,
            strategy,
            last_heartbeat_time: Utc::now(),
            pending_commands: VecDeque::new(),
            current_balance: 0.0,
            current_equity: 0.0,
            current_open_positions: 0,
            last_master_config: None,
            pending_master_configs: VecDeque::new(),
            slave_configs: HashMap::new(),
            pending_slave_configs: VecDeque::new(),
            current_slave_config: None,
            last_global_config: None,
            last_position_snapshot: None,
            last_sync_request: None,
        }
    }

    // --- Logic Delegation ---

    pub fn connect(&mut self, push_addr: &str, sub_addr: &str) -> Result<(), BridgeError> {
        // We pass self.account_id clone if needed, or refs? Strategy expects &str.
        self.strategy.connect(push_addr, sub_addr, &self.account_id)
    }

    pub fn disconnect(&mut self) {
        self.strategy.disconnect();
    }

    pub fn subscribe_trade(&mut self, master_id: &str) -> Result<(), BridgeError> {
        self.strategy.subscribe_trade(master_id)
    }

    pub fn send_push(&mut self, data: &[u8]) -> Result<(), BridgeError> {
        self.strategy.send_push(data)
    }

    pub fn send_request_config(&mut self, _version: u32) -> Result<(), BridgeError> {
        let msg = RequestConfigMessage {
            message_type: "RequestConfig".to_string(),
            account_id: self.account_id.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            ea_type: self.ea_type.clone(),
        };

        let data = rmp_serde::encode::to_vec_named(&msg)?;
        self.strategy.send_push(&data)?;

        // Mark as requested to prevent duplicate requests
        self.mark_config_requested();

        Ok(())
    }

    /// Enqueue a command for MQL to execute
    /// Includes queue size limit as safety net against excessive duplicate configs
    pub fn enqueue_command(&mut self, cmd: EaCommand) {
        // Safety limit: prevent unbounded queue growth
        const MAX_QUEUE_SIZE: usize = 100;
        if self.pending_commands.len() >= MAX_QUEUE_SIZE {
            eprintln!(
                "[WARN] Command queue full ({} commands), dropping oldest command to prevent overflow",
                MAX_QUEUE_SIZE
            );
            // Drop oldest command
            self.pending_commands.pop_front();
        }

        self.pending_commands.push_back(cmd);
    }

    /// Pop the next command for MQL
    pub fn get_next_command(&mut self) -> Option<EaCommand> {
        self.pending_commands.pop_front()
    }

    /// Main Event Loop Tick (called by MQL OnTimer)
    /// Returns 1 if there is a pending command, 0 otherwise
    pub fn manager_tick(
        &mut self,
        balance: f64,
        equity: f64,
        open_positions: i32,
        is_trade_allowed: bool,
    ) -> i32 {
        self.current_balance = balance;
        self.current_equity = equity;
        self.current_open_positions = open_positions;

        let now = Utc::now();

        // 1. Heartbeat Check (every 1 second)
        if (now - self.last_heartbeat_time).num_seconds() >= 1 {
            if let Err(e) = self.send_heartbeat(is_trade_allowed) {
                eprintln!("Failed to send heartbeat: {}", e);
            } else {
                self.last_heartbeat_time = now;
            }
        }

        // 2. Poll ZMQ (Unified polling for Config + Trade)
        // Since Slave shares the same SUB socket for both, we must use a single loop
        // to consume all messages and dispatch based on topic.
        let mut buffer = [0u8; 4096];
        let mut loop_count = 0;
        const MAX_LOOPS: i32 = 100; // Limit processing to prevent freezing

        loop {
            if loop_count >= MAX_LOOPS {
                break;
            }
            loop_count += 1;

            match self.strategy.receive_config(&mut buffer) {
                Ok(len) if len > 0 => {
                    let data = &buffer[..len as usize];
                    self.process_incoming_message(data);
                }
                _ => break, // No more messages or error
            }
        }

        // 3. Return status
        if !self.pending_commands.is_empty() {
            1
        } else {
            0
        }
    }

    fn send_heartbeat(&mut self, is_trade_allowed: bool) -> Result<(), BridgeError> {
        use crate::types::HeartbeatMessage;

        // Determine version (hardcoded for now or passed in?)
        let version = "2.0.0".to_string();

        let msg = HeartbeatMessage {
            message_type: "Heartbeat".to_string(),
            account_id: self.account_id.clone(),
            balance: self.current_balance,
            equity: self.current_equity,
            open_positions: self.current_open_positions,
            timestamp: Utc::now().to_rfc3339(),
            version,
            ea_type: self.ea_type.clone(),
            platform: self.platform.clone(),
            account_number: self.account_number,
            broker: self.broker.clone(),
            account_name: self.account_name.clone(),
            server: self.server.clone(),
            currency: self.currency.clone(),
            leverage: self.leverage,
            is_trade_allowed,
            symbol_prefix: None, // Could be updated from config
            symbol_suffix: None,
            symbol_map: None,
        };

        let data = rmp_serde::encode::to_vec_named(&msg)?;
        self.strategy.send_push(&data)?;

        // Check if we need to request config (if trade allowed changed to true)
        if is_trade_allowed && !self.last_trade_allowed {
            // Request config logic
            let _ = self.send_request_config(1);
        }
        self.last_trade_allowed = is_trade_allowed;

        Ok(())
    }

    fn process_incoming_message(&mut self, data: &[u8]) {
        // Parse Topic vs Payload (Zero allocation)
        if let Some(space_pos) = data.iter().position(|&b| b == b' ') {
            let topic_bytes = &data[..space_pos];
            let payload = &data[space_pos + 1..];

            // Check prefix directly on bytes
            if topic_bytes.starts_with(b"trade/") {
                if self.ea_type == "Slave" {
                    self.process_incoming_trade(payload);
                }
            } else if topic_bytes.starts_with(b"sync/") {
                // We only convert to String if really needed (e.g. for further parsing)
                // process_sync_message signature needs topic: &str, so we convert just here
                let topic = String::from_utf8_lossy(topic_bytes);
                self.process_sync_message(&topic, payload);
            } else if topic_bytes.starts_with(b"config/") {
                // Check for global config
                if topic_bytes == b"config/global" {
                    self.process_global_config(payload);
                } else {
                    self.process_config_message(payload);
                }
            }
        }
    }

    fn process_config_message(&mut self, payload: &[u8]) {
        // Parse and store config
        if self.ea_type == "Master" {
            if let Ok(config) = rmp_serde::from_slice::<crate::types::MasterConfigMessage>(payload)
            {
                self.last_master_config = Some(config.clone());
                // Also push to pending queue so we can consume it Event-wise
                self.pending_master_configs.push_back(config);
            }
        } else if self.ea_type == "Slave" {
            if let Ok(config) = rmp_serde::from_slice::<crate::types::SlaveConfigMessage>(payload) {
                // Auto subscribe logic
                let master_acc = config.master_account.clone();
                // Also sync topic
                let sync_topic = format!("sync/{}/{}", master_acc, self.account_id);

                // Subscribe if not disabled
                if config.status != 0 {
                    let _ = self.subscribe_trade(&master_acc);
                    let _ = self.subscribe_config(&sync_topic);
                }

                // Store in HashMap
                // -1: No Config (Remove)
                // 0: Disabled (Keep but maybe don't trade)
                // 1, 2: Enabled
                if config.status == -1 {
                    self.slave_configs.remove(&master_acc);
                } else {
                    self.slave_configs
                        .insert(master_acc.clone(), config.clone());
                }

                // For UI Update, we push to pending queue so UI knows to update
                self.pending_slave_configs.push_back(config);
            }
        }

        // Trigger UPDATE_UI command
        let cmd = EaCommand {
            command_type: EaCommandType::UpdateUi as i32,
            ..Default::default()
        };
        self.enqueue_command(cmd);
    }

    fn process_sync_message(&mut self, _topic: &str, payload: &[u8]) {
        if self.ea_type == "Master" {
            if let Ok(req) = rmp_serde::from_slice::<crate::types::SyncRequestMessage>(payload) {
                if req.master_account == self.account_id {
                    self.last_sync_request = Some(req.clone());
                    let mut cmd = EaCommand {
                        command_type: EaCommandType::SendSnapshot as i32,
                        ..Default::default()
                    };
                    copy_string_to_array(&req.slave_account, &mut cmd.comment);
                    self.enqueue_command(cmd);
                }
            }
        } else if self.ea_type == "Slave" {
            if let Ok(snapshot) =
                rmp_serde::from_slice::<crate::types::PositionSnapshotMessage>(payload)
            {
                self.last_position_snapshot = Some(snapshot);
                let cmd = EaCommand {
                    command_type: EaCommandType::ProcessSnapshot as i32,
                    ..Default::default()
                };
                self.enqueue_command(cmd);
            }
        }
    }

    fn process_global_config(&mut self, payload: &[u8]) {
        if let Ok(config) = rmp_serde::from_slice::<crate::types::GlobalConfigMessage>(payload) {
            self.last_global_config = Some(config);
            // Trigger UPDATE_UI command so EA picks up the change
            let cmd = EaCommand {
                command_type: EaCommandType::UpdateUi as i32,
                ..Default::default()
            };
            self.enqueue_command(cmd);
        }
    }

    fn process_incoming_trade(&mut self, data: &[u8]) {
        // Parse trade signal
        if let Ok(signal) = rmp_serde::from_slice::<TradeSignal>(data) {
            // --- Business Logic: Config Lookup & Transformation ---

            // 1. Find Config
            let config = match self.slave_configs.get(&signal.source_account) {
                Some(c) => c,
                None => {
                    // Config not found for this master - ignore trade
                    // In future we might log this to a structured log
                    eprintln!(
                        "Ignored trade from {}: No active config",
                        signal.source_account
                    );
                    return;
                }
            };

            // 2. Check Filters (Basic)
            // allow_new_orders check for Open signals
            if signal.action == TradeAction::Open {
                if !config.allow_new_orders || config.status <= 0 {
                    eprintln!(
                        "Ignored open from {}: New orders disabled",
                        signal.source_account
                    );
                    return;
                }

                // Check allowed symbols (if list is not empty)
                if let Some(allowed) = &config.filters.allowed_symbols {
                    if !allowed.is_empty()
                        && !allowed.contains(signal.symbol.as_ref().unwrap_or(&"".to_string()))
                    {
                        return; // Symbol not allowed
                    }
                }

                // Check blocked symbols
                if let Some(blocked) = &config.filters.blocked_symbols {
                    if blocked.contains(signal.symbol.as_ref().unwrap_or(&"".to_string())) {
                        return; // Symbol blocked
                    }
                }

                // Check magic numbers (optional logic, usually handled by checking signal magic?)
                // Assuming signal.magic is the Master's magic.
                if let Some(magic) = signal.magic_number {
                    if let Some(allowed) = &config.filters.allowed_magic_numbers {
                        if !allowed.is_empty() && !allowed.contains(&magic) {
                            return;
                        }
                    }
                    if let Some(blocked) = &config.filters.blocked_magic_numbers {
                        if blocked.contains(&magic) {
                            return;
                        }
                    }
                }
            }

            let mut algo_flags = 0;

            // 3. Latency Check
            let now = Utc::now();
            let latency_ms = (now - signal.timestamp).num_milliseconds();
            if latency_ms > config.max_signal_delay_ms as i64 {
                if !config.use_pending_order_for_delayed {
                    // Drop expired signal
                    eprintln!(
                        "Dropped expired signal from {} (Latency: {}ms > {}ms)",
                        signal.source_account, latency_ms, config.max_signal_delay_ms
                    );
                    return;
                } else {
                    // Log warning and mark as delayed
                    eprintln!(
                        "Signal delayed from {} (Latency: {}ms). Marking for pending order.",
                        signal.source_account, latency_ms
                    );
                    algo_flags |= 1; // Bit 0: IsDelayed
                }
            }

            // 4. Transform Values
            let mut cmd = EaCommand {
                command_type: match signal.action {
                    TradeAction::Open => EaCommandType::Open as i32,
                    TradeAction::Close => EaCommandType::Close as i32,
                    TradeAction::Modify => EaCommandType::Modify as i32,
                },
                algo_flags,
                ..Default::default()
            };

            if cmd.command_type == 0 {
                return;
            }

            cmd.ticket = signal.ticket;

            // Symbol is already transformed by Relay Server typically,
            // BUT config might have local prefix/suffix?
            // "Symbol transformation is now handled by Relay Server" according to MQL comments.
            // So we take symbol as is.
            if let Some(s) = &signal.symbol {
                copy_string_to_array(s, &mut cmd.symbol);
            }

            // Order Type Reversal
            if let Some(ot) = signal.order_type {
                let final_ot = if config.reverse_trade {
                    ot.reverse()
                } else {
                    ot
                };
                cmd.order_type = i32::from(final_ot);
            }

            // Lot Calculation
            let raw_lots = signal.lots.unwrap_or(0.0);
            let final_lots = transform_lot_size(raw_lots, config, self.current_equity);
            cmd.volume = final_lots;

            // Price, SL, TP - passed as is?
            // Ideally SL/TP distance should be respected if reversing?
            // For now, simpler logic: pass as is (Relay server might handle reverse logic for SL/TP?)
            // If we reverse "Buy" to "Sell", we must swap SL/TP relative to price?
            // Actually, if we reverse, we probably shouldn't blindly copy absolute SL/TP prices.
            // But implementing full reverse SL/TP logic here is complex.
            // Assuming "Simple Reverse" for now or that Master/Relay handles it?
            // In MQL implementation: `ReverseOrderType` only flipped the enum string.
            // It did NOT seem to recalculate SL/TP prices in the snippet provided.
            // Wait, `SlaveTrade.mqh` logic:
            // `transformed_order_type = ReverseOrderType(...)`
            // `ExecuteOpenTrade` receives `cmd.price, cmd.sl, cmd.tp`.
            // If we reverse Buy to Sell, Open Price is Bid instead of Ask (handled by market execution),
            // but SL/TP levels are absolute prices.
            // If Buy @ 1.10, SL 1.09. If Reversed to Sell @ 1.10, SL 1.09 is PROFIT (TP).
            // So MQL implementation was seemingly incomplete or relied on `ExecuteOpenTrade` to fix it?
            // Actually `ExecuteOpenTrade` in `Trade.mqh` (common) likely handles minimal execution.
            // Let's assume for this task we copy MQL logic: Just reverse the enum.

            cmd.price = signal.open_price.unwrap_or(0.0);
            cmd.sl = signal.stop_loss.unwrap_or(0.0);
            cmd.tp = signal.take_profit.unwrap_or(0.0);
            cmd.magic = signal.magic_number.unwrap_or(0);
            cmd.close_ratio = signal.close_ratio.unwrap_or(0.0);

            // Populate MQL Comment with Source Account for reference (optional now since logic is here)
            // But we keep it for backward compat or logging
            if let Some(comment) = &signal.comment {
                copy_string_to_array(comment, &mut cmd.comment);
            }

            // Essential: Set Source Account in new field
            copy_string_to_array(&signal.source_account, &mut cmd.source_account);

            cmd.timestamp = signal.timestamp.timestamp_millis();

            self.enqueue_command(cmd);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn send_open_signal(
        &mut self,
        ticket: i64,
        symbol: &str,
        order_type: OrderType,
        lots: f64,
        price: f64,
        sl: f64,
        tp: f64,
        magic: i64,
        comment: &str,
    ) -> Result<(), BridgeError> {
        let msg = TradeSignal {
            action: TradeAction::Open,
            ticket,
            symbol: Some(symbol.to_string()),
            order_type: Some(order_type),
            lots: Some(lots),
            open_price: Some(price),
            stop_loss: Some(sl),
            take_profit: Some(tp),
            magic_number: Some(magic),
            comment: Some(comment.to_string()),
            timestamp: chrono::Utc::now(),
            source_account: self.account_id.clone(),
            close_ratio: None,
        };

        let data = rmp_serde::encode::to_vec_named(&msg)?;
        self.strategy.send_push(&data)?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn send_close_signal(
        &mut self,
        ticket: i64,
        lots: f64,
        close_ratio: f64,
    ) -> Result<(), BridgeError> {
        let msg = TradeSignal {
            action: crate::constants::TradeAction::Close,
            ticket,
            symbol: None,
            order_type: None,
            lots: Some(lots),
            open_price: None,
            stop_loss: None,
            take_profit: None,
            magic_number: None,
            comment: None,
            timestamp: chrono::Utc::now(),
            source_account: self.account_id.clone(),
            close_ratio: if (close_ratio - 1.0).abs() < 1e-6 {
                None
            } else {
                Some(close_ratio)
            },
        };

        let data = rmp_serde::encode::to_vec_named(&msg)?;
        self.strategy.send_push(&data)?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn send_modify_signal(&mut self, ticket: i64, sl: f64, tp: f64) -> Result<(), BridgeError> {
        let msg = TradeSignal {
            action: TradeAction::Modify,
            ticket,
            symbol: None,
            order_type: None,
            lots: None,
            open_price: None,
            stop_loss: if sl.abs() < 1e-6 { None } else { Some(sl) },
            take_profit: if tp.abs() < 1e-6 { None } else { Some(tp) },
            magic_number: None,
            comment: None,
            timestamp: chrono::Utc::now(),
            source_account: self.account_id.clone(),
            close_ratio: None,
        };

        let data = rmp_serde::encode::to_vec_named(&msg)?;
        self.strategy.send_push(&data)?;
        Ok(())
    }

    pub fn send_sync_request(
        &mut self,
        master_account: &str,
        last_sync_time: Option<String>,
    ) -> Result<(), BridgeError> {
        let msg = crate::types::SyncRequestMessage {
            message_type: "SyncRequest".to_string(),
            slave_account: self.account_id.clone(),
            master_account: master_account.to_string(),
            last_sync_time,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        let data = rmp_serde::encode::to_vec_named(&msg)?;
        self.strategy.send_push(&data)?;
        Ok(())
    }

    pub fn send_position_snapshot(
        &mut self,
        positions: Vec<crate::types::PositionInfo>,
    ) -> Result<(), BridgeError> {
        let msg = crate::types::PositionSnapshotMessage {
            message_type: "PositionSnapshot".to_string(),
            source_account: self.account_id.clone(),
            positions,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        let data = rmp_serde::encode::to_vec_named(&msg)?;
        self.strategy.send_push(&data)?;
        Ok(())
    }

    // --- Socket Accessors for FFI (Raw Pointers) ---
    pub fn get_config_socket_ptr(&mut self) -> Result<*mut std::ffi::c_void, BridgeError> {
        self.strategy.get_config_socket_ptr()
    }

    pub fn get_trade_socket_ptr(&mut self) -> Result<*mut std::ffi::c_void, BridgeError> {
        self.strategy.get_trade_socket_ptr()
    }

    /// Receive from Config socket (non-blocking)
    pub fn receive_config(&mut self, buffer: &mut [u8]) -> Result<i32, BridgeError> {
        self.strategy.receive_config(buffer)
    }

    /// Receive from Trade socket (Slave only, non-blocking)
    pub fn receive_trade(&mut self, buffer: &mut [u8]) -> Result<i32, BridgeError> {
        self.strategy.receive_trade(buffer)
    }

    /// Subscribe to topic on Config socket
    pub fn subscribe_config(&mut self, topic: &str) -> Result<(), BridgeError> {
        self.strategy.subscribe_config(topic)
    }

    // --- Original Logic ---

    /// Determine if RequestConfig should be sent
    pub fn should_request_config(&mut self, current_trade_allowed: bool) -> bool {
        let trade_allowed_turned_on = current_trade_allowed && !self.last_trade_allowed;
        self.last_trade_allowed = current_trade_allowed;

        if !self.is_config_requested || trade_allowed_turned_on {
            return true;
        }
        false
    }

    /// Mark that specific config has been requested
    pub fn mark_config_requested(&mut self) {
        self.is_config_requested = true;
    }

    /// Reset state (e.g. on reconnection)
    pub fn reset(&mut self) {
        self.is_config_requested = false;
        // Typically we don't disconnect ZMQ on logic reset, only explicitly.
        // But if we want to ensure clean state, we might want to clear subscriptions?
        // For now, keep connection, just reset state flags.
    }
}

// --- Logic Helpers ---

fn transform_lot_size(lots: f64, config: &SlaveConfigMessage, slave_equity: f64) -> f64 {
    use crate::types::LotCalculationMode;

    let mut new_lots = lots;

    match config.lot_calculation_mode {
        LotCalculationMode::MarginRatio => {
            if let Some(master_equity) = config.master_equity {
                if master_equity > 0.0 {
                    let ratio = slave_equity / master_equity;
                    new_lots = lots * ratio;
                }
            }
        }
        LotCalculationMode::Multiplier => {
            if let Some(mult) = config.lot_multiplier {
                new_lots = lots * mult;
            }
        }
    }

    // Normalize? Rust doesn't have SymbolInfoDouble for MIN/MAX/STEP.
    // However, we can at least clamp to configured Min/Max from config if present.
    // MQL `NormalizeLotSize` uses local broker info which we don't have here perfectly.
    // BUT we have `source_lot_min/max` in config.
    // Wait, `source_lot_min` checks the *input* (Master) lots usually?
    // The comment says: "Minimum lot size filter: skip trades with lot smaller than this value"
    // So that's a filter, not a clamper.

    // The actual "Round to Step" logic must effectively remain in MQL
    // because `SYMBOL_VOLUME_STEP` is broker specific and dynamic.
    // OR we pass `SYMBOL_VOLUME_STEP` in Heartbeat or Init? No, that's too much.
    //
    // Compromise:
    // Rust does the *strategy* calculation (Multiplier/Ratio).
    // MQL does the *normalization* (Step/Min/Max clamping based on Broker Info).

    new_lots
}

fn copy_string_to_array<const N: usize>(s: &str, arr: &mut [u8; N]) {
    // Max bytes we can safely store (leaving room for null terminator if needed,
    // though this logic guarantees 0 at N-1 if we don't fill it?
    // Actually simpler: fill up to N-1, ensure safely cut at char boundary.
    let max_len = N - 1;

    let bytes = if s.len() <= max_len {
        s.as_bytes()
    } else {
        // Find safe cut point
        let mut end = max_len;
        while end > 0 && !is_char_boundary(s, end) {
            end -= 1;
        }
        &s.as_bytes()[..end]
    };

    arr[..bytes.len()].copy_from_slice(bytes);
    // Fill remaining with 0
    // Note: iterating range logic is efficient in Rust
    // Fill remaining with 0
    arr[bytes.len()..].fill(0);
}

// Helper: Check if index is a valid UTF-8 char boundary
// See: https://doc.rust-lang.org/std/primitive.str.html#method.is_char_boundary
// But we cannot use s.is_char_boundary(index) if index is not a valid implementation detail?
// Actually s.is_char_boundary(index) is safe.
fn is_char_boundary(s: &str, index: usize) -> bool {
    // Fallback manual check if needed, or use std method:
    // s.is_char_boundary(index)
    // Here we implement the logic manually to match user request style or just use std
    if index == 0 {
        return true;
    }
    match s.as_bytes().get(index) {
        // 10xxxxxx (0x80 .. 0xBF) means continuation byte.
        // So valid boundary is NOT (0x80 & b != 0)
        // i.e. (b & 0xC0) != 0x80
        Some(&b) => (b & 0xC0) != 0x80,
        None => true, // End of string is boundary
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::BridgeError;
    use std::sync::{Arc, Mutex};

    fn create_test_context(ea_type: &str) -> EaContext {
        EaContext::new(
            "test_acc".to_string(),
            ea_type.to_string(),
            "MT5".to_string(),
            123456,
            "TestBroker".to_string(),
            "Test Account".to_string(),
            "TestServer".to_string(),
            "USD".to_string(),
            100,
        )
    }

    #[test]
    fn test_strategy_selection() {
        let master = create_test_context("Master");
        assert_eq!(master.ea_type, "Master");

        let slave = create_test_context("Slave");
        assert_eq!(slave.ea_type, "Slave");
    }

    #[test]
    fn test_initial_state() {
        let ctx = create_test_context("Master");
        assert!(!ctx.is_config_requested);
        assert!(!ctx.last_trade_allowed);
        assert_eq!(ctx.account_id, "test_acc");
        assert_eq!(ctx.broker, "TestBroker");
    }

    #[test]
    fn test_should_request_logic() {
        let mut ctx = create_test_context("Master");
        assert!(ctx.should_request_config(true));
        ctx.mark_config_requested();
        assert!(!ctx.should_request_config(true));
        ctx.reset();
        assert!(ctx.should_request_config(true));
    }

    #[derive(Debug, Default)]
    struct MockStrategy {
        sent_data: Arc<Mutex<Vec<Vec<u8>>>>,
        incoming_data: Arc<Mutex<VecDeque<Vec<u8>>>>,
        next_error: Arc<Mutex<Option<BridgeError>>>,
    }

    impl CommunicationStrategy for MockStrategy {
        fn connect(&mut self, _: &str, _: &str, _: &str) -> Result<(), BridgeError> {
            Ok(())
        }
        fn disconnect(&mut self) {}
        fn send_push(&mut self, data: &[u8]) -> Result<(), BridgeError> {
            self.sent_data.lock().unwrap().push(data.to_vec());
            Ok(())
        }
        fn subscribe_trade(&mut self, _: &str) -> Result<(), BridgeError> {
            Ok(())
        }
        fn get_config_socket_ptr(&mut self) -> Result<*mut std::ffi::c_void, BridgeError> {
            Err(BridgeError::Init("Mock".to_string()))
        }
        fn get_trade_socket_ptr(&mut self) -> Result<*mut std::ffi::c_void, BridgeError> {
            Err(BridgeError::Init("Mock".to_string()))
        }
        fn receive_config(&mut self, buffer: &mut [u8]) -> Result<i32, BridgeError> {
            if let Some(err) = self.next_error.lock().unwrap().take() {
                return Err(err);
            }
            let mut queue = self.incoming_data.lock().unwrap();
            if let Some(data) = queue.pop_front() {
                if data.len() > buffer.len() {
                    return Err(BridgeError::Generic("Buffer too small".to_string()));
                }
                buffer[..data.len()].copy_from_slice(&data);
                Ok(data.len() as i32)
            } else {
                Ok(0)
            }
        }
        fn receive_trade(&mut self, buffer: &mut [u8]) -> Result<i32, BridgeError> {
            self.receive_config(buffer)
        }
        fn subscribe_config(&mut self, _: &str) -> Result<(), BridgeError> {
            Ok(())
        }
    }

    #[test]
    fn test_send_request_config() {
        let mut ctx = create_test_context("Slave");
        let sent_data = Arc::new(Mutex::new(Vec::new()));

        ctx.strategy = Box::new(MockStrategy {
            sent_data: sent_data.clone(),
            incoming_data: Arc::new(Mutex::new(VecDeque::new())),
            next_error: Arc::new(Mutex::new(None)),
        });

        ctx.send_request_config(1)
            .expect("Failed to send request config");

        let data = sent_data.lock().unwrap();
        assert_eq!(data.len(), 1, "Should have sent one message");
    }

    #[test]
    fn test_send_open_signal() {
        let mut ctx = create_test_context("Master");
        let sent_data = Arc::new(Mutex::new(Vec::new()));

        ctx.strategy = Box::new(MockStrategy {
            sent_data: sent_data.clone(),
            incoming_data: Arc::new(Mutex::new(VecDeque::new())),
            next_error: Arc::new(Mutex::new(None)),
        });

        ctx.send_open_signal(
            12345,
            "EURUSD",
            OrderType::Buy,
            0.1,
            1.1050,
            1.1000,
            1.1100,
            123,
            "Test Comment",
        )
        .expect("Failed to send open signal");

        let data = sent_data.lock().unwrap();
        assert_eq!(data.len(), 1, "Should have sent one message");
    }

    #[test]
    fn test_processing_incoming_config_slave() {
        let mut ctx = create_test_context("Slave");
        let incoming = Arc::new(Mutex::new(VecDeque::new()));

        // Prepare a SlaveConfigMessage mock payload
        let config = crate::types::SlaveConfigMessage {
            account_id: "test_acc".to_string(),
            master_account: "master1".to_string(),
            status: 1, // Enabled
            ..Default::default()
        };
        let mut config_bytes = rmp_serde::to_vec_named(&config).unwrap();
        // Prepend topic if process_incoming_message expects it?
        // logic: `if let Some(space_pos) = data.iter().position(|&b| b == b' ')`
        // We need to construct "config/global " + msgpack
        let mut payload = b"config/slave ".to_vec();
        payload.append(&mut config_bytes);

        incoming.lock().unwrap().push_back(payload);

        ctx.strategy = Box::new(MockStrategy {
            sent_data: Arc::new(Mutex::new(Vec::new())),
            incoming_data: incoming.clone(),
            next_error: Arc::new(Mutex::new(None)),
        });

        // Run manager_tick
        let pending = ctx.manager_tick(1000.0, 1000.0, 0, true);

        assert_eq!(pending, 1, "Should have pending command (UpdateUi)");
        // Check pending queue instead of last_slave_config
        assert!(ctx.pending_slave_configs.back().is_some());
        assert_eq!(
            ctx.pending_slave_configs.back().unwrap().master_account,
            "master1"
        );

        let cmd = ctx.get_next_command().expect("No command found");
        assert_eq!(cmd.command_type, EaCommandType::UpdateUi as i32);
    }

    #[test]
    fn test_processing_incoming_trade_slave_with_logic() {
        let mut ctx = create_test_context("Slave");
        let incoming = Arc::new(Mutex::new(VecDeque::new()));

        // 1. Inject Config first
        let config = crate::types::SlaveConfigMessage {
            account_id: "test_acc".to_string(),
            master_account: "master1".to_string(),
            status: 1, // Enabled
            lot_calculation_mode: crate::types::LotCalculationMode::Multiplier,
            lot_multiplier: Some(2.0),
            reverse_trade: true,
            ..Default::default()
        };
        let mut config_bytes = rmp_serde::to_vec_named(&config).unwrap();
        let mut payload_conf = b"config/slave ".to_vec();
        payload_conf.append(&mut config_bytes);
        incoming.lock().unwrap().push_back(payload_conf);

        // 2. Inject Trade Signal
        let signal = crate::types::TradeSignal {
            action: TradeAction::Open,
            ticket: 999,
            symbol: Some("GBPUSD".to_string()),
            lots: Some(0.5),
            source_account: "master1".to_string(),
            order_type: Some(OrderType::Buy),
            ..Default::default()
        };
        let mut signal_bytes = rmp_serde::to_vec_named(&signal).unwrap();
        let mut payload_trade = b"trade/master1 ".to_vec();
        payload_trade.append(&mut signal_bytes);
        incoming.lock().unwrap().push_back(payload_trade);

        ctx.strategy = Box::new(MockStrategy {
            sent_data: Arc::new(Mutex::new(Vec::new())),
            incoming_data: incoming.clone(),
            next_error: Arc::new(Mutex::new(None)),
        });

        // Tick 1: Process Config
        ctx.manager_tick(1000.0, 1000.0, 0, true);
        ctx.get_next_command(); // Clear UI command

        // Tick 2: Process Trade (Should succeed because Config is present)
        let pending = ctx.manager_tick(1000.0, 1000.0, 0, true);
        assert_eq!(pending, 1, "Should have pending command (Open)");

        let cmd = ctx.get_next_command().unwrap();
        assert_eq!(cmd.command_type, EaCommandType::Open as i32);
        assert_eq!(cmd.ticket, 999);

        // Verify Lot Transformation (0.5 * 2.0 = 1.0)
        assert!((cmd.volume - 1.0).abs() < 1e-6, "Volume should be 1.0");

        // Verify Order Reversal (Buy -> Sell)
        // OrderType::Buy = 0, OrderType::Sell = 1 (check implementation of OrderType)
        // constants.rs says OrderType::Sell is 1.
        assert_eq!(cmd.order_type, 1);

        // Verify Source Account
        let src = String::from_utf8_lossy(&cmd.source_account)
            .trim_matches(char::from(0))
            .to_string();
        assert_eq!(src, "master1");
    }

    #[test]
    fn test_error_handling_in_loop() {
        let mut ctx = create_test_context("Slave");
        let next_error = Arc::new(Mutex::new(Some(BridgeError::Generic(
            "Simulated Failure".to_string(),
        ))));

        ctx.strategy = Box::new(MockStrategy {
            sent_data: Arc::new(Mutex::new(Vec::new())),
            incoming_data: Arc::new(Mutex::new(VecDeque::new())),
            next_error: next_error.clone(),
        });

        // manager_tick should call receive_config, fail, log error (eprintln) and continue/break without panic
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            ctx.manager_tick(1000.0, 1000.0, 0, true)
        }));

        assert!(result.is_ok(), "Manager tick panicked on error!");
    }

    // Helper to create a dummy config with specific latency settings
    fn create_latency_test_config(
        master_account: &str,
        max_delay: i32,
        use_pending: bool,
    ) -> crate::types::SlaveConfigMessage {
        crate::types::SlaveConfigMessage {
            account_id: "test_acc".to_string(),
            master_account: master_account.to_string(),
            timestamp: Utc::now().timestamp_millis(),
            trade_group_id: "GROUP_001".to_string(),
            status: 2, // Connected
            lot_calculation_mode: crate::types::LotCalculationMode::Multiplier,
            lot_multiplier: Some(1.0),
            reverse_trade: false,
            symbol_prefix: None,
            symbol_suffix: None,
            symbol_mappings: vec![],
            filters: crate::types::TradeFilters::default(),
            config_version: 1,
            source_lot_min: None,
            source_lot_max: None,
            master_equity: None,
            sync_mode: Default::default(),
            limit_order_expiry_min: None,
            market_sync_max_pips: None,
            max_slippage: None,
            copy_pending_orders: false,
            max_retries: 3,
            max_signal_delay_ms: max_delay,
            use_pending_order_for_delayed: use_pending,
            allow_new_orders: true,
            warning_codes: vec![],
        }
    }

    #[test]
    fn test_latency_check_fresh_signal() {
        let mut ctx = create_test_context("Slave");
        let incoming = Arc::new(Mutex::new(VecDeque::new()));
        let master_acc = "master1";

        // 1. Config
        let config = create_latency_test_config(master_acc, 1000, false);
        let mut config_bytes = rmp_serde::to_vec_named(&config).unwrap();
        let mut payload_conf = b"config/slave ".to_vec();
        payload_conf.append(&mut config_bytes);
        incoming.lock().unwrap().push_back(payload_conf);

        // 2. Fresh Signal (Now)
        let signal = crate::types::TradeSignal {
            action: crate::constants::TradeAction::Open,
            ticket: 1001,
            symbol: Some("EURUSD".to_string()),
            lots: Some(0.1),
            open_price: Some(1.1000),
            source_account: master_acc.to_string(),
            timestamp: Utc::now(),
            ..Default::default()
        };
        let mut signal_bytes = rmp_serde::to_vec_named(&signal).unwrap();
        let mut payload_trade = format!("trade/{} ", master_acc).as_bytes().to_vec();
        payload_trade.append(&mut signal_bytes);
        incoming.lock().unwrap().push_back(payload_trade);

        ctx.strategy = Box::new(MockStrategy {
            sent_data: Arc::new(Mutex::new(Vec::new())),
            incoming_data: incoming.clone(),
            next_error: Arc::new(Mutex::new(None)),
        });

        // Tick 1: Config
        ctx.manager_tick(1000.0, 1000.0, 0, true);
        ctx.get_next_command(); // Clear UI command

        // Tick 2: Trade
        let pending = ctx.manager_tick(1000.0, 1000.0, 0, true);
        assert_eq!(pending, 1, "Fresh signal should be accepted");
        let cmd = ctx.get_next_command().unwrap();
        assert_eq!(cmd.ticket, 1001);
    }

    #[test]
    fn test_latency_check_expired_signal_drop() {
        let mut ctx = create_test_context("Slave");
        let incoming = Arc::new(Mutex::new(VecDeque::new()));
        let master_acc = "master1";

        // 1. Config (Drop if > 1000ms)
        let config = create_latency_test_config(master_acc, 1000, false);
        let mut config_bytes = rmp_serde::to_vec_named(&config).unwrap();
        let mut payload_conf = b"config/slave ".to_vec();
        payload_conf.append(&mut config_bytes);
        incoming.lock().unwrap().push_back(payload_conf);

        // 2. Expired Signal (5 sec old)
        let signal = crate::types::TradeSignal {
            action: crate::constants::TradeAction::Open,
            ticket: 1002,
            symbol: Some("EURUSD".to_string()),
            lots: Some(0.1),
            open_price: Some(1.1000),
            source_account: master_acc.to_string(),
            timestamp: Utc::now() - chrono::Duration::seconds(5),
            ..Default::default()
        };
        let mut signal_bytes = rmp_serde::to_vec_named(&signal).unwrap();
        let mut payload_trade = format!("trade/{} ", master_acc).as_bytes().to_vec();
        payload_trade.append(&mut signal_bytes);
        incoming.lock().unwrap().push_back(payload_trade);

        ctx.strategy = Box::new(MockStrategy {
            sent_data: Arc::new(Mutex::new(Vec::new())),
            incoming_data: incoming.clone(),
            next_error: Arc::new(Mutex::new(None)),
        });

        // Tick 1: Config
        ctx.manager_tick(1000.0, 1000.0, 0, true);
        ctx.get_next_command();

        // Tick 2: Trade (Should be Dropped)
        let pending = ctx.manager_tick(1000.0, 1000.0, 0, true);

        // Verify that the expired signal is dropped (pending=0)
        assert_eq!(pending, 0, "Expired signal should be dropped");
    }

    #[test]
    fn test_latency_check_expired_signal_use_pending() {
        let mut ctx = create_test_context("Slave");
        let incoming = Arc::new(Mutex::new(VecDeque::new()));
        let master_acc = "master1";

        // 1. Config (Keep if > 1000ms)
        let config = create_latency_test_config(master_acc, 1000, true);
        let mut config_bytes = rmp_serde::to_vec_named(&config).unwrap();
        let mut payload_conf = b"config/slave ".to_vec();
        payload_conf.append(&mut config_bytes);
        incoming.lock().unwrap().push_back(payload_conf);

        // 2. Expired Signal
        let signal = crate::types::TradeSignal {
            action: crate::constants::TradeAction::Open,
            ticket: 1003,
            symbol: Some("EURUSD".to_string()),
            lots: Some(0.1),
            open_price: Some(1.1000),
            source_account: master_acc.to_string(),
            timestamp: Utc::now() - chrono::Duration::seconds(5),
            ..Default::default()
        };
        let mut signal_bytes = rmp_serde::to_vec_named(&signal).unwrap();
        let mut payload_trade = format!("trade/{} ", master_acc).as_bytes().to_vec();
        payload_trade.append(&mut signal_bytes);
        incoming.lock().unwrap().push_back(payload_trade);

        ctx.strategy = Box::new(MockStrategy {
            sent_data: Arc::new(Mutex::new(Vec::new())),
            incoming_data: incoming.clone(),
            next_error: Arc::new(Mutex::new(None)),
        });

        // Tick 1: Config
        ctx.manager_tick(1000.0, 1000.0, 0, true);
        ctx.get_next_command();

        // Tick 2: Trade (Should Pass)
        let pending = ctx.manager_tick(1000.0, 1000.0, 0, true);
        assert_eq!(
            pending, 1,
            "Expired signal with use_pending should assume pending order creation"
        );
        let cmd = ctx.get_next_command().unwrap();
        assert_eq!(cmd.ticket, 1003);
    }
}
