// Location: mt-bridge/src/ffi.rs
// Purpose: Unified FFI functions for MQL4/MQL5 integration (ZMQ + MessagePack)
// Why: Provides C-compatible interface for ZMQ operations and MessagePack message handling

use crate::constants::{self, TOPIC_GLOBAL_CONFIG};
use crate::ea_context::{EaCommand, EaContext};
use crate::ffi_helpers::{copy_string_to_fixed_array, utf16_to_string};
use crate::ffi_types::{CMasterConfig, CPositionInfo, CSlaveConfig, CSymbolMapping, CSyncRequest};
use crate::types::{LotCalculationMode, SyncMode};
use chrono::DateTime;

// ===========================================================================
// Connection Management
// ===========================================================================

/// Connect to Relay Server (Initialize ZMQ sockets and subscribe context-specifically)
///
/// # Safety
/// - context: Valid EaContext pointer
/// - push_addr: Valid UTF-16 string (e.g. "tcp://localhost:5555")
/// - sub_addr: Valid UTF-16 string (e.g. "tcp://localhost:5556")
#[no_mangle]
pub unsafe extern "C" fn ea_connect(
    context: *mut EaContext,
    push_addr: *const u16,
    sub_addr: *const u16,
) -> i32 {
    let ctx = match context.as_mut() {
        Some(c) => c,
        None => return 0,
    };

    let push = match utf16_to_string(push_addr) {
        Some(s) => s,
        None => return 0,
    };
    let sub = match utf16_to_string(sub_addr) {
        Some(s) => s,
        None => return 0,
    };

    match ctx.connect(&push, &sub) {
        Ok(_) => 1,
        Err(e) => {
            eprintln!("ea_connect failed: {}", e);
            0
        }
    }
}

/// Send RequestConfig message (Slave only)
///
/// # Safety
/// - context: Valid EaContext pointer
#[no_mangle]
pub unsafe extern "C" fn ea_send_request_config(context: *mut EaContext, version: u32) -> i32 {
    let ctx = match context.as_mut() {
        Some(c) => c,
        None => return 0,
    };

    match ctx.send_request_config(version) {
        Ok(_) => 1,
        Err(e) => {
            eprintln!("ea_send_request_config failed: {}", e);
            0
        }
    }
}

/// Send raw data via PUSH socket
///
/// # Safety
/// - context: Valid EaContext pointer
/// - data: Valid buffer
#[no_mangle]
pub unsafe extern "C" fn ea_send_push(context: *mut EaContext, data: *const u8, len: i32) -> i32 {
    let ctx = match context.as_mut() {
        Some(c) => c,
        None => return 0,
    };
    if data.is_null() || len <= 0 {
        return 0;
    }
    let slice = std::slice::from_raw_parts(data, len as usize);
    match ctx.send_push(slice) {
        Ok(_) => 1,
        Err(e) => {
            eprintln!("ea_send_push failed: {}", e);
            0
        }
    }
}

/// Receive message from Config socket (high-level, non-blocking)
///
/// # Safety
/// - context: Valid EaContext pointer
/// - buffer: Valid buffer pointer
#[no_mangle]
pub unsafe extern "C" fn ea_receive_config(
    context: *mut EaContext,
    buffer: *mut u8,
    buffer_size: i32,
) -> i32 {
    let ctx = match context.as_mut() {
        Some(c) => c,
        None => return 0,
    };
    if buffer.is_null() || buffer_size <= 0 {
        return 0;
    }
    let slice = std::slice::from_raw_parts_mut(buffer, buffer_size as usize);
    ctx.receive_config(slice).unwrap_or_default()
}

/// Subscribe to topic on Config socket
///
/// # Safety
/// - context: Valid EaContext pointer
/// - topic: Valid null-terminated UTF-16 string
#[no_mangle]
pub unsafe extern "C" fn ea_subscribe_config(context: *mut EaContext, topic: *const u16) -> i32 {
    let ctx = match context.as_mut() {
        Some(c) => c,
        None => return 0,
    };
    let topic_str = match utf16_to_string(topic) {
        Some(s) => s,
        None => return 0,
    };
    match ctx.subscribe_config(&topic_str) {
        Ok(_) => 1,
        Err(_) => 0,
    }
}

// ===========================================================================
// Topic Generation FFI Functions
// ===========================================================================

/// Helper function to write a string to UTF-16 buffer
/// Returns the string length (excluding null terminator), 0 on error, -1 if buffer too small
#[inline]
unsafe fn write_string_to_utf16_buffer(s: &str, output: *mut u16, output_len: i32) -> i32 {
    let utf16: Vec<u16> = s.encode_utf16().chain(std::iter::once(0)).collect();

    if utf16.len() > output_len as usize {
        return -1; // Buffer too small
    }

    let output_slice = std::slice::from_raw_parts_mut(output, output_len as usize);
    output_slice[..utf16.len()].copy_from_slice(&utf16);

    (utf16.len() - 1) as i32 // Return length without null terminator
}

/// Build a config topic string: "config/{account_id}"
///
/// # Safety
/// - account_id must be a valid null-terminated UTF-16 string pointer
/// - output must be a valid buffer of at least output_len u16 elements
/// - output_len must be positive
#[no_mangle]
pub unsafe extern "C" fn build_config_topic(
    account_id: *const u16,
    output: *mut u16,
    output_len: i32,
) -> i32 {
    if account_id.is_null() || output.is_null() || output_len <= 0 {
        return 0;
    }

    let account = match utf16_to_string(account_id) {
        Some(s) => s,
        None => return 0,
    };

    let topic = constants::build_config_topic(&account);
    write_string_to_utf16_buffer(&topic, output, output_len)
}

/// Build a trade topic string: "trade/{master_id}/{slave_id}"
///
/// # Safety
/// - master_id must be a valid null-terminated UTF-16 string pointer
/// - slave_id must be a valid null-terminated UTF-16 string pointer
/// - output must be a valid buffer of at least output_len u16 elements
/// - output_len must be positive
#[no_mangle]
pub unsafe extern "C" fn build_trade_topic(
    master_id: *const u16,
    slave_id: *const u16,
    output: *mut u16,
    output_len: i32,
) -> i32 {
    if master_id.is_null() || slave_id.is_null() || output.is_null() || output_len <= 0 {
        return 0;
    }

    let master = match utf16_to_string(master_id) {
        Some(s) => s,
        None => return 0,
    };

    let slave = match utf16_to_string(slave_id) {
        Some(s) => s,
        None => return 0,
    };

    let topic = constants::build_trade_topic(&master, &slave);
    write_string_to_utf16_buffer(&topic, output, output_len)
}

/// Get the global config topic string: "config/global"
///
/// # Safety
/// - output must be a valid buffer of at least output_len u16 elements
/// - output_len must be positive
#[no_mangle]
pub unsafe extern "C" fn get_global_config_topic(output: *mut u16, output_len: i32) -> i32 {
    if output.is_null() || output_len <= 0 {
        return 0;
    }

    write_string_to_utf16_buffer(TOPIC_GLOBAL_CONFIG, output, output_len)
}

/// Build a sync topic string: "sync/{master_id}/{slave_id}"
///
/// Used for PositionSnapshot (Master → Slave) and SyncRequest (Slave → Master) messages.
///
/// # Safety
/// - master_id must be a valid null-terminated UTF-16 string pointer
/// - slave_id must be a valid null-terminated UTF-16 string pointer
/// - output must be a valid buffer of at least output_len u16 elements
/// - output_len must be positive
///
/// # Returns
/// Length of the generated topic string (not including null terminator), or 0 on error.
#[no_mangle]
pub unsafe extern "C" fn build_sync_topic_ffi(
    master_id: *const u16,
    slave_id: *const u16,
    output: *mut u16,
    output_len: i32,
) -> i32 {
    if master_id.is_null() || slave_id.is_null() || output.is_null() || output_len <= 0 {
        return 0;
    }

    let master = match utf16_to_string(master_id) {
        Some(s) => s,
        None => return 0,
    };

    let slave = match utf16_to_string(slave_id) {
        Some(s) => s,
        None => return 0,
    };

    let topic = constants::build_sync_topic(&master, &slave);
    write_string_to_utf16_buffer(&topic, output, output_len)
}

/// Get the sync topic prefix for a Master EA: "sync/{account_id}/"
///
/// Master EAs subscribe to this prefix to receive all SyncRequest messages
/// from any slave connected to them.
///
/// # Safety
/// - account_id must be a valid null-terminated UTF-16 string pointer
/// - output must be a valid buffer of at least output_len u16 elements
/// - output_len must be positive
///
/// # Returns
/// Length of the generated topic prefix (not including null terminator), or 0 on error.
#[no_mangle]
pub unsafe extern "C" fn get_sync_topic_prefix(
    account_id: *const u16,
    output: *mut u16,
    output_len: i32,
) -> i32 {
    if account_id.is_null() || output.is_null() || output_len <= 0 {
        return 0;
    }

    let account = match utf16_to_string(account_id) {
        Some(s) => s,
        None => return 0,
    };

    // Format: "sync/{account_id}/"
    let topic_prefix = format!("{}{}/", constants::TOPIC_SYNC_PREFIX, account);
    write_string_to_utf16_buffer(&topic_prefix, output, output_len)
}

// ============================================================================
// EA State Management FFI Functions
// ============================================================================

/// Create and serialize a RegisterMessage using context data (Zero arguments from MQL!)
///
/// # Safety
/// - `context` must be a valid pointer returned by `ea_init()`
/// - `output` must be a valid buffer
#[no_mangle]
pub unsafe extern "C" fn ea_send_register(
    context: *mut crate::EaContext,
    output: *mut u8,
    output_len: i32,
) -> i32 {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if context.is_null() {
            return -1;
        }

        let ctx = &*context;

        // Build RegisterMessage using cached context data
        let msg = crate::types::RegisterMessage {
            message_type: "Register".to_string(),
            account_id: ctx.account_id.clone(),
            ea_type: ctx.ea_type.clone(),
            platform: ctx.platform.clone(),
            account_number: ctx.account_number,
            broker: ctx.broker.clone(),
            account_name: ctx.account_name.clone(),
            server: ctx.server.clone(),
            currency: ctx.currency.clone(),
            leverage: ctx.leverage,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        unsafe { crate::ffi_helpers::serialize_to_buffer(&msg, output, output_len) }
    }));

    result.unwrap_or(-1)
}

/// Create and serialize a HeartbeatMessage using context data + dynamic args
///
/// # Safety
/// - `context` must be a valid pointer returned by `ea_init()`
#[no_mangle]
pub unsafe extern "C" fn ea_send_heartbeat(
    context: *mut crate::EaContext,
    balance: f64,
    equity: f64,
    open_positions: i32,
    is_trade_allowed: i32,
    output: *mut u8,
    output_len: i32,
) -> i32 {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if context.is_null() {
            return -1;
        }

        let ctx = &*context;

        let msg = crate::types::HeartbeatMessage {
            message_type: "Heartbeat".to_string(),
            account_id: ctx.account_id.clone(),
            balance,
            equity,
            open_positions,
            timestamp: chrono::Utc::now().to_rfc3339(),
            version: env!("BUILD_INFO").to_string(),
            ea_type: ctx.ea_type.clone(),
            platform: ctx.platform.clone(),
            account_number: ctx.account_number,
            broker: ctx.broker.clone(),
            account_name: ctx.account_name.clone(),
            server: ctx.server.clone(),
            currency: ctx.currency.clone(),
            leverage: ctx.leverage,
            is_trade_allowed: is_trade_allowed != 0,
            symbol_prefix: None,
            symbol_suffix: None,
            symbol_map: None,
        };

        unsafe { crate::ffi_helpers::serialize_to_buffer(&msg, output, output_len) }
    }));

    result.unwrap_or(-1)
}

/// Create and serialize an UnregisterMessage using context data
///
/// # Safety
/// - `context` must be a valid pointer created by `ea_init`
/// - `output` must point to a valid buffer with at least `output_len` bytes
#[no_mangle]
pub unsafe extern "C" fn ea_send_unregister(
    context: *mut crate::EaContext,
    output: *mut u8,
    output_len: i32,
) -> i32 {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if context.is_null() {
            return -1;
        }
        let ctx = &*context;

        let msg = crate::types::UnregisterMessage {
            message_type: "Unregister".to_string(),
            account_id: ctx.account_id.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            ea_type: Some(ctx.ea_type.clone()),
        };

        unsafe { crate::ffi_helpers::serialize_to_buffer(&msg, output, output_len) }
    }));

    result.unwrap_or(-1)
}

/// Create and initialize an EA Context
///
/// This should be called once in OnInit() and the handle stored in a global variable.
///
/// # Safety
/// All pointers must be valid null-terminated UTF-16 strings
#[no_mangle]
pub unsafe extern "C" fn ea_init(
    account_id: *const u16,
    ea_type: *const u16,
    platform: *const u16,
    account_number: i64,
    broker: *const u16,
    account_name: *const u16,
    server: *const u16,
    currency: *const u16,
    leverage: i64,
) -> *mut EaContext {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let acc_id = match utf16_to_string(account_id) {
            Some(s) => s,
            None => return std::ptr::null_mut(),
        };
        let et = match utf16_to_string(ea_type) {
            Some(s) => s,
            None => return std::ptr::null_mut(),
        };
        let plt = match utf16_to_string(platform) {
            Some(s) => s,
            None => return std::ptr::null_mut(),
        };
        let brk = match utf16_to_string(broker) {
            Some(s) => s,
            None => return std::ptr::null_mut(),
        };
        let acc_name = match utf16_to_string(account_name) {
            Some(s) => s,
            None => return std::ptr::null_mut(),
        };
        let srv = match utf16_to_string(server) {
            Some(s) => s,
            None => return std::ptr::null_mut(),
        };
        let curr = match utf16_to_string(currency) {
            Some(s) => s,
            None => return std::ptr::null_mut(),
        };

        let context = Box::new(crate::EaContext::new(
            acc_id,
            et,
            plt,
            account_number,
            brk,
            acc_name,
            srv,
            curr,
            leverage,
        ));
        Box::into_raw(context)
    }));

    result.unwrap_or(std::ptr::null_mut())
}

/// Free an EA Context instance
///
/// This should be called in OnDeinit() to clean up the state.
///
/// # Safety
/// - `context` must have been returned by `ea_init()`
/// - `context` must not be null
/// - `context` must only be freed once
#[no_mangle]
pub unsafe extern "C" fn ea_context_free(context: *mut crate::EaContext) {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if !context.is_null() {
            let _ = Box::from_raw(context);
        }
    }));
}

/// Mark that a ConfigMessage has been received
///
/// This should be called when the EA receives a ConfigMessage from the relay server.
/// After calling this, `ea_context_should_request_config()` will return false until
/// `ea_context_reset()` is called.
///
/// # Safety
/// - `context` must be a valid pointer returned by `ea_init()`
/// - `context` must not have been freed
#[no_mangle]
pub unsafe extern "C" fn ea_context_mark_config_requested(context: *mut crate::EaContext) {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if !context.is_null() {
            (*context).mark_config_requested();
        }
    }));
}

/// Send a Sync Request (Slave -> Master)
///
/// # Safety
/// - `context` must be a valid pointer returned by `ea_init()`
/// - `master_account` must be a valid null-terminated UTF-16 string
/// - `last_sync_time` can be null (for full sync) or valid UTF-16 string
#[no_mangle]
pub unsafe extern "C" fn ea_send_sync_request(
    context: *mut crate::EaContext,
    master_account: *const u16,
    last_sync_time: *const u16,
) -> i32 {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if context.is_null() {
            return 0;
        }

        let ma = match utf16_to_string(master_account) {
            Some(s) => s,
            None => return 0,
        };

        // last_sync_time can be null or empty
        let lst = if !last_sync_time.is_null() {
            utf16_to_string(last_sync_time)
        } else {
            None
        };

        let ctx = &mut *context;
        match ctx.send_sync_request(&ma, lst) {
            Ok(_) => 1,
            Err(_) => 0,
        }
    }));

    result.unwrap_or(0)
}

/// Reset the EA state to initial conditions
///
/// This should be called when:
/// - Connection to relay server is lost
/// - EA needs to re-request configuration
///
/// After calling this, `ea_context_should_request_config()` will return true on the next call.
///
/// # Safety
/// - `context` must be a valid pointer returned by `ea_init()`
/// - `context` must not have been freed
#[no_mangle]
pub unsafe extern "C" fn ea_context_reset(context: *mut crate::EaContext) {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if !context.is_null() {
            (*context).reset();
        }
    }));
}

// ============================================================================
// Trade Signal FFI Functions (High-Level)
// ============================================================================

/// Send an Open Trade Signal
///
/// # Safety
/// - `context` must be a valid pointer returned by `ea_init()`
/// - `symbol`, `order_type`, `comment` must be valid null-terminated UTF-16 strings
#[no_mangle]
pub unsafe extern "C" fn ea_send_open_signal(
    context: *mut crate::EaContext,
    ticket: i64,
    symbol: *const u16,
    order_type: *const u16,
    lots: f64,
    price: f64,
    sl: f64,
    tp: f64,
    magic: i64,
    comment: *const u16,
) -> i32 {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if context.is_null() {
            return 0;
        }
        let ctx = &mut *context;

        let sym = match utf16_to_string(symbol) {
            Some(s) => s,
            None => return 0,
        };
        let o_type_str = match utf16_to_string(order_type) {
            Some(s) => s,
            None => return 0,
        };
        let o_type = match crate::constants::OrderType::try_parse(&o_type_str) {
            Some(ot) => ot,
            None => return 0,
        };
        let cmt = match utf16_to_string(comment) {
            Some(s) => s,
            None => return 0,
        };

        match ctx.send_open_signal(ticket, &sym, o_type, lots, price, sl, tp, magic, &cmt) {
            Ok(_) => 1,
            Err(_) => 0,
        }
    }));

    result.unwrap_or(0)
}

/// Send a Close Trade Signal
///
/// # Safety
/// - `context` must be a valid pointer returned by `ea_init()`
#[no_mangle]
pub unsafe extern "C" fn ea_send_close_signal(
    context: *mut crate::EaContext,
    ticket: i64,
    lots: f64,
    close_ratio: f64,
) -> i32 {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if context.is_null() {
            return 0;
        }
        let ctx = &mut *context;

        match ctx.send_close_signal(ticket, lots, close_ratio) {
            Ok(_) => 1,
            Err(_) => 0,
        }
    }));

    result.unwrap_or(0)
}

/// Send a Modify Trade Signal
///
/// # Safety
/// - `context` must be a valid pointer returned by `ea_init()`
#[no_mangle]
pub unsafe extern "C" fn ea_send_modify_signal(
    context: *mut crate::EaContext,
    ticket: i64,
    sl: f64,
    tp: f64,
) -> i32 {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if context.is_null() {
            return 0;
        }
        let ctx = &mut *context;

        match ctx.send_modify_signal(ticket, sl, tp) {
            Ok(_) => 1,
            Err(_) => 0,
        }
    }));

    result.unwrap_or(0)
}

/// Main Manager Tick (replaces ea_tick_timer)
/// Handles heartbeat, polling, and internal state
/// Returns 1 if commands are pending, 0 otherwise
///
/// # Safety
/// - context: Valid EaContext pointer
#[no_mangle]
pub unsafe extern "C" fn ea_manager_tick(
    context: *mut EaContext,
    balance: f64,
    equity: f64,
    open_positions: i32,
    is_trade_allowed: i32,
) -> i32 {
    let ctx = match context.as_mut() {
        Some(c) => c,
        None => return 0,
    };
    ctx.manager_tick(balance, equity, open_positions, is_trade_allowed != 0)
}

/// Retrieve the next pending command for MQL
/// Returns 1 if command retrieved, 0 if queue empty
///
/// # Safety
/// - context: Valid EaContext pointer
/// - command: Pointer to allocated EaCommand struct to be filled
#[no_mangle]
pub unsafe extern "C" fn ea_get_command(context: *mut EaContext, command: *mut EaCommand) -> i32 {
    let ctx = match context.as_mut() {
        Some(c) => c,
        None => return 0,
    };
    if command.is_null() {
        return 0;
    }

    if let Some(cmd) = ctx.get_next_command() {
        *command = cmd;
        1
    } else {
        0
    }
}

// ===========================================================================
// New Structure-Based Accessors (Replacing old fine-grained getters)
// ===========================================================================

/// Get the last received Master Config as a C-compatible struct
///
/// # Safety
/// - context: Valid EaContext pointer
/// - config: Pointer to allocated CMasterConfig struct
#[no_mangle]
pub unsafe extern "C" fn ea_context_get_master_config(
    context: *const EaContext,
    config: *mut CMasterConfig,
) -> i32 {
    let ctx = match context.as_ref() {
        Some(c) => c,
        None => return 0,
    };
    if config.is_null() {
        return 0;
    }

    if let Some(src) = &ctx.last_master_config {
        let dest = &mut *config;
        copy_string_to_fixed_array(&src.account_id, &mut dest.account_id);
        dest.status = src.status;
        copy_string_to_fixed_array(
            src.symbol_prefix.as_deref().unwrap_or(""),
            &mut dest.symbol_prefix,
        );
        copy_string_to_fixed_array(
            src.symbol_suffix.as_deref().unwrap_or(""),
            &mut dest.symbol_suffix,
        );
        dest.config_version = src.config_version;
        1
    } else {
        0
    }
}

/// Get the last received Slave Config as a C-compatible struct
///
/// # Safety
/// - context: Valid EaContext pointer
/// - config: Pointer to allocated CSlaveConfig struct
#[no_mangle]
pub unsafe extern "C" fn ea_context_get_slave_config(
    context: *const EaContext,
    config: *mut CSlaveConfig,
) -> i32 {
    let ctx = match context.as_ref() {
        Some(c) => c,
        None => return 0,
    };
    if config.is_null() {
        return 0;
    }

    if let Some(src) = &ctx.last_slave_config {
        let dest = &mut *config;
        copy_string_to_fixed_array(&src.account_id, &mut dest.account_id);
        copy_string_to_fixed_array(&src.master_account, &mut dest.master_account);
        copy_string_to_fixed_array(&src.trade_group_id, &mut dest.trade_group_id);

        dest.status = src.status;
        dest.lot_calculation_mode = match src.lot_calculation_mode {
            LotCalculationMode::Multiplier => 0,
            LotCalculationMode::MarginRatio => 1,
        };
        dest.lot_multiplier = src.lot_multiplier.unwrap_or(0.0);
        dest.reverse_trade = if src.reverse_trade { 1 } else { 0 };

        copy_string_to_fixed_array(
            src.symbol_prefix.as_deref().unwrap_or(""),
            &mut dest.symbol_prefix,
        );
        copy_string_to_fixed_array(
            src.symbol_suffix.as_deref().unwrap_or(""),
            &mut dest.symbol_suffix,
        );

        dest.config_version = src.config_version;
        dest.source_lot_min = src.source_lot_min.unwrap_or(0.0);
        dest.source_lot_max = src.source_lot_max.unwrap_or(0.0);
        dest.master_equity = src.master_equity.unwrap_or(0.0);

        dest.sync_mode = match src.sync_mode {
            SyncMode::Skip => 0,
            SyncMode::LimitOrder => 1,
            SyncMode::MarketOrder => 2,
        };
        dest.limit_order_expiry_min = src.limit_order_expiry_min.unwrap_or(0);
        dest.market_sync_max_pips = src.market_sync_max_pips.unwrap_or(0.0);
        dest.max_slippage = src.max_slippage.unwrap_or(0);
        dest.copy_pending_orders = if src.copy_pending_orders { 1 } else { 0 };

        dest.max_retries = src.max_retries;
        dest.max_signal_delay_ms = src.max_signal_delay_ms;
        dest.use_pending_order_for_delayed = if src.use_pending_order_for_delayed {
            1
        } else {
            0
        };
        dest.allow_new_orders = if src.allow_new_orders { 1 } else { 0 };

        1
    } else {
        0
    }
}

/// Get number of symbol mappings in the last slave config
///
/// # Safety
/// - context: Valid EaContext pointer
#[no_mangle]
pub unsafe extern "C" fn ea_context_get_symbol_mappings_count(context: *const EaContext) -> i32 {
    let ctx = match context.as_ref() {
        Some(c) => c,
        None => return 0,
    };
    if let Some(cfg) = &ctx.last_slave_config {
        cfg.symbol_mappings.len() as i32
    } else {
        0
    }
}

/// Get symbol mappings from the last slave config
///
/// # Safety
/// - mappings: Pointer to array of CSymbolMapping
/// - max_count: Maximum number of mappings to copy
#[no_mangle]
pub unsafe extern "C" fn ea_context_get_symbol_mappings(
    context: *const EaContext,
    mappings: *mut CSymbolMapping,
    max_count: i32,
) -> i32 {
    let ctx = match context.as_ref() {
        Some(c) => c,
        None => return 0,
    };
    if mappings.is_null() || max_count <= 0 {
        return 0;
    }

    if let Some(cfg) = &ctx.last_slave_config {
        let count = cfg.symbol_mappings.len().min(max_count as usize);
        let slice = std::slice::from_raw_parts_mut(mappings, count);

        for (i, src) in cfg.symbol_mappings.iter().take(count).enumerate() {
            copy_string_to_fixed_array(&src.source_symbol, &mut slice[i].source);
            copy_string_to_fixed_array(&src.target_symbol, &mut slice[i].target);
        }
        count as i32
    } else {
        0
    }
}

/// Get number of positions in the last position snapshot
///
/// # Safety
/// - context: Valid EaContext pointer
#[no_mangle]
pub unsafe extern "C" fn ea_context_get_position_snapshot_count(context: *const EaContext) -> i32 {
    let ctx = match context.as_ref() {
        Some(c) => c,
        None => return 0,
    };
    if let Some(snap) = &ctx.last_position_snapshot {
        snap.positions.len() as i32
    } else {
        0
    }
}

/// Get positions from the last position snapshot
///
/// # Safety
/// - positions: Pointer to array of CPositionInfo
/// - max_count: Maximum number of positions to copy
#[no_mangle]
pub unsafe extern "C" fn ea_context_get_position_snapshot(
    context: *const EaContext,
    positions: *mut CPositionInfo,
    max_count: i32,
) -> i32 {
    let ctx = match context.as_ref() {
        Some(c) => c,
        None => return 0,
    };
    if positions.is_null() || max_count <= 0 {
        return 0;
    }

    if let Some(snap) = &ctx.last_position_snapshot {
        let count = snap.positions.len().min(max_count as usize);
        let slice = std::slice::from_raw_parts_mut(positions, count);

        for (i, src) in snap.positions.iter().take(count).enumerate() {
            slice[i].ticket = src.ticket;
            copy_string_to_fixed_array(&src.symbol, &mut slice[i].symbol);

            // Map string order type to integer
            // Simple mapping for now: Buy=0, Sell=1, etc.
            // But src.order_type is a string "Buy", "Sell" etc.
            // Ideally we should parse it.
            // Using OrderType::try_parse logic:
            let ot = crate::constants::OrderType::try_parse(&src.order_type)
                .unwrap_or(crate::constants::OrderType::Buy); // Default fallback
            slice[i].order_type = i32::from(ot);

            slice[i].lots = src.lots;
            slice[i].open_price = src.open_price;

            // Open Time: String to Timestamp conversion
            // src.open_time is ISO8601 string.
            if let Ok(dt) = DateTime::parse_from_rfc3339(&src.open_time) {
                slice[i].open_time = dt.timestamp();
            } else {
                slice[i].open_time = 0;
            }

            slice[i].stop_loss = src.stop_loss.unwrap_or(0.0);
            slice[i].take_profit = src.take_profit.unwrap_or(0.0);
            slice[i].magic_number = src.magic_number.unwrap_or(0);

            copy_string_to_fixed_array(src.comment.as_deref().unwrap_or(""), &mut slice[i].comment);
        }
        count as i32
    } else {
        0
    }
}

/// Get the source account from the last position snapshot
///
/// # Safety
/// - context: Valid EaContext pointer
/// - buffer: Pointer to buffer for the source account string
/// - len: Size of buffer (should be at least MAX_ACCOUNT_ID_LEN)
#[no_mangle]
pub unsafe extern "C" fn ea_context_get_position_snapshot_source_account(
    context: *const EaContext,
    buffer: *mut u8,
    len: i32,
) -> i32 {
    let ctx = match context.as_ref() {
        Some(c) => c,
        None => return 0,
    };
    if buffer.is_null() || len <= 0 {
        return 0;
    }

    if let Some(snap) = &ctx.last_position_snapshot {
        // Convert to fixed array manually since we have raw pointer
        let slice = std::slice::from_raw_parts_mut(buffer, len as usize);
        // Use helper or manual
        // Since helper takes [u8; N], we can't easily use it with dynamic len slice.
        // We can reimplement logic here.
        let s = &snap.source_account;
        let max_len = (len - 1) as usize;
        let bytes = if s.len() <= max_len {
            s.as_bytes()
        } else {
            let mut end = max_len;
            while end > 0 && !crate::ffi_helpers::is_char_boundary(s, end) {
                end -= 1;
            }
            &s.as_bytes()[..end]
        };
        slice[..bytes.len()].copy_from_slice(bytes);
        slice[bytes.len()..].fill(0);
        1
    } else {
        0
    }
}

/// Get the last received SyncRequest as a C-compatible struct
///
/// # Safety
/// - context: Valid EaContext pointer
/// - request: Pointer to allocated CSyncRequest struct
#[no_mangle]
pub unsafe extern "C" fn ea_context_get_sync_request(
    context: *const EaContext,
    request: *mut CSyncRequest,
) -> i32 {
    let ctx = match context.as_ref() {
        Some(c) => c,
        None => return 0,
    };
    if request.is_null() {
        return 0;
    }

    if let Some(src) = &ctx.last_sync_request {
        let dest = &mut *request;
        copy_string_to_fixed_array(&src.slave_account, &mut dest.slave_account);
        copy_string_to_fixed_array(&src.master_account, &mut dest.master_account);
        copy_string_to_fixed_array(
            src.last_sync_time.as_deref().unwrap_or(""),
            &mut dest.last_sync_time,
        );
        1
    } else {
        0
    }
}
