use serde::{Deserialize, Serialize};
use std::ffi::CString;
use std::os::raw::c_char;
use std::sync::Mutex;
use once_cell::sync::Lazy;

// Static buffers for returning UTF-16 strings to MQL5 (supports up to 4 concurrent strings)
const MAX_STRING_LEN: usize = 512;
static STRING_BUFFER_1: Lazy<Mutex<Vec<u16>>> = Lazy::new(|| Mutex::new(vec![0; MAX_STRING_LEN]));
static STRING_BUFFER_2: Lazy<Mutex<Vec<u16>>> = Lazy::new(|| Mutex::new(vec![0; MAX_STRING_LEN]));
static STRING_BUFFER_3: Lazy<Mutex<Vec<u16>>> = Lazy::new(|| Mutex::new(vec![0; MAX_STRING_LEN]));
static STRING_BUFFER_4: Lazy<Mutex<Vec<u16>>> = Lazy::new(|| Mutex::new(vec![0; MAX_STRING_LEN]));
static BUFFER_INDEX: Lazy<Mutex<usize>> = Lazy::new(|| Mutex::new(0));

/// Symbol mapping structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolMapping {
    pub source_symbol: String,
    pub target_symbol: String,
}

/// Trade filters structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeFilters {
    pub allowed_symbols: Option<Vec<String>>,
    pub blocked_symbols: Option<Vec<String>>,
    pub allowed_magic_numbers: Option<Vec<i32>>,
    pub blocked_magic_numbers: Option<Vec<i32>>,
}

/// Configuration message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigMessage {
    pub account_id: String,
    pub master_account: String,
    pub trade_group_id: String,
    pub timestamp: String,  // ISO 8601 format
    pub enabled: bool,
    pub lot_multiplier: Option<f64>,
    pub reverse_trade: bool,
    pub symbol_mappings: Vec<SymbolMapping>,
    pub filters: TradeFilters,
    pub config_version: u32,
}

/// Parse MessagePack data and return an opaque handle to ConfigMessage
///
/// # Safety
/// This function is unsafe because it dereferences raw pointers.
/// The returned handle must be freed with `config_free()`.
#[no_mangle]
pub unsafe extern "C" fn msgpack_parse(
    data: *const u8,
    data_len: i32,
) -> *mut ConfigMessage {
    if data.is_null() || data_len <= 0 {
        return std::ptr::null_mut();
    }

    let slice = std::slice::from_raw_parts(data, data_len as usize);
    match rmp_serde::from_slice::<ConfigMessage>(slice) {
        Ok(config) => Box::into_raw(Box::new(config)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Free a ConfigMessage handle
///
/// # Safety
/// This function is unsafe because it takes ownership of a raw pointer.
/// The caller must ensure:
/// - `handle` was returned by `msgpack_parse()`
/// - `handle` is only freed once
#[no_mangle]
pub unsafe extern "C" fn config_free(handle: *mut ConfigMessage) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

/// Get a string field from ConfigMessage handle
///
/// # Safety
/// This function is unsafe because it dereferences raw pointers.
/// Returns a pointer to a static UTF-16 buffer (valid until next 4 calls).
#[no_mangle]
pub unsafe extern "C" fn config_get_string(
    handle: *const ConfigMessage,
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

    let value = match field.as_str() {
        "account_id" => &config.account_id,
        "master_account" => &config.master_account,
        "trade_group_id" => &config.trade_group_id,
        "timestamp" => &config.timestamp,
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

/// Get a double field from ConfigMessage handle
#[no_mangle]
pub unsafe extern "C" fn config_get_double(
    handle: *const ConfigMessage,
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

/// Get a boolean field from ConfigMessage handle
#[no_mangle]
pub unsafe extern "C" fn config_get_bool(
    handle: *const ConfigMessage,
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
        "enabled" => config.enabled,
        "reverse_trade" => config.reverse_trade,
        _ => false,
    };

    if result { 1 } else { 0 }
}

/// Get an integer field from ConfigMessage handle
#[no_mangle]
pub unsafe extern "C" fn config_get_int(
    handle: *const ConfigMessage,
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

    if field == "config_version" {
        config.config_version as i32
    } else {
        0
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
pub unsafe extern "C" fn msgpack_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(CString::from_raw(ptr));
    }
}
