use serde::{Deserialize, Serialize};
use std::ffi::CString;
use std::os::raw::c_char;
use std::sync::{LazyLock, Mutex};

// Static buffers for returning UTF-16 strings to MQL5 (supports up to 4 concurrent strings)
const MAX_STRING_LEN: usize = 512;
static STRING_BUFFER_1: LazyLock<Mutex<Vec<u16>>> = LazyLock::new(|| Mutex::new(vec![0; MAX_STRING_LEN]));
static STRING_BUFFER_2: LazyLock<Mutex<Vec<u16>>> = LazyLock::new(|| Mutex::new(vec![0; MAX_STRING_LEN]));
static STRING_BUFFER_3: LazyLock<Mutex<Vec<u16>>> = LazyLock::new(|| Mutex::new(vec![0; MAX_STRING_LEN]));
static STRING_BUFFER_4: LazyLock<Mutex<Vec<u16>>> = LazyLock::new(|| Mutex::new(vec![0; MAX_STRING_LEN]));
static BUFFER_INDEX: LazyLock<Mutex<usize>> = LazyLock::new(|| Mutex::new(0));

/// Symbol mapping structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolMapping {
    pub source_symbol: String,
    pub target_symbol: String,
}

/// Trade filters structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeFilters {
    #[serde(default)]
    pub allowed_symbols: Option<Vec<String>>,
    #[serde(default)]
    pub blocked_symbols: Option<Vec<String>>,
    #[serde(default)]
    pub allowed_magic_numbers: Option<Vec<i32>>,
    #[serde(default)]
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
    #[serde(default)]
    pub lot_multiplier: Option<f64>,
    pub reverse_trade: bool,
    pub symbol_mappings: Vec<SymbolMapping>,
    pub filters: TradeFilters,
    pub config_version: u32,
}

/// Registration message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterMessage {
    pub message_type: String,  // "Register"
    pub account_id: String,
    pub ea_type: String,  // "Master" or "Slave"
    pub platform: String,  // "MT4" or "MT5"
    pub account_number: i64,
    pub broker: String,
    pub account_name: String,
    pub server: String,
    pub balance: f64,
    pub equity: f64,
    pub currency: String,
    pub leverage: i64,
    pub timestamp: String,
}

/// Unregistration message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnregisterMessage {
    pub message_type: String,  // "Unregister"
    pub account_id: String,
    pub timestamp: String,
}

/// Heartbeat message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatMessage {
    pub message_type: String,  // "Heartbeat"
    pub account_id: String,
    pub balance: f64,
    pub equity: f64,
    pub open_positions: i32,
    pub timestamp: String,
}

/// Trade signal message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeSignalMessage {
    pub action: String,  // "Open", "Close", "Modify"
    pub ticket: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lots: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open_price: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_loss: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub take_profit: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub magic_number: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    pub timestamp: String,
    pub source_account: String,
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

//==============================================================================
// Generic MessagePack Serialization/Deserialization for Trade Messages
//==============================================================================

// Static buffer for serialized data
static SERIALIZE_BUFFER: LazyLock<Mutex<Vec<u8>>> = LazyLock::new(|| Mutex::new(Vec::with_capacity(8192)));

/// Serialize a RegisterMessage to MessagePack
///
/// Returns the length of serialized data (or 0 on error).
/// The serialized data is stored in an internal buffer accessible via msgpack_get_buffer().
#[no_mangle]
pub unsafe extern "C" fn msgpack_serialize_register(
    message_type: *const u16,
    account_id: *const u16,
    ea_type: *const u16,
    platform: *const u16,
    account_number: i64,
    broker: *const u16,
    account_name: *const u16,
    server: *const u16,
    balance: f64,
    equity: f64,
    currency: *const u16,
    leverage: i64,
    timestamp: *const u16,
) -> i32 {
    let msg = RegisterMessage {
        message_type: utf16_to_string(message_type).unwrap_or_default(),
        account_id: utf16_to_string(account_id).unwrap_or_default(),
        ea_type: utf16_to_string(ea_type).unwrap_or_default(),
        platform: utf16_to_string(platform).unwrap_or_default(),
        account_number,
        broker: utf16_to_string(broker).unwrap_or_default(),
        account_name: utf16_to_string(account_name).unwrap_or_default(),
        server: utf16_to_string(server).unwrap_or_default(),
        balance,
        equity,
        currency: utf16_to_string(currency).unwrap_or_default(),
        leverage,
        timestamp: utf16_to_string(timestamp).unwrap_or_default(),
    };

    match rmp_serde::to_vec_named(&msg) {
        Ok(data) => {
            let mut buffer = SERIALIZE_BUFFER.lock().unwrap();
            *buffer = data;
            buffer.len() as i32
        }
        Err(_) => 0,
    }
}

/// Serialize an UnregisterMessage to MessagePack
#[no_mangle]
pub unsafe extern "C" fn msgpack_serialize_unregister(
    message_type: *const u16,
    account_id: *const u16,
    timestamp: *const u16,
) -> i32 {
    let msg = UnregisterMessage {
        message_type: utf16_to_string(message_type).unwrap_or_default(),
        account_id: utf16_to_string(account_id).unwrap_or_default(),
        timestamp: utf16_to_string(timestamp).unwrap_or_default(),
    };

    match rmp_serde::to_vec_named(&msg) {
        Ok(data) => {
            let mut buffer = SERIALIZE_BUFFER.lock().unwrap();
            *buffer = data;
            buffer.len() as i32
        }
        Err(_) => 0,
    }
}

/// Serialize a HeartbeatMessage to MessagePack
#[no_mangle]
pub unsafe extern "C" fn msgpack_serialize_heartbeat(
    message_type: *const u16,
    account_id: *const u16,
    balance: f64,
    equity: f64,
    open_positions: i32,
    timestamp: *const u16,
) -> i32 {
    let msg = HeartbeatMessage {
        message_type: utf16_to_string(message_type).unwrap_or_default(),
        account_id: utf16_to_string(account_id).unwrap_or_default(),
        balance,
        equity,
        open_positions,
        timestamp: utf16_to_string(timestamp).unwrap_or_default(),
    };

    match rmp_serde::to_vec_named(&msg) {
        Ok(data) => {
            let mut buffer = SERIALIZE_BUFFER.lock().unwrap();
            *buffer = data;
            buffer.len() as i32
        }
        Err(_) => 0,
    }
}

/// Serialize a TradeSignalMessage to MessagePack
#[no_mangle]
pub unsafe extern "C" fn msgpack_serialize_trade_signal(
    action: *const u16,
    ticket: i64,
    symbol: *const u16,
    order_type: *const u16,
    lots: f64,
    open_price: f64,
    stop_loss: f64,
    take_profit: f64,
    magic_number: i64,
    comment: *const u16,
    timestamp: *const u16,
    source_account: *const u16,
) -> i32 {
    let msg = TradeSignalMessage {
        action: utf16_to_string(action).unwrap_or_default(),
        ticket,
        symbol: utf16_to_string_opt(symbol),
        order_type: utf16_to_string_opt(order_type),
        lots: if lots > 0.0 { Some(lots) } else { None },
        open_price: if open_price > 0.0 { Some(open_price) } else { None },
        stop_loss: if stop_loss > 0.0 { Some(stop_loss) } else { None },
        take_profit: if take_profit > 0.0 { Some(take_profit) } else { None },
        magic_number: Some(magic_number),
        comment: utf16_to_string_opt(comment),
        timestamp: utf16_to_string(timestamp).unwrap_or_default(),
        source_account: utf16_to_string(source_account).unwrap_or_default(),
    };

    match rmp_serde::to_vec_named(&msg) {
        Ok(data) => {
            let mut buffer = SERIALIZE_BUFFER.lock().unwrap();
            *buffer = data;
            buffer.len() as i32
        }
        Err(_) => 0,
    }
}

/// Get pointer to the serialized MessagePack buffer
///
/// # Safety
/// The returned pointer is valid until the next serialization call.
/// The caller must copy the data before the next call.
#[no_mangle]
pub unsafe extern "C" fn msgpack_get_buffer() -> *const u8 {
    let buffer = SERIALIZE_BUFFER.lock().unwrap();
    buffer.as_ptr()
}

/// Copy the serialized MessagePack buffer to an MQL array
///
/// # Parameters
/// - dest: Destination buffer provided by MQL
/// - max_len: Maximum size of the destination buffer
///
/// Returns the actual number of bytes copied (or 0 if buffer is larger than max_len)
///
/// # Safety
/// Destination pointer must be valid and have at least max_len bytes available.
#[no_mangle]
pub unsafe extern "C" fn msgpack_copy_buffer(dest: *mut u8, max_len: i32) -> i32 {
    if dest.is_null() || max_len <= 0 {
        return 0;
    }

    let buffer = SERIALIZE_BUFFER.lock().unwrap();
    let len = buffer.len();

    if len > max_len as usize {
        eprintln!("msgpack_copy_buffer: buffer size {} exceeds max_len {}", len, max_len);
        return 0;
    }

    std::ptr::copy_nonoverlapping(buffer.as_ptr(), dest, len);
    len as i32
}

/// Parse a TradeSignalMessage from MessagePack data
#[no_mangle]
pub unsafe extern "C" fn msgpack_parse_trade_signal(
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
#[no_mangle]
pub unsafe extern "C" fn trade_signal_free(handle: *mut TradeSignalMessage) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

/// Get a string field from TradeSignalMessage handle
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

//==============================================================================
// Helper Functions
//==============================================================================

/// Convert UTF-16 string to Rust String
unsafe fn utf16_to_string(ptr: *const u16) -> Option<String> {
    if ptr.is_null() {
        return None;
    }

    let mut len = 0;
    while *ptr.add(len) != 0 {
        len += 1;
    }
    let slice = std::slice::from_raw_parts(ptr, len);
    String::from_utf16(slice).ok()
}

/// Convert UTF-16 string to Option<String> (empty becomes None)
unsafe fn utf16_to_string_opt(ptr: *const u16) -> Option<String> {
    let s = utf16_to_string(ptr)?;
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// Convert Rust String to UTF-16 buffer
unsafe fn string_to_utf16_buffer(s: &str) -> *const u16 {
    let mut index = BUFFER_INDEX.lock().unwrap();
    let current_index = *index;
    *index = (*index + 1) % 4;
    drop(index);

    let buffer_mutex = match current_index {
        0 => &STRING_BUFFER_1,
        1 => &STRING_BUFFER_2,
        2 => &STRING_BUFFER_3,
        _ => &STRING_BUFFER_4,
    };

    let mut buffer = buffer_mutex.lock().unwrap();
    let utf16: Vec<u16> = s.encode_utf16().chain(std::iter::once(0)).collect();
    let copy_len = utf16.len().min(MAX_STRING_LEN - 1);
    buffer[..copy_len].copy_from_slice(&utf16[..copy_len]);
    buffer[copy_len] = 0;

    buffer.as_ptr()
}

//==============================================================================
// Unit Tests
//==============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_message_serialization() {
        let msg = RegisterMessage {
            message_type: "Register".to_string(),
            account_id: "test_account_123".to_string(),
            ea_type: "Master".to_string(),
            platform: "MT5".to_string(),
            account_number: 12345,
            broker: "TestBroker".to_string(),
            account_name: "Test Account".to_string(),
            server: "TestServer-Live".to_string(),
            balance: 10000.50,
            equity: 10000.50,
            currency: "USD".to_string(),
            leverage: 100,
            timestamp: "2025-01-01T00:00:00Z".to_string(),
        };

        // Serialize
        let serialized = rmp_serde::to_vec_named(&msg).expect("Failed to serialize");
        assert!(serialized.len() > 0, "Serialized data should not be empty");

        // Deserialize
        let deserialized: RegisterMessage = rmp_serde::from_slice(&serialized)
            .expect("Failed to deserialize");

        // Verify fields
        assert_eq!(msg.message_type, deserialized.message_type);
        assert_eq!(msg.account_id, deserialized.account_id);
        assert_eq!(msg.ea_type, deserialized.ea_type);
        assert_eq!(msg.platform, deserialized.platform);
        assert_eq!(msg.account_number, deserialized.account_number);
        assert_eq!(msg.broker, deserialized.broker);
        assert_eq!(msg.account_name, deserialized.account_name);
        assert_eq!(msg.server, deserialized.server);
        assert_eq!(msg.balance, deserialized.balance);
        assert_eq!(msg.equity, deserialized.equity);
        assert_eq!(msg.currency, deserialized.currency);
        assert_eq!(msg.leverage, deserialized.leverage);
        assert_eq!(msg.timestamp, deserialized.timestamp);
    }

    #[test]
    fn test_unregister_message_serialization() {
        let msg = UnregisterMessage {
            message_type: "Unregister".to_string(),
            account_id: "test_account_123".to_string(),
            timestamp: "2025-01-01T00:00:00Z".to_string(),
        };

        let serialized = rmp_serde::to_vec_named(&msg).expect("Failed to serialize");
        let deserialized: UnregisterMessage = rmp_serde::from_slice(&serialized)
            .expect("Failed to deserialize");

        assert_eq!(msg.message_type, deserialized.message_type);
        assert_eq!(msg.account_id, deserialized.account_id);
        assert_eq!(msg.timestamp, deserialized.timestamp);
    }

    #[test]
    fn test_heartbeat_message_serialization() {
        let msg = HeartbeatMessage {
            message_type: "Heartbeat".to_string(),
            account_id: "test_account_123".to_string(),
            balance: 10500.75,
            equity: 10600.25,
            open_positions: 3,
            timestamp: "2025-01-01T00:00:00Z".to_string(),
        };

        let serialized = rmp_serde::to_vec_named(&msg).expect("Failed to serialize");
        let deserialized: HeartbeatMessage = rmp_serde::from_slice(&serialized)
            .expect("Failed to deserialize");

        assert_eq!(msg.message_type, deserialized.message_type);
        assert_eq!(msg.account_id, deserialized.account_id);
        assert_eq!(msg.balance, deserialized.balance);
        assert_eq!(msg.equity, deserialized.equity);
        assert_eq!(msg.open_positions, deserialized.open_positions);
        assert_eq!(msg.timestamp, deserialized.timestamp);
    }

    #[test]
    fn test_trade_signal_message_serialization() {
        let msg = TradeSignalMessage {
            action: "Open".to_string(),
            ticket: 123456,
            symbol: Some("EURUSD".to_string()),
            order_type: Some("Buy".to_string()),
            lots: Some(0.1),
            open_price: Some(1.0850),
            stop_loss: Some(1.0800),
            take_profit: Some(1.0900),
            magic_number: Some(0),
            comment: Some("Test trade".to_string()),
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            source_account: "master_account".to_string(),
        };

        let serialized = rmp_serde::to_vec_named(&msg).expect("Failed to serialize");
        let deserialized: TradeSignalMessage = rmp_serde::from_slice(&serialized)
            .expect("Failed to deserialize");

        assert_eq!(msg.action, deserialized.action);
        assert_eq!(msg.ticket, deserialized.ticket);
        assert_eq!(msg.symbol, deserialized.symbol);
        assert_eq!(msg.order_type, deserialized.order_type);
        assert_eq!(msg.lots, deserialized.lots);
        assert_eq!(msg.open_price, deserialized.open_price);
        assert_eq!(msg.stop_loss, deserialized.stop_loss);
        assert_eq!(msg.take_profit, deserialized.take_profit);
        assert_eq!(msg.magic_number, deserialized.magic_number);
        assert_eq!(msg.comment, deserialized.comment);
        assert_eq!(msg.timestamp, deserialized.timestamp);
        assert_eq!(msg.source_account, deserialized.source_account);
    }

    #[test]
    fn test_trade_signal_close_action() {
        // Close action should have minimal fields
        let msg = TradeSignalMessage {
            action: "Close".to_string(),
            ticket: 123456,
            symbol: None,
            order_type: None,
            lots: None,
            open_price: None,
            stop_loss: None,
            take_profit: None,
            magic_number: None,
            comment: None,
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            source_account: "master_account".to_string(),
        };

        let serialized = rmp_serde::to_vec_named(&msg).expect("Failed to serialize");
        let deserialized: TradeSignalMessage = rmp_serde::from_slice(&serialized)
            .expect("Failed to deserialize");

        assert_eq!(msg.action, deserialized.action);
        assert_eq!(msg.ticket, deserialized.ticket);
        assert!(deserialized.symbol.is_none());
        assert!(deserialized.order_type.is_none());
        assert!(deserialized.lots.is_none());
    }

    #[test]
    fn test_config_message_serialization() {
        let config = ConfigMessage {
            account_id: "slave_account_123".to_string(),
            master_account: "master_account_456".to_string(),
            trade_group_id: "group_789".to_string(),
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            enabled: true,
            lot_multiplier: Some(1.5),
            reverse_trade: false,
            symbol_mappings: vec![
                SymbolMapping {
                    source_symbol: "EURUSD".to_string(),
                    target_symbol: "EURUSD.raw".to_string(),
                },
            ],
            filters: TradeFilters {
                allowed_symbols: Some(vec!["EURUSD".to_string(), "GBPUSD".to_string()]),
                blocked_symbols: None,
                allowed_magic_numbers: Some(vec![0, 123]),
                blocked_magic_numbers: None,
            },
            config_version: 1,
        };

        let serialized = rmp_serde::to_vec_named(&config).expect("Failed to serialize");
        let deserialized: ConfigMessage = rmp_serde::from_slice(&serialized)
            .expect("Failed to deserialize");

        assert_eq!(config.account_id, deserialized.account_id);
        assert_eq!(config.master_account, deserialized.master_account);
        assert_eq!(config.enabled, deserialized.enabled);
        assert_eq!(config.lot_multiplier, deserialized.lot_multiplier);
        assert_eq!(config.reverse_trade, deserialized.reverse_trade);
        assert_eq!(config.symbol_mappings.len(), deserialized.symbol_mappings.len());
        assert_eq!(config.config_version, deserialized.config_version);
    }

    #[test]
    fn test_messagepack_size_optimization() {
        // Test that optional None fields are omitted in serialization
        let msg_full = TradeSignalMessage {
            action: "Open".to_string(),
            ticket: 123456,
            symbol: Some("EURUSD".to_string()),
            order_type: Some("Buy".to_string()),
            lots: Some(0.1),
            open_price: Some(1.0850),
            stop_loss: Some(1.0800),
            take_profit: Some(1.0900),
            magic_number: Some(0),
            comment: Some("Test".to_string()),
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            source_account: "master".to_string(),
        };

        let msg_minimal = TradeSignalMessage {
            action: "Close".to_string(),
            ticket: 123456,
            symbol: None,
            order_type: None,
            lots: None,
            open_price: None,
            stop_loss: None,
            take_profit: None,
            magic_number: None,
            comment: None,
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            source_account: "master".to_string(),
        };

        let serialized_full = rmp_serde::to_vec_named(&msg_full).unwrap();
        let serialized_minimal = rmp_serde::to_vec_named(&msg_minimal).unwrap();

        // Minimal message should be smaller
        assert!(serialized_minimal.len() < serialized_full.len(),
                "Minimal message ({} bytes) should be smaller than full message ({} bytes)",
                serialized_minimal.len(), serialized_full.len());
    }

    #[test]
    fn test_serialization_buffer_thread_safety() {
        use std::thread;

        // Test that multiple threads can serialize concurrently
        let handles: Vec<_> = (0..4)
            .map(|i| {
                thread::spawn(move || {
                    let msg = RegisterMessage {
                        message_type: "Register".to_string(),
                        account_id: format!("account_{}", i),
                        ea_type: "Master".to_string(),
                        platform: "MT5".to_string(),
                        account_number: i as i64,
                        broker: "TestBroker".to_string(),
                        account_name: format!("Account {}", i),
                        server: "TestServer".to_string(),
                        balance: 10000.0 + i as f64,
                        equity: 10000.0 + i as f64,
                        currency: "USD".to_string(),
                        leverage: 100,
                        timestamp: "2025-01-01T00:00:00Z".to_string(),
                    };

                    // This should not panic
                    rmp_serde::to_vec_named(&msg).expect("Serialization failed")
                })
            })
            .collect();

        for handle in handles {
            handle.join().expect("Thread panicked");
        }
    }
}
