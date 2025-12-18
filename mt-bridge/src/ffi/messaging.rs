use crate::ea_context::{EaCommand, EaContext};
use crate::ffi::helpers::utf16_to_string;

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

/// Create and serialize a RegisterMessage using context data (using Rust-side SymbolMatcher)
///
/// # Safety
/// - `context` must be a valid pointer returned by `ea_init()`
/// - `output` must be a valid buffer
#[no_mangle]
pub unsafe extern "C" fn ea_send_register(
    context: *mut crate::EaContext,
    output: *mut u8,
    output_len: i32,
    detected_prefix: *const u16,
    detected_suffix: *const u16,
    detected_specials: *const u16,
    is_trade_allowed: i32, // Added argument
) -> i32 {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if context.is_null() {
            return -1;
        }

        let ctx = &mut *context;
        let is_trade_allowed_bool = is_trade_allowed != 0;

        // Update context state to avoid redundant ConfigRequest in heartbeat
        ctx.last_trade_allowed = is_trade_allowed_bool;
        // Register implies we want config, and we get it automatically.
        // We can mark it as requested to be safe, though send_heartbeat doesn't check this flag for trade_allowed trigger.
        ctx.mark_config_requested();

        // Parse Context Info
        let prefix = if !detected_prefix.is_null() {
            crate::ffi::helpers::utf16_to_string(detected_prefix).unwrap_or_default()
        } else {
            String::new()
        };

        let suffix = if !detected_suffix.is_null() {
            crate::ffi::helpers::utf16_to_string(detected_suffix).unwrap_or_default()
        } else {
            String::new()
        };

        let specials = if !detected_specials.is_null() {
            crate::ffi::helpers::utf16_to_string(detected_specials)
                .map(|s| {
                    s.split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<String>>()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        let symbol_context = if !prefix.is_empty() || !suffix.is_empty() || !specials.is_empty() {
            Some(crate::types::SymbolContext {
                detected_prefix: prefix,
                detected_suffix: suffix,
                available_special_symbols: specials,
            })
        } else {
            None
        };

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
            symbol_context,
            is_trade_allowed: is_trade_allowed_bool,
        };

        unsafe { crate::ffi::helpers::serialize_to_buffer(&msg, output, output_len) }
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

        unsafe { crate::ffi::helpers::serialize_to_buffer(&msg, output, output_len) }
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
            timestamp: chrono::Utc::now().timestamp_millis(),
            ea_type: Some(ctx.ea_type.clone()),
        };

        unsafe { crate::ffi::helpers::serialize_to_buffer(&msg, output, output_len) }
    }));

    result.unwrap_or(-1)
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
