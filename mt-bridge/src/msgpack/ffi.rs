// Location: mt-bridge/src/msgpack/ffi.rs
// Purpose: FFI functions for MQL4/MQL5 integration
// Why: Provides C-compatible interface for parsing and accessing MessagePack messages from EA

use super::helpers::{
    string_to_utf16_buffer, utf16_to_string, BUFFER_INDEX, MAX_STRING_LEN, STRING_BUFFER_1,
    STRING_BUFFER_2, STRING_BUFFER_3, STRING_BUFFER_4,
};
use super::types::{MasterConfigMessage, SlaveConfigMessage, TradeSignalMessage};
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

    // Use a static empty string to avoid temporary value dropped error
    static EMPTY_STRING: LazyLock<String> = LazyLock::new(String::new);

    let value = match field.as_str() {
        "account_id" => &config.account_id,
        "master_account" => &config.master_account,
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

    if field == "lot_multiplier" {
        config.lot_multiplier.unwrap_or(1.0)
    } else {
        0.0
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
