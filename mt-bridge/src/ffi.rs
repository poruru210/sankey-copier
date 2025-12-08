// Location: mt-bridge/src/ffi.rs
// Purpose: Unified FFI functions for MQL4/MQL5 integration (ZMQ + MessagePack)
// Why: Provides C-compatible interface for ZMQ operations and MessagePack message handling

use crate::constants::{self, TOPIC_GLOBAL_CONFIG};
use crate::ea_context::EaContext;
use crate::ffi_helpers::{
    free_handle, parse_msgpack, string_to_utf16_buffer, utf16_to_string, BUFFER_INDEX,
    MAX_STRING_LEN, STRING_BUFFER_1, STRING_BUFFER_2, STRING_BUFFER_3, STRING_BUFFER_4,
};
use crate::types::{
    LotCalculationMode, MasterConfigMessage, PositionInfo, PositionSnapshotMessage,
    SlaveConfigMessage, SyncMode, SyncRequestMessage, TradeSignalMessage, VLogsConfigMessage,
};
use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;
use std::sync::{LazyLock, Mutex};

/// Parse MessagePack data as Slave ConfigMessage and return an opaque handle
///
/// # Safety
/// This function is unsafe because it dereferences raw pointers.
/// The returned handle must be freed with `slave_config_free()`.
#[no_mangle]
pub unsafe extern "C" fn parse_slave_config(
    data: *const u8,
    data_len: i32,
) -> *mut SlaveConfigMessage {
    parse_msgpack(data, data_len)
}

/// Free a Slave ConfigMessage handle
///
/// # Safety
/// This function is unsafe because it takes ownership of a raw pointer.
/// The caller must ensure:
/// - `handle` was returned by `parse_slave_config()`
/// - `handle` is only freed once
#[no_mangle]
pub unsafe extern "C" fn slave_config_free(handle: *mut SlaveConfigMessage) {
    free_handle(handle)
}

/// Parse MessagePack data as MasterConfigMessage and return an opaque handle
///
/// # Safety
/// This function is unsafe because it dereferences raw pointers.
/// The returned handle must be freed with `master_config_free()`.
#[no_mangle]
pub unsafe extern "C" fn parse_master_config(
    data: *const u8,
    data_len: i32,
) -> *mut MasterConfigMessage {
    parse_msgpack(data, data_len)
}

/// Free a MasterConfigMessage handle
///
/// # Safety
/// This function is unsafe because it takes ownership of a raw pointer.
/// The caller must ensure:
/// - `handle` was returned by `parse_master_config()`
/// - `handle` is only freed once
#[no_mangle]
pub unsafe extern "C" fn master_config_free(handle: *mut MasterConfigMessage) {
    free_handle(handle)
}

/// Get a string field from Slave ConfigMessage handle
///
/// # Safety
/// This function is unsafe because it dereferences raw pointers.
/// Returns a pointer to a static UTF-16 buffer (valid until next 4 calls).
#[no_mangle]
pub unsafe extern "C" fn slave_config_get_string(
    handle: *const SlaveConfigMessage,
    field_name: *const u16,
) -> *const u16 {
    if handle.is_null() || field_name.is_null() {
        return std::ptr::null();
    }

    let config = &*handle;

    // Parse field name from UTF-16
    let mut len = 0;
    while *field_name.add(len) != 0 {
        len += 1;
    }
    let field_slice = std::slice::from_raw_parts(field_name, len);
    let field = match String::from_utf16(field_slice) {
        Ok(s) => s,
        Err(_) => return std::ptr::null(),
    };

    // Use static strings for enum values and empty string
    static EMPTY_STRING: LazyLock<String> = LazyLock::new(String::new);
    static SYNC_MODE_SKIP: LazyLock<String> = LazyLock::new(|| "skip".to_string());
    static SYNC_MODE_LIMIT_ORDER: LazyLock<String> = LazyLock::new(|| "limit_order".to_string());
    static SYNC_MODE_MARKET_ORDER: LazyLock<String> = LazyLock::new(|| "market_order".to_string());
    static LOT_CALC_MODE_MULTIPLIER: LazyLock<String> = LazyLock::new(|| "multiplier".to_string());
    static LOT_CALC_MODE_MARGIN_RATIO: LazyLock<String> =
        LazyLock::new(|| "margin_ratio".to_string());

    let value = match field.as_str() {
        "account_id" => &config.account_id,
        "master_account" => &config.master_account,
        "trade_group_id" => &config.trade_group_id,
        "timestamp" => &config.timestamp,
        "symbol_prefix" => config.symbol_prefix.as_ref().unwrap_or(&EMPTY_STRING),
        "symbol_suffix" => config.symbol_suffix.as_ref().unwrap_or(&EMPTY_STRING),
        "lot_calculation_mode" => match config.lot_calculation_mode {
            LotCalculationMode::Multiplier => &*LOT_CALC_MODE_MULTIPLIER,
            LotCalculationMode::MarginRatio => &*LOT_CALC_MODE_MARGIN_RATIO,
        },
        "sync_mode" => match config.sync_mode {
            SyncMode::Skip => &*SYNC_MODE_SKIP,
            SyncMode::LimitOrder => &*SYNC_MODE_LIMIT_ORDER,
            SyncMode::MarketOrder => &*SYNC_MODE_MARKET_ORDER,
        },
        _ => return std::ptr::null(),
    };

    // Get next buffer in round-robin fashion
    let mut index = BUFFER_INDEX.lock().unwrap();
    let current_index = *index;
    *index = (*index + 1) % 4;
    drop(index);

    // Select buffer based on index
    let buffer_mutex = match current_index {
        0 => &STRING_BUFFER_1,
        1 => &STRING_BUFFER_2,
        2 => &STRING_BUFFER_3,
        _ => &STRING_BUFFER_4,
    };

    let mut buffer = buffer_mutex.lock().unwrap();

    // Convert to UTF-16 and copy to buffer
    let utf16: Vec<u16> = value.encode_utf16().chain(std::iter::once(0)).collect();
    let copy_len = utf16.len().min(MAX_STRING_LEN - 1);
    buffer[..copy_len].copy_from_slice(&utf16[..copy_len]);
    buffer[copy_len] = 0; // Ensure null termination

    buffer.as_ptr()
}

/// Get a double field from Slave ConfigMessage handle
///
/// # Safety
/// - handle must be a valid pointer to SlaveConfigMessage
/// - field_name must be a valid null-terminated UTF-16 string pointer
#[no_mangle]
pub unsafe extern "C" fn slave_config_get_double(
    handle: *const SlaveConfigMessage,
    field_name: *const u16,
) -> f64 {
    if handle.is_null() || field_name.is_null() {
        return 0.0;
    }

    let config = &*handle;

    // Parse field name
    let mut len = 0;
    while *field_name.add(len) != 0 {
        len += 1;
    }
    let field_slice = std::slice::from_raw_parts(field_name, len);
    let field = match String::from_utf16(field_slice) {
        Ok(s) => s,
        Err(_) => return 0.0,
    };

    match field.as_str() {
        "lot_multiplier" => config.lot_multiplier.unwrap_or(1.0),
        "source_lot_min" => config.source_lot_min.unwrap_or(0.0),
        "source_lot_max" => config.source_lot_max.unwrap_or(0.0),
        "master_equity" => config.master_equity.unwrap_or(0.0),
        "market_sync_max_pips" => config.market_sync_max_pips.unwrap_or(0.0),
        _ => 0.0,
    }
}

/// Get a boolean field from Slave ConfigMessage handle
///
/// # Safety
/// - handle must be a valid pointer to SlaveConfigMessage
/// - field_name must be a valid null-terminated UTF-16 string pointer
#[no_mangle]
pub unsafe extern "C" fn slave_config_get_bool(
    handle: *const SlaveConfigMessage,
    field_name: *const u16,
) -> i32 {
    if handle.is_null() || field_name.is_null() {
        return 0;
    }

    let config = &*handle;

    // Parse field name
    let mut len = 0;
    while *field_name.add(len) != 0 {
        len += 1;
    }
    let field_slice = std::slice::from_raw_parts(field_name, len);
    let field = match String::from_utf16(field_slice) {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let result = match field.as_str() {
        "reverse_trade" => config.reverse_trade,
        "copy_pending_orders" => config.copy_pending_orders,
        "use_pending_order_for_delayed" => config.use_pending_order_for_delayed,
        "allow_new_orders" => config.allow_new_orders,
        _ => false,
    };

    if result {
        1
    } else {
        0
    }
}

/// Get an integer field from Slave ConfigMessage handle
///
/// # Safety
/// - handle must be a valid pointer to SlaveConfigMessage
/// - field_name must be a valid null-terminated UTF-16 string pointer
#[no_mangle]
pub unsafe extern "C" fn slave_config_get_int(
    handle: *const SlaveConfigMessage,
    field_name: *const u16,
) -> i32 {
    if handle.is_null() || field_name.is_null() {
        return 0;
    }

    let config = &*handle;

    // Parse field name
    let mut len = 0;
    while *field_name.add(len) != 0 {
        len += 1;
    }
    let field_slice = std::slice::from_raw_parts(field_name, len);
    let field = match String::from_utf16(field_slice) {
        Ok(s) => s,
        Err(_) => return 0,
    };

    match field.as_str() {
        "config_version" => config.config_version as i32,
        "status" => config.status,
        "max_slippage" => config.max_slippage.unwrap_or(30), // default: 30 points
        "limit_order_expiry_min" => config.limit_order_expiry_min.unwrap_or(0), // default: 0 (GTC)
        "max_retries" => config.max_retries,
        "max_signal_delay_ms" => config.max_signal_delay_ms,
        _ => 0,
    }
}

/// Get a string field from MasterConfigMessage handle
///
/// # Safety
/// This function is unsafe because it dereferences raw pointers.
/// Returns a pointer to a static UTF-16 buffer (valid until next 4 calls).
#[no_mangle]
pub unsafe extern "C" fn master_config_get_string(
    handle: *const MasterConfigMessage,
    field_name: *const u16,
) -> *const u16 {
    if handle.is_null() || field_name.is_null() {
        return std::ptr::null();
    }

    let config = &*handle;

    // Parse field name from UTF-16
    let mut len = 0;
    while *field_name.add(len) != 0 {
        len += 1;
    }
    let field_slice = std::slice::from_raw_parts(field_name, len);
    let field = match String::from_utf16(field_slice) {
        Ok(s) => s,
        Err(_) => return std::ptr::null(),
    };

    // Use a static empty string to avoid temporary value dropped error
    static EMPTY_STRING: LazyLock<String> = LazyLock::new(String::new);

    let value = match field.as_str() {
        "account_id" => &config.account_id,
        "timestamp" => &config.timestamp,
        "symbol_prefix" => config.symbol_prefix.as_ref().unwrap_or(&EMPTY_STRING),
        "symbol_suffix" => config.symbol_suffix.as_ref().unwrap_or(&EMPTY_STRING),
        _ => return std::ptr::null(),
    };

    // Get next buffer in round-robin fashion
    let mut index = BUFFER_INDEX.lock().unwrap();
    let current_index = *index;
    *index = (*index + 1) % 4;
    drop(index);

    // Select buffer based on index
    let buffer_mutex = match current_index {
        0 => &STRING_BUFFER_1,
        1 => &STRING_BUFFER_2,
        2 => &STRING_BUFFER_3,
        _ => &STRING_BUFFER_4,
    };

    let mut buffer = buffer_mutex.lock().unwrap();

    // Convert to UTF-16 and copy to buffer
    let utf16: Vec<u16> = value.encode_utf16().chain(std::iter::once(0)).collect();
    let copy_len = utf16.len().min(MAX_STRING_LEN - 1);
    buffer[..copy_len].copy_from_slice(&utf16[..copy_len]);
    buffer[copy_len] = 0; // Ensure null termination

    buffer.as_ptr()
}

// ===========================================================================
// Symbol Mapping Array Access Functions
// ===========================================================================

/// Get the number of symbol mappings in a SlaveConfigMessage
///
/// # Safety
/// - handle must be a valid pointer to SlaveConfigMessage
#[no_mangle]
pub unsafe extern "C" fn slave_config_get_symbol_mappings_count(
    handle: *const SlaveConfigMessage,
) -> i32 {
    if handle.is_null() {
        return 0;
    }
    let config = &*handle;
    config.symbol_mappings.len() as i32
}

/// Get the source symbol at a specific index from symbol_mappings
///
/// # Safety
/// - handle must be a valid pointer to SlaveConfigMessage
/// - index must be within bounds (0 <= index < count)
#[no_mangle]
pub unsafe extern "C" fn slave_config_get_symbol_mapping_source(
    handle: *const SlaveConfigMessage,
    index: i32,
) -> *const u16 {
    if handle.is_null() || index < 0 {
        return std::ptr::null();
    }
    let config = &*handle;
    let idx = index as usize;
    if idx >= config.symbol_mappings.len() {
        return std::ptr::null();
    }
    string_to_utf16_buffer(&config.symbol_mappings[idx].source_symbol)
}

/// Get the target symbol at a specific index from symbol_mappings
///
/// # Safety
/// - handle must be a valid pointer to SlaveConfigMessage
/// - index must be within bounds (0 <= index < count)
#[no_mangle]
pub unsafe extern "C" fn slave_config_get_symbol_mapping_target(
    handle: *const SlaveConfigMessage,
    index: i32,
) -> *const u16 {
    if handle.is_null() || index < 0 {
        return std::ptr::null();
    }
    let config = &*handle;
    let idx = index as usize;
    if idx >= config.symbol_mappings.len() {
        return std::ptr::null();
    }
    string_to_utf16_buffer(&config.symbol_mappings[idx].target_symbol)
}

// ===========================================================================
// Filter Array Access Functions (allowed_magic_numbers)
// ===========================================================================

/// Get the number of allowed magic numbers in a SlaveConfigMessage
///
/// # Safety
/// - handle must be a valid pointer to SlaveConfigMessage
#[no_mangle]
pub unsafe extern "C" fn slave_config_get_allowed_magic_count(
    handle: *const SlaveConfigMessage,
) -> i32 {
    if handle.is_null() {
        return 0;
    }
    let config = &*handle;
    match &config.filters.allowed_magic_numbers {
        Some(v) => v.len() as i32,
        None => 0,
    }
}

/// Get the allowed magic number at a specific index
///
/// # Safety
/// - handle must be a valid pointer to SlaveConfigMessage
/// - index must be within bounds (0 <= index < count)
#[no_mangle]
pub unsafe extern "C" fn slave_config_get_allowed_magic_at(
    handle: *const SlaveConfigMessage,
    index: i32,
) -> i32 {
    if handle.is_null() || index < 0 {
        return 0;
    }
    let config = &*handle;
    let idx = index as usize;
    match &config.filters.allowed_magic_numbers {
        Some(v) if idx < v.len() => v[idx],
        _ => 0,
    }
}

/// Get an integer field from MasterConfigMessage handle
///
/// # Safety
/// - handle must be a valid pointer to MasterConfigMessage
/// - field_name must be a valid null-terminated UTF-16 string pointer
#[no_mangle]
pub unsafe extern "C" fn master_config_get_int(
    handle: *const MasterConfigMessage,
    field_name: *const u16,
) -> i32 {
    if handle.is_null() || field_name.is_null() {
        return 0;
    }

    let config = &*handle;

    // Parse field name
    let mut len = 0;
    while *field_name.add(len) != 0 {
        len += 1;
    }
    let field_slice = std::slice::from_raw_parts(field_name, len);
    let field = match String::from_utf16(field_slice) {
        Ok(s) => s,
        Err(_) => return 0,
    };

    match field.as_str() {
        "config_version" => config.config_version as i32,
        "status" => config.status,
        _ => 0,
    }
}

/// Free a string allocated by msgpack_deserialize_config
///
/// # Safety
/// This function is unsafe because it takes ownership of a raw pointer.
/// The caller must ensure:
/// - `ptr` was returned by `msgpack_deserialize_config`
/// - `ptr` is only freed once
#[no_mangle]
pub unsafe extern "C" fn free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(CString::from_raw(ptr));
    }
}

/// Parse a TradeSignalMessage from MessagePack data
///
/// # Safety
/// - data must be a valid pointer to a buffer of at least data_len bytes
/// - data_len must accurately represent the buffer size
#[no_mangle]
pub unsafe extern "C" fn parse_trade_signal(
    data: *const u8,
    data_len: i32,
) -> *mut TradeSignalMessage {
    parse_msgpack(data, data_len)
}

/// Free a TradeSignalMessage handle
///
/// # Safety
/// - handle must be a valid pointer created by parse_trade_signal or null
/// - handle must not be used after calling this function
#[no_mangle]
pub unsafe extern "C" fn trade_signal_free(handle: *mut TradeSignalMessage) {
    free_handle(handle)
}

/// Get a string field from TradeSignalMessage handle
///
/// # Safety
/// - handle must be a valid pointer to TradeSignalMessage
/// - field_name must be a valid null-terminated UTF-16 string pointer
#[no_mangle]
pub unsafe extern "C" fn trade_signal_get_string(
    handle: *const TradeSignalMessage,
    field_name: *const u16,
) -> *const u16 {
    if handle.is_null() || field_name.is_null() {
        return std::ptr::null();
    }

    let msg = &*handle;
    let field = match utf16_to_string(field_name) {
        Some(s) => s,
        None => return std::ptr::null(),
    };

    let value = match field.as_str() {
        "action" => Some(&msg.action),
        "symbol" => msg.symbol.as_ref(),
        "order_type" => msg.order_type.as_ref(),
        "comment" => msg.comment.as_ref(),
        "timestamp" => Some(&msg.timestamp),
        "source_account" => Some(&msg.source_account),
        _ => return std::ptr::null(),
    };

    match value {
        Some(s) => string_to_utf16_buffer(s),
        None => std::ptr::null(),
    }
}

/// Get a numeric field from TradeSignalMessage handle
///
/// # Safety
/// - handle must be a valid pointer to TradeSignalMessage
/// - field_name must be a valid null-terminated UTF-16 string pointer
#[no_mangle]
pub unsafe extern "C" fn trade_signal_get_double(
    handle: *const TradeSignalMessage,
    field_name: *const u16,
) -> f64 {
    if handle.is_null() || field_name.is_null() {
        return 0.0;
    }

    let msg = &*handle;
    let field = match utf16_to_string(field_name) {
        Some(s) => s,
        None => return 0.0,
    };

    match field.as_str() {
        "lots" => msg.lots.unwrap_or(0.0),
        "open_price" => msg.open_price.unwrap_or(0.0),
        "stop_loss" => msg.stop_loss.unwrap_or(0.0),
        "take_profit" => msg.take_profit.unwrap_or(0.0),
        "close_ratio" => msg.close_ratio.unwrap_or(0.0),
        _ => 0.0,
    }
}

/// Get an integer field from TradeSignalMessage handle
///
/// # Safety
/// - handle must be a valid pointer to TradeSignalMessage
/// - field_name must be a valid null-terminated UTF-16 string pointer
#[no_mangle]
pub unsafe extern "C" fn trade_signal_get_int(
    handle: *const TradeSignalMessage,
    field_name: *const u16,
) -> i64 {
    if handle.is_null() || field_name.is_null() {
        return 0;
    }

    let msg = &*handle;
    let field = match utf16_to_string(field_name) {
        Some(s) => s,
        None => return 0,
    };

    match field.as_str() {
        "ticket" => msg.ticket,
        "magic_number" => msg.magic_number.unwrap_or(0),
        _ => 0,
    }
}

// ===========================================================================
// PositionSnapshot FFI Functions
// ===========================================================================

/// Parse MessagePack data as PositionSnapshotMessage and return an opaque handle
///
/// # Safety
/// This function is unsafe because it dereferences raw pointers.
/// The returned handle must be freed with `position_snapshot_free()`.
#[no_mangle]
pub unsafe extern "C" fn parse_position_snapshot(
    data: *const u8,
    data_len: i32,
) -> *mut PositionSnapshotMessage {
    parse_msgpack(data, data_len)
}

/// Free a PositionSnapshotMessage handle
///
/// # Safety
/// - handle must be a valid pointer created by parse_position_snapshot or null
/// - handle must not be used after calling this function
#[no_mangle]
pub unsafe extern "C" fn position_snapshot_free(handle: *mut PositionSnapshotMessage) {
    free_handle(handle)
}

/// Get a string field from PositionSnapshotMessage handle
///
/// # Safety
/// - handle must be a valid pointer to PositionSnapshotMessage
/// - field_name must be a valid null-terminated UTF-16 string pointer
#[no_mangle]
pub unsafe extern "C" fn position_snapshot_get_string(
    handle: *const PositionSnapshotMessage,
    field_name: *const u16,
) -> *const u16 {
    if handle.is_null() || field_name.is_null() {
        return std::ptr::null();
    }

    let snapshot = &*handle;
    let field = match utf16_to_string(field_name) {
        Some(s) => s,
        None => return std::ptr::null(),
    };

    let value = match field.as_str() {
        "message_type" => &snapshot.message_type,
        "source_account" => &snapshot.source_account,
        "timestamp" => &snapshot.timestamp,
        _ => return std::ptr::null(),
    };

    string_to_utf16_buffer(value)
}

/// Get the number of positions in a PositionSnapshotMessage
///
/// # Safety
/// - handle must be a valid pointer to PositionSnapshotMessage
#[no_mangle]
pub unsafe extern "C" fn position_snapshot_get_positions_count(
    handle: *const PositionSnapshotMessage,
) -> i32 {
    if handle.is_null() {
        return 0;
    }
    let snapshot = &*handle;
    snapshot.positions.len() as i32
}

/// Get a string field from a position at the specified index
///
/// # Safety
/// - handle must be a valid pointer to PositionSnapshotMessage
/// - index must be within bounds (0 <= index < count)
/// - field_name must be a valid null-terminated UTF-16 string pointer
#[no_mangle]
pub unsafe extern "C" fn position_snapshot_get_position_string(
    handle: *const PositionSnapshotMessage,
    index: i32,
    field_name: *const u16,
) -> *const u16 {
    if handle.is_null() || index < 0 || field_name.is_null() {
        return std::ptr::null();
    }

    let snapshot = &*handle;
    let idx = index as usize;
    if idx >= snapshot.positions.len() {
        return std::ptr::null();
    }

    let pos = &snapshot.positions[idx];
    let field = match utf16_to_string(field_name) {
        Some(s) => s,
        None => return std::ptr::null(),
    };

    // For optional string fields, use static empty string
    static EMPTY_STRING: LazyLock<String> = LazyLock::new(String::new);

    let value = match field.as_str() {
        "symbol" => &pos.symbol,
        "order_type" => &pos.order_type,
        "open_time" => &pos.open_time,
        "comment" => pos.comment.as_ref().unwrap_or(&EMPTY_STRING),
        _ => return std::ptr::null(),
    };

    string_to_utf16_buffer(value)
}

/// Get a double field from a position at the specified index
///
/// # Safety
/// - handle must be a valid pointer to PositionSnapshotMessage
/// - index must be within bounds (0 <= index < count)
/// - field_name must be a valid null-terminated UTF-16 string pointer
#[no_mangle]
pub unsafe extern "C" fn position_snapshot_get_position_double(
    handle: *const PositionSnapshotMessage,
    index: i32,
    field_name: *const u16,
) -> f64 {
    if handle.is_null() || index < 0 || field_name.is_null() {
        return 0.0;
    }

    let snapshot = &*handle;
    let idx = index as usize;
    if idx >= snapshot.positions.len() {
        return 0.0;
    }

    let pos = &snapshot.positions[idx];
    let field = match utf16_to_string(field_name) {
        Some(s) => s,
        None => return 0.0,
    };

    match field.as_str() {
        "lots" => pos.lots,
        "open_price" => pos.open_price,
        "stop_loss" => pos.stop_loss.unwrap_or(0.0),
        "take_profit" => pos.take_profit.unwrap_or(0.0),
        _ => 0.0,
    }
}

/// Get an integer field from a position at the specified index
///
/// # Safety
/// - handle must be a valid pointer to PositionSnapshotMessage
/// - index must be within bounds (0 <= index < count)
/// - field_name must be a valid null-terminated UTF-16 string pointer
#[no_mangle]
pub unsafe extern "C" fn position_snapshot_get_position_int(
    handle: *const PositionSnapshotMessage,
    index: i32,
    field_name: *const u16,
) -> i64 {
    if handle.is_null() || index < 0 || field_name.is_null() {
        return 0;
    }

    let snapshot = &*handle;
    let idx = index as usize;
    if idx >= snapshot.positions.len() {
        return 0;
    }

    let pos = &snapshot.positions[idx];
    let field = match utf16_to_string(field_name) {
        Some(s) => s,
        None => return 0,
    };

    match field.as_str() {
        "ticket" => pos.ticket,
        "magic_number" => pos.magic_number.unwrap_or(0),
        _ => 0,
    }
}

// ===========================================================================
// SyncRequest FFI Functions
// ===========================================================================

/// Create and serialize a SyncRequestMessage to MessagePack
/// Returns the number of bytes written to the output buffer, or 0 on error
///
/// # Safety
/// - slave_account must be a valid null-terminated UTF-16 string pointer
/// - master_account must be a valid null-terminated UTF-16 string pointer
/// - output must be a valid buffer of at least output_len bytes
#[no_mangle]
pub unsafe extern "C" fn create_sync_request(
    slave_account: *const u16,
    master_account: *const u16,
    output: *mut u8,
    output_len: i32,
) -> i32 {
    if slave_account.is_null() || master_account.is_null() || output.is_null() || output_len <= 0 {
        return 0;
    }

    let slave_str = match utf16_to_string(slave_account) {
        Some(s) => s,
        None => return 0,
    };

    let master_str = match utf16_to_string(master_account) {
        Some(s) => s,
        None => return 0,
    };

    let msg = SyncRequestMessage {
        message_type: "SyncRequest".to_string(),
        slave_account: slave_str,
        master_account: master_str,
        last_sync_time: None, // Not tracking sync history yet
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    match rmp_serde::to_vec_named(&msg) {
        Ok(bytes) => {
            if bytes.len() > output_len as usize {
                return 0; // Buffer too small
            }
            let out_slice = std::slice::from_raw_parts_mut(output, bytes.len());
            out_slice.copy_from_slice(&bytes);
            bytes.len() as i32
        }
        Err(_) => 0,
    }
}

/// Parse a SyncRequestMessage from MessagePack data
/// Returns a pointer to the parsed message, or null on error
///
/// # Safety
/// - data must be a valid buffer of at least data_len bytes
/// - The returned handle must be freed with `sync_request_free()`
#[no_mangle]
pub unsafe extern "C" fn parse_sync_request(
    data: *const u8,
    data_len: i32,
) -> *mut SyncRequestMessage {
    parse_msgpack(data, data_len)
}

/// Get a string field from SyncRequestMessage
///
/// # Safety
/// - handle must be a valid pointer to SyncRequestMessage
/// - field_name must be a valid null-terminated UTF-16 string pointer
/// - Caller is responsible for the returned string memory
#[no_mangle]
pub unsafe extern "C" fn sync_request_get_string(
    handle: *const SyncRequestMessage,
    field_name: *const u16,
) -> *const u16 {
    if handle.is_null() || field_name.is_null() {
        return std::ptr::null();
    }

    let msg = &*handle;
    let field = match utf16_to_string(field_name) {
        Some(s) => s,
        None => return std::ptr::null(),
    };

    // Handle last_sync_time separately since it's Option<String>
    if field == "last_sync_time" {
        let value = msg.last_sync_time.as_deref().unwrap_or("");
        return string_to_utf16_buffer(value);
    }

    let value = match field.as_str() {
        "message_type" => &msg.message_type,
        "slave_account" => &msg.slave_account,
        "master_account" => &msg.master_account,
        "timestamp" => &msg.timestamp,
        _ => return std::ptr::null(),
    };

    string_to_utf16_buffer(value)
}

/// Free a parsed SyncRequestMessage
///
/// # Safety
/// - handle must be a valid pointer returned by `parse_sync_request()`
/// - The handle must not be used after calling this function
#[no_mangle]
pub unsafe extern "C" fn sync_request_free(handle: *mut SyncRequestMessage) {
    free_handle(handle)
}

// ===========================================================================
// PositionSnapshot Builder FFI Functions (for Master EA)
// ===========================================================================

/// Create a new PositionSnapshotMessage builder
///
/// # Safety
/// - source_account must be a valid null-terminated UTF-16 string pointer
/// - The returned handle must be freed with `position_snapshot_builder_free()`
#[no_mangle]
pub unsafe extern "C" fn create_position_snapshot_builder(
    source_account: *const u16,
) -> *mut PositionSnapshotMessage {
    if source_account.is_null() {
        return std::ptr::null_mut();
    }

    let account_str = match utf16_to_string(source_account) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    let snapshot = PositionSnapshotMessage {
        message_type: "PositionSnapshot".to_string(),
        source_account: account_str,
        positions: Vec::new(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    Box::into_raw(Box::new(snapshot))
}

/// Add a position to the PositionSnapshotMessage builder
///
/// # Safety
/// - handle must be a valid pointer to PositionSnapshotMessage
/// - symbol, order_type, open_time must be valid null-terminated UTF-16 string pointers
#[no_mangle]
pub unsafe extern "C" fn position_snapshot_builder_add_position(
    handle: *mut PositionSnapshotMessage,
    ticket: i64,
    symbol: *const u16,
    order_type: *const u16,
    lots: f64,
    open_price: f64,
    stop_loss: f64,
    take_profit: f64,
    magic_number: i64,
    open_time: *const u16,
) -> i32 {
    if handle.is_null() || symbol.is_null() || order_type.is_null() || open_time.is_null() {
        return 0;
    }

    let snapshot = &mut *handle;

    let symbol_str = match utf16_to_string(symbol) {
        Some(s) => s,
        None => return 0,
    };

    let order_type_str = match utf16_to_string(order_type) {
        Some(s) => s,
        None => return 0,
    };

    let open_time_str = match utf16_to_string(open_time) {
        Some(s) => s,
        None => return 0,
    };

    let position = PositionInfo {
        ticket,
        symbol: symbol_str,
        order_type: order_type_str,
        lots,
        open_price,
        open_time: open_time_str,
        stop_loss: if stop_loss > 0.0 {
            Some(stop_loss)
        } else {
            None
        },
        take_profit: if take_profit > 0.0 {
            Some(take_profit)
        } else {
            None
        },
        magic_number: if magic_number != 0 {
            Some(magic_number)
        } else {
            None
        },
        comment: None, // EA doesn't need to send comment for sync
    };

    snapshot.positions.push(position);
    1 // Success
}

/// Serialize the PositionSnapshotMessage to MessagePack
/// Returns the number of bytes written, or 0 on error
///
/// # Safety
/// - handle must be a valid pointer to PositionSnapshotMessage
/// - output must be a valid buffer of at least output_len bytes
#[no_mangle]
pub unsafe extern "C" fn position_snapshot_builder_serialize(
    handle: *const PositionSnapshotMessage,
    output: *mut u8,
    output_len: i32,
) -> i32 {
    if handle.is_null() || output.is_null() || output_len <= 0 {
        return 0;
    }

    let snapshot = &*handle;

    // Update timestamp to current time before serializing
    let mut snapshot_copy = snapshot.clone();
    snapshot_copy.timestamp = chrono::Utc::now().to_rfc3339();

    match rmp_serde::to_vec_named(&snapshot_copy) {
        Ok(bytes) => {
            if bytes.len() > output_len as usize {
                return 0; // Buffer too small
            }
            let out_slice = std::slice::from_raw_parts_mut(output, bytes.len());
            out_slice.copy_from_slice(&bytes);
            bytes.len() as i32
        }
        Err(_) => 0,
    }
}

/// Free a PositionSnapshotMessage builder handle
///
/// # Safety
/// - handle must be a valid pointer created by create_position_snapshot_builder or null
/// - handle must not be used after calling this function
#[no_mangle]
pub unsafe extern "C" fn position_snapshot_builder_free(handle: *mut PositionSnapshotMessage) {
    free_handle(handle)
}

// ===========================================================================
// VLogsConfigMessage FFI Functions
// ===========================================================================

/// Parse MessagePack data as VLogsConfigMessage and return an opaque handle
///
/// # Safety
/// This function is unsafe because it dereferences raw pointers.
/// The returned handle must be freed with `vlogs_config_free()`.
#[no_mangle]
pub unsafe extern "C" fn parse_vlogs_config(
    data: *const u8,
    data_len: i32,
) -> *mut VLogsConfigMessage {
    parse_msgpack(data, data_len)
}

/// Free a VLogsConfigMessage handle
///
/// # Safety
/// - handle must be a valid pointer created by parse_vlogs_config or null
/// - handle must not be used after calling this function
#[no_mangle]
pub unsafe extern "C" fn vlogs_config_free(handle: *mut VLogsConfigMessage) {
    free_handle(handle)
}

/// Get a string field from VLogsConfigMessage handle
///
/// # Safety
/// - handle must be a valid pointer to VLogsConfigMessage
/// - field_name must be a valid null-terminated UTF-16 string pointer
#[no_mangle]
pub unsafe extern "C" fn vlogs_config_get_string(
    handle: *const VLogsConfigMessage,
    field_name: *const u16,
) -> *const u16 {
    if handle.is_null() || field_name.is_null() {
        return std::ptr::null();
    }

    let config = &*handle;
    let field = match utf16_to_string(field_name) {
        Some(s) => s,
        None => return std::ptr::null(),
    };

    let value = match field.as_str() {
        "endpoint" => &config.endpoint,
        "timestamp" => &config.timestamp,
        _ => return std::ptr::null(),
    };

    string_to_utf16_buffer(value)
}

/// Get a boolean field from VLogsConfigMessage handle (returns 1 for true, 0 for false)
///
/// # Safety
/// - handle must be a valid pointer to VLogsConfigMessage
/// - field_name must be a valid null-terminated UTF-16 string pointer
#[no_mangle]
pub unsafe extern "C" fn vlogs_config_get_bool(
    handle: *const VLogsConfigMessage,
    field_name: *const u16,
) -> i32 {
    if handle.is_null() || field_name.is_null() {
        return 0;
    }

    let config = &*handle;
    let field = match utf16_to_string(field_name) {
        Some(s) => s,
        None => return 0,
    };

    let result = match field.as_str() {
        "enabled" => config.enabled,
        _ => false,
    };

    if result {
        1
    } else {
        0
    }
}

/// Get an integer field from VLogsConfigMessage handle
///
/// # Safety
/// - handle must be a valid pointer to VLogsConfigMessage
/// - field_name must be a valid null-terminated UTF-16 string pointer
#[no_mangle]
pub unsafe extern "C" fn vlogs_config_get_int(
    handle: *const VLogsConfigMessage,
    field_name: *const u16,
) -> i32 {
    if handle.is_null() || field_name.is_null() {
        return 0;
    }

    let config = &*handle;
    let field = match utf16_to_string(field_name) {
        Some(s) => s,
        None => return 0,
    };

    match field.as_str() {
        "batch_size" => config.batch_size,
        "flush_interval_secs" => config.flush_interval_secs,
        _ => 0,
    }
}

// ===========================================================================
// ZeroMQ FFI Functions
// ===========================================================================

// ZeroMQ socket types
pub const ZMQ_PUSH: i32 = 8;
pub const ZMQ_PULL: i32 = 7;
pub const ZMQ_PUB: i32 = 1;
pub const ZMQ_SUB: i32 = 2;

// Global storage for contexts and sockets using handles
static CONTEXTS: LazyLock<Mutex<Vec<Option<Box<zmq::Context>>>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));
static SOCKETS: LazyLock<Mutex<Vec<Option<Box<zmq::Socket>>>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

/// Create a new ZeroMQ context
/// Returns an integer handle to the context, or -1 on error
#[no_mangle]
pub extern "C" fn zmq_context_create() -> i32 {
    let ctx = zmq::Context::new();
    let boxed_ctx = Box::new(ctx);

    let mut contexts = match CONTEXTS.lock() {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("zmq_context_create: failed to lock contexts: {}", e);
            return -1;
        }
    };

    // Try to find an empty slot
    for (i, slot) in contexts.iter_mut().enumerate() {
        if slot.is_none() {
            *slot = Some(boxed_ctx);
            return i as i32;
        }
    }

    // No empty slot, push new one
    contexts.push(Some(boxed_ctx));
    (contexts.len() - 1) as i32
}

/// Destroy a ZeroMQ context
///
/// # Parameters
/// - handle: Integer handle to the context
#[no_mangle]
pub extern "C" fn zmq_context_destroy(handle: i32) {
    if handle < 0 {
        eprintln!("zmq_context_destroy: invalid handle {}", handle);
        return;
    }

    let mut contexts = match CONTEXTS.lock() {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("zmq_context_destroy: failed to lock contexts: {}", e);
            return;
        }
    };

    let idx = handle as usize;
    if idx < contexts.len() {
        contexts[idx] = None;
    }
}

/// Create a new ZeroMQ socket
///
/// # Parameters
/// - context_handle: Integer handle to a valid ZeroMQ context
/// - socket_type: Socket type (ZMQ_PUSH=8, ZMQ_PULL=7, etc.)
///
/// Returns an integer handle to the socket, or -1 on error
#[no_mangle]
pub extern "C" fn zmq_socket_create(context_handle: i32, socket_type: i32) -> i32 {
    if context_handle < 0 {
        eprintln!(
            "zmq_socket_create: invalid context handle {}",
            context_handle
        );
        return -1;
    }

    let contexts = match CONTEXTS.lock() {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("zmq_socket_create: failed to lock contexts: {}", e);
            return -1;
        }
    };

    let ctx_idx = context_handle as usize;
    if ctx_idx >= contexts.len() {
        eprintln!("zmq_socket_create: context handle out of range");
        return -1;
    }

    let ctx = match &contexts[ctx_idx] {
        Some(c) => c.as_ref(),
        None => {
            eprintln!("zmq_socket_create: context handle points to destroyed context");
            return -1;
        }
    };

    let sock_type = match socket_type {
        ZMQ_PUSH => zmq::PUSH,
        ZMQ_PULL => zmq::PULL,
        ZMQ_PUB => zmq::PUB,
        ZMQ_SUB => zmq::SUB,
        _ => {
            eprintln!("zmq_socket_create: unknown socket type {}", socket_type);
            return -1;
        }
    };

    let socket = match ctx.socket(sock_type) {
        Ok(s) => {
            // Set LINGER to 1000ms to allow messages to be sent before socket close
            // LINGER=0 caused message loss with create-connect-send-destroy pattern
            // 1000ms is enough for local connections while avoiding long blocks on exit
            if let Err(e) = s.set_linger(1000) {
                eprintln!("zmq_socket_create: failed to set linger: {}", e);
            }
            Box::new(s)
        }
        Err(e) => {
            eprintln!("zmq_socket_create: failed to create socket: {}", e);
            return -1;
        }
    };

    drop(contexts);

    let mut sockets = match SOCKETS.lock() {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("zmq_socket_create: failed to lock sockets: {}", e);
            return -1;
        }
    };

    // Try to find an empty slot
    for (i, slot) in sockets.iter_mut().enumerate() {
        if slot.is_none() {
            *slot = Some(socket);
            return i as i32;
        }
    }

    // No empty slot, push new one
    sockets.push(Some(socket));
    (sockets.len() - 1) as i32
}

/// Destroy a ZeroMQ socket
///
/// # Parameters
/// - handle: Integer handle to the socket
#[no_mangle]
pub extern "C" fn zmq_socket_destroy(handle: i32) {
    if handle < 0 {
        eprintln!("zmq_socket_destroy: invalid handle {}", handle);
        return;
    }

    let mut sockets = match SOCKETS.lock() {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("zmq_socket_destroy: failed to lock sockets: {}", e);
            return;
        }
    };

    let idx = handle as usize;
    if idx < sockets.len() {
        sockets[idx] = None;
    }
}

/// Connect a ZeroMQ socket to an endpoint
///
/// # Parameters
/// - socket_handle: Integer handle to a valid ZeroMQ socket
/// - address: Null-terminated UTF-16 string containing the endpoint (e.g., "tcp://localhost:5555")
///
/// Returns 1 on success, 0 on failure
///
/// # Safety
/// Address pointer must be valid
#[no_mangle]
pub unsafe extern "C" fn zmq_socket_connect(socket_handle: i32, address: *const u16) -> i32 {
    if socket_handle < 0 {
        eprintln!("zmq_socket_connect: invalid socket handle");
        return 0;
    }

    if address.is_null() {
        eprintln!("zmq_socket_connect: null address pointer");
        return 0;
    }

    // Convert UTF-16 string from MQL5 to Rust String
    let mut len = 0;
    while *address.add(len) != 0 {
        len += 1;
    }
    let slice = std::slice::from_raw_parts(address, len);
    let addr = match String::from_utf16(slice) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("zmq_socket_connect: invalid UTF-16: {}", e);
            return 0;
        }
    };

    let sockets = match SOCKETS.lock() {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("zmq_socket_connect: failed to lock sockets: {}", e);
            return 0;
        }
    };

    let sock_idx = socket_handle as usize;
    if sock_idx >= sockets.len() {
        eprintln!("zmq_socket_connect: socket handle out of range");
        return 0;
    }

    let sock = match &sockets[sock_idx] {
        Some(s) => s.as_ref(),
        None => {
            eprintln!("zmq_socket_connect: socket handle points to destroyed socket");
            return 0;
        }
    };

    match sock.connect(&addr) {
        Ok(_) => 1,
        Err(e) => {
            eprintln!("zmq_socket_connect failed: {}", e);
            0
        }
    }
}

/// Bind a ZeroMQ socket to an endpoint
///
/// # Parameters
/// - socket_handle: Integer handle to a valid ZeroMQ socket
/// - address: Null-terminated UTF-16 string containing the endpoint (e.g., "tcp://*:5555")
///
/// Returns 1 on success, 0 on failure
///
/// # Safety
/// Address pointer must be valid
#[no_mangle]
pub unsafe extern "C" fn zmq_socket_bind(socket_handle: i32, address: *const u16) -> i32 {
    if socket_handle < 0 {
        eprintln!("zmq_socket_bind: invalid socket handle");
        return 0;
    }

    if address.is_null() {
        eprintln!("zmq_socket_bind: null address pointer");
        return 0;
    }

    // Convert UTF-16 string from MQL5 to Rust String
    let mut len = 0;
    while *address.add(len) != 0 {
        len += 1;
    }
    let slice = std::slice::from_raw_parts(address, len);
    let addr = match String::from_utf16(slice) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("zmq_socket_bind: invalid UTF-16: {}", e);
            return 0;
        }
    };

    let sockets = match SOCKETS.lock() {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("zmq_socket_bind: failed to lock sockets: {}", e);
            return 0;
        }
    };

    let sock_idx = socket_handle as usize;
    if sock_idx >= sockets.len() {
        eprintln!("zmq_socket_bind: socket handle out of range");
        return 0;
    }

    let sock = match &sockets[sock_idx] {
        Some(s) => s.as_ref(),
        None => {
            eprintln!("zmq_socket_bind: socket handle points to destroyed socket");
            return 0;
        }
    };

    match sock.bind(&addr) {
        Ok(_) => 1,
        Err(e) => {
            eprintln!("zmq_socket_bind failed: {}", e);
            0
        }
    }
}

/// Send a message through a ZeroMQ socket
///
/// # Parameters
/// - socket_handle: Integer handle to a valid ZeroMQ socket
/// - message: Null-terminated UTF-16 string containing the message to send
///
/// Returns 1 on success, 0 on failure
///
/// # Safety
/// Message pointer must be valid
#[no_mangle]
pub unsafe extern "C" fn zmq_socket_send(socket_handle: i32, message: *const u16) -> i32 {
    if socket_handle < 0 {
        eprintln!("zmq_socket_send: invalid socket handle");
        return 0;
    }

    if message.is_null() {
        eprintln!("zmq_socket_send: null message pointer");
        return 0;
    }

    // Convert UTF-16 string from MQL5 to Rust String
    let mut len = 0;
    while *message.add(len) != 0 {
        len += 1;
    }
    let slice = std::slice::from_raw_parts(message, len);
    let msg = match String::from_utf16(slice) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("zmq_socket_send: invalid UTF-16: {}", e);
            return 0;
        }
    };

    let sockets = match SOCKETS.lock() {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("zmq_socket_send: failed to lock sockets: {}", e);
            return 0;
        }
    };

    let sock_idx = socket_handle as usize;
    if sock_idx >= sockets.len() {
        eprintln!("zmq_socket_send: socket handle out of range");
        return 0;
    }

    let sock = match &sockets[sock_idx] {
        Some(s) => s.as_ref(),
        None => {
            eprintln!("zmq_socket_send: socket handle points to destroyed socket");
            return 0;
        }
    };

    match sock.send(&msg, 0) {
        Ok(_) => 1,
        Err(e) => {
            eprintln!("zmq_socket_send failed: {}", e);
            0
        }
    }
}

/// Send binary data through a ZeroMQ socket (for MessagePack)
///
/// # Parameters
/// - socket_handle: Integer handle to a valid ZeroMQ socket
/// - data: Pointer to the binary data to send
/// - len: Length of the data in bytes
///
/// Returns 1 on success, 0 on failure
///
/// # Safety
/// Data pointer must be valid and have at least len bytes
#[no_mangle]
pub unsafe extern "C" fn zmq_socket_send_binary(
    socket_handle: i32,
    data: *const u8,
    len: i32,
) -> i32 {
    if socket_handle < 0 {
        eprintln!("zmq_socket_send_binary: invalid socket handle");
        return 0;
    }

    if data.is_null() || len <= 0 {
        eprintln!("zmq_socket_send_binary: invalid data parameters");
        return 0;
    }

    // Convert pointer to slice
    let data_slice = std::slice::from_raw_parts(data, len as usize);

    let sockets = match SOCKETS.lock() {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("zmq_socket_send_binary: failed to lock sockets: {}", e);
            return 0;
        }
    };

    let sock_idx = socket_handle as usize;
    if sock_idx >= sockets.len() {
        eprintln!("zmq_socket_send_binary: socket handle out of range");
        return 0;
    }

    let sock = match &sockets[sock_idx] {
        Some(s) => s.as_ref(),
        None => {
            eprintln!("zmq_socket_send_binary: socket handle points to destroyed socket");
            return 0;
        }
    };

    match sock.send(data_slice, 0) {
        Ok(_) => 1,
        Err(e) => {
            eprintln!("zmq_socket_send_binary failed: {}", e);
            0
        }
    }
}

/// Receive a message from a ZeroMQ socket (non-blocking)
///
/// # Parameters
/// - socket_handle: Integer handle to a valid ZeroMQ socket
/// - buffer: Buffer to write the received message into
/// - buffer_size: Size of the buffer in bytes
///
/// Returns the number of bytes received, or -1 on error, or 0 if no message available (EAGAIN)
///
/// # Safety
/// Buffer pointer must be valid and have at least buffer_size bytes
#[no_mangle]
pub unsafe extern "C" fn zmq_socket_receive(
    socket_handle: i32,
    buffer: *mut c_char,
    buffer_size: i32,
) -> i32 {
    if socket_handle < 0 {
        eprintln!("zmq_socket_receive: invalid socket handle");
        return -1;
    }

    if buffer.is_null() || buffer_size <= 0 {
        eprintln!("zmq_socket_receive: invalid buffer parameters");
        return -1;
    }

    let sockets = match SOCKETS.lock() {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("zmq_socket_receive: failed to lock sockets: {}", e);
            return -1;
        }
    };

    let sock_idx = socket_handle as usize;
    if sock_idx >= sockets.len() {
        eprintln!("zmq_socket_receive: socket handle out of range");
        return -1;
    }

    let sock = match &sockets[sock_idx] {
        Some(s) => s.as_ref(),
        None => {
            eprintln!("zmq_socket_receive: socket handle points to destroyed socket");
            return -1;
        }
    };

    // Try to receive with DONTWAIT flag (non-blocking)
    match sock.recv_bytes(zmq::DONTWAIT) {
        Ok(bytes) => {
            let len = bytes.len().min((buffer_size - 1) as usize);
            ptr::copy_nonoverlapping(bytes.as_ptr() as *const c_char, buffer, len);
            // Null-terminate the string
            *buffer.add(len) = 0;
            len as i32
        }
        Err(zmq::Error::EAGAIN) => {
            // No message available (non-blocking)
            0
        }
        Err(e) => {
            eprintln!("zmq_socket_receive failed: {}", e);
            -1
        }
    }
}

/// Set socket option to subscribe to all messages (for SUB sockets)
///
/// # Parameters
/// - socket_handle: Integer handle to a valid ZeroMQ SUB socket
///
/// Returns 1 on success, 0 on failure
#[no_mangle]
pub extern "C" fn zmq_socket_subscribe_all(socket_handle: i32) -> i32 {
    if socket_handle < 0 {
        eprintln!("zmq_socket_subscribe_all: invalid socket handle");
        return 0;
    }

    let sockets = match SOCKETS.lock() {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("zmq_socket_subscribe_all: failed to lock sockets: {}", e);
            return 0;
        }
    };

    let sock_idx = socket_handle as usize;
    if sock_idx >= sockets.len() {
        eprintln!("zmq_socket_subscribe_all: socket handle out of range");
        return 0;
    }

    let sock = match &sockets[sock_idx] {
        Some(s) => s.as_ref(),
        None => {
            eprintln!("zmq_socket_subscribe_all: socket handle points to destroyed socket");
            return 0;
        }
    };

    match sock.set_subscribe(b"") {
        Ok(_) => 1,
        Err(e) => {
            eprintln!("zmq_socket_subscribe_all failed: {}", e);
            0
        }
    }
}

/// Set socket option to subscribe to a specific topic (for SUB sockets)
///
/// # Parameters
/// - socket_handle: Integer handle to a valid ZeroMQ SUB socket
/// - topic: Null-terminated UTF-16 string containing the topic to subscribe to
///
/// Returns 1 on success, 0 on failure
///
/// # Safety
/// Topic pointer must be valid
#[no_mangle]
pub unsafe extern "C" fn zmq_socket_subscribe(socket_handle: i32, topic: *const u16) -> i32 {
    if socket_handle < 0 {
        eprintln!("zmq_socket_subscribe: invalid socket handle");
        return 0;
    }

    if topic.is_null() {
        eprintln!("zmq_socket_subscribe: null topic pointer");
        return 0;
    }

    // Convert UTF-16 string from MQL5 to Rust String
    let mut len = 0;
    while *topic.add(len) != 0 {
        len += 1;
    }
    let slice = std::slice::from_raw_parts(topic, len);
    let topic_str = match String::from_utf16(slice) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("zmq_socket_subscribe: invalid UTF-16: {}", e);
            return 0;
        }
    };

    let sockets = match SOCKETS.lock() {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("zmq_socket_subscribe: failed to lock sockets: {}", e);
            return 0;
        }
    };

    let sock_idx = socket_handle as usize;
    if sock_idx >= sockets.len() {
        eprintln!("zmq_socket_subscribe: socket handle out of range");
        return 0;
    }

    let sock = match &sockets[sock_idx] {
        Some(s) => s.as_ref(),
        None => {
            eprintln!("zmq_socket_subscribe: socket handle points to destroyed socket");
            return 0;
        }
    };

    match sock.set_subscribe(topic_str.as_bytes()) {
        Ok(_) => {
            eprintln!("zmq_socket_subscribe: subscribed to topic '{}'", topic_str);
            1
        }
        Err(e) => {
            eprintln!("zmq_socket_subscribe failed: {}", e);
            0
        }
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
    if context.is_null() {
        return 0;
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

    crate::ffi_helpers::serialize_to_buffer(&msg, output, output_len)
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
    if context.is_null() {
        return 0;
    }
    let ctx = &*context;

    let msg = crate::types::HeartbeatMessage {
        message_type: "Heartbeat".to_string(),
        account_id: ctx.account_id.clone(),
        balance,
        equity,
        open_positions,
        timestamp: chrono::Utc::now().to_rfc3339(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        ea_type: ctx.ea_type.clone(),
        platform: ctx.platform.clone(),
        account_number: ctx.account_number,
        broker: ctx.broker.clone(),
        account_name: ctx.account_name.clone(),
        server: ctx.server.clone(),
        currency: ctx.currency.clone(),
        leverage: ctx.leverage,
        is_trade_allowed: is_trade_allowed != 0,
        // Optional fields (defaults)
        symbol_prefix: None,
        symbol_suffix: None,
        symbol_map: None,
    };

    crate::ffi_helpers::serialize_to_buffer(&msg, output, output_len)
}

/// Create and serialize an UnregisterMessage using context data
#[no_mangle]
pub unsafe extern "C" fn ea_send_unregister(
    context: *mut crate::EaContext,
    output: *mut u8,
    output_len: i32,
) -> i32 {
    if context.is_null() {
        return 0;
    }
    let ctx = &*context;

    let msg = crate::types::UnregisterMessage {
        message_type: "Unregister".to_string(),
        account_id: ctx.account_id.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        ea_type: Some(ctx.ea_type.clone()),
    };

    crate::ffi_helpers::serialize_to_buffer(&msg, output, output_len)
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
    if !context.is_null() {
        drop(Box::from_raw(context));
    }
}

/// Determine if RequestConfig should be sent based on current state
///
/// # Parameters
/// - `context`: Pointer to EA Context (must not be null)
/// - `current_trade_allowed`: 1 if auto-trading is currently enabled, 0 otherwise
///
/// # Returns
/// - 1 if RequestConfig should be sent
/// - 0 if RequestConfig should not be sent (already requested)
///
/// # Safety
/// - `context` must be a valid pointer returned by `ea_init()`
/// - `context` must not have been freed
#[no_mangle]
pub unsafe extern "C" fn ea_context_should_request_config(
    context: *mut crate::EaContext,
    current_trade_allowed: i32,
) -> i32 {
    if context.is_null() {
        return 0;
    }

    let ea_ctx = &mut *context;
    let trade_allowed = current_trade_allowed != 0;

    if ea_ctx.should_request_config(trade_allowed) {
        1
    } else {
        0
    }
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
    if !context.is_null() {
        (*context).mark_config_requested();
    }
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
    if !context.is_null() {
        (*context).reset();
    }
}
