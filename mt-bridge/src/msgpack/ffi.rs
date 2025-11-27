// Location: mt-bridge/src/msgpack/ffi.rs
// Purpose: FFI functions for MQL4/MQL5 integration
// Why: Provides C-compatible interface for parsing and accessing MessagePack messages from EA

use super::helpers::{
    string_to_utf16_buffer, utf16_to_string, BUFFER_INDEX, MAX_STRING_LEN, STRING_BUFFER_1,
    STRING_BUFFER_2, STRING_BUFFER_3, STRING_BUFFER_4,
};
use super::types::{
    MasterConfigMessage, PositionInfo, PositionSnapshotMessage, SlaveConfigMessage, SyncMode,
    SyncRequestMessage, TradeSignalMessage,
};
use std::ffi::CString;
use std::os::raw::c_char;
use std::sync::LazyLock;

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
    if data.is_null() || data_len <= 0 {
        return std::ptr::null_mut();
    }

    let slice = std::slice::from_raw_parts(data, data_len as usize);
    match rmp_serde::from_slice::<SlaveConfigMessage>(slice) {
        Ok(config) => Box::into_raw(Box::new(config)),
        Err(_) => std::ptr::null_mut(),
    }
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
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
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
    if data.is_null() || data_len <= 0 {
        return std::ptr::null_mut();
    }

    let slice = std::slice::from_raw_parts(data, data_len as usize);
    match rmp_serde::from_slice::<MasterConfigMessage>(slice) {
        Ok(config) => Box::into_raw(Box::new(config)),
        Err(_) => std::ptr::null_mut(),
    }
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
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
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

    let value = match field.as_str() {
        "account_id" => &config.account_id,
        "master_account" => &config.master_account,
        "timestamp" => &config.timestamp,
        "symbol_prefix" => config.symbol_prefix.as_ref().unwrap_or(&EMPTY_STRING),
        "symbol_suffix" => config.symbol_suffix.as_ref().unwrap_or(&EMPTY_STRING),
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
    if data.is_null() || data_len <= 0 {
        return std::ptr::null_mut();
    }

    let slice = std::slice::from_raw_parts(data, data_len as usize);
    match rmp_serde::from_slice::<TradeSignalMessage>(slice) {
        Ok(msg) => Box::into_raw(Box::new(msg)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Free a TradeSignalMessage handle
///
/// # Safety
/// - handle must be a valid pointer created by parse_trade_signal or null
/// - handle must not be used after calling this function
#[no_mangle]
pub unsafe extern "C" fn trade_signal_free(handle: *mut TradeSignalMessage) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
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
    if data.is_null() || data_len <= 0 {
        return std::ptr::null_mut();
    }

    let slice = std::slice::from_raw_parts(data, data_len as usize);
    match rmp_serde::from_slice::<PositionSnapshotMessage>(slice) {
        Ok(snapshot) => Box::into_raw(Box::new(snapshot)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Free a PositionSnapshotMessage handle
///
/// # Safety
/// - handle must be a valid pointer created by parse_position_snapshot or null
/// - handle must not be used after calling this function
#[no_mangle]
pub unsafe extern "C" fn position_snapshot_free(handle: *mut PositionSnapshotMessage) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
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
pub unsafe extern "C" fn parse_sync_request(data: *const u8, data_len: i32) -> *mut SyncRequestMessage {
    if data.is_null() || data_len <= 0 {
        return std::ptr::null_mut();
    }

    let slice = std::slice::from_raw_parts(data, data_len as usize);

    match rmp_serde::from_slice::<SyncRequestMessage>(slice) {
        Ok(msg) => Box::into_raw(Box::new(msg)),
        Err(_) => std::ptr::null_mut(),
    }
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
    if !handle.is_null() {
        let _ = Box::from_raw(handle);
    }
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
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}
