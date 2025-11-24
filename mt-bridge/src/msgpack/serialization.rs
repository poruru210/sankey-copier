// Location: mt-bridge/src/msgpack/serialization.rs
// Purpose: MessagePack serialization/deserialization functions for FFI
// Why: Provides binary serialization for efficient message passing between EA and relay-server

use std::sync::{LazyLock, Mutex};
use super::helpers::{utf16_to_string, utf16_to_string_opt};
use super::types::{RequestConfigMessage, UnregisterMessage, HeartbeatMessage, TradeSignalMessage};

// Static buffer for serialized data
static SERIALIZE_BUFFER: LazyLock<Mutex<Vec<u8>>> =
    LazyLock::new(|| Mutex::new(Vec::with_capacity(8192)));

/// Serialize a RequestConfigMessage to MessagePack
///
/// Returns the length of serialized data (or 0 on error).
/// The serialized data is stored in an internal buffer accessible via copy_serialized_buffer().
///
/// # Safety
/// - All pointer parameters must be valid null-terminated UTF-16 string pointers
#[no_mangle]
pub unsafe extern "C" fn serialize_request_config(
    message_type: *const u16,
    account_id: *const u16,
    timestamp: *const u16,
    ea_type: *const u16,
) -> i32 {
    let msg = RequestConfigMessage {
        message_type: utf16_to_string(message_type).unwrap_or_default(),
        account_id: utf16_to_string(account_id).unwrap_or_default(),
        timestamp: utf16_to_string(timestamp).unwrap_or_default(),
        ea_type: utf16_to_string(ea_type).unwrap_or_default(),
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
///
/// # Safety
/// - All pointer parameters must be valid null-terminated UTF-16 string pointers
#[no_mangle]
pub unsafe extern "C" fn serialize_unregister(
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
///
/// # Safety
/// - All pointer parameters must be valid null-terminated UTF-16 string pointers
#[no_mangle]
pub unsafe extern "C" fn serialize_heartbeat(
    message_type: *const u16,
    account_id: *const u16,
    balance: f64,
    equity: f64,
    open_positions: i32,
    timestamp: *const u16,
    ea_type: *const u16,
    platform: *const u16,
    account_number: i64,
    broker: *const u16,
    account_name: *const u16,
    server: *const u16,
    currency: *const u16,
    leverage: i64,
    is_trade_allowed: i32,
    symbol_prefix: *const u16,
    symbol_suffix: *const u16,
    symbol_map: *const u16,
) -> i32 {
    let msg = HeartbeatMessage {
        message_type: utf16_to_string(message_type).unwrap_or_default(),
        account_id: utf16_to_string(account_id).unwrap_or_default(),
        balance,
        equity,
        open_positions,
        timestamp: utf16_to_string(timestamp).unwrap_or_default(),
        version: env!("BUILD_INFO").to_string(),
        ea_type: utf16_to_string(ea_type).unwrap_or_default(),
        platform: utf16_to_string(platform).unwrap_or_default(),
        account_number,
        broker: utf16_to_string(broker).unwrap_or_default(),
        account_name: utf16_to_string(account_name).unwrap_or_default(),
        server: utf16_to_string(server).unwrap_or_default(),
        currency: utf16_to_string(currency).unwrap_or_default(),
        leverage,
        is_trade_allowed: is_trade_allowed != 0,
        symbol_prefix: utf16_to_string_opt(symbol_prefix),
        symbol_suffix: utf16_to_string_opt(symbol_suffix),
        symbol_map: utf16_to_string_opt(symbol_map),
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
///
/// # Safety
/// - All pointer parameters must be valid null-terminated UTF-16 string pointers
#[no_mangle]
pub unsafe extern "C" fn serialize_trade_signal(
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
        open_price: if open_price > 0.0 {
            Some(open_price)
        } else {
            None
        },
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
pub unsafe extern "C" fn get_serialized_buffer() -> *const u8 {
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
pub unsafe extern "C" fn copy_serialized_buffer(dest: *mut u8, max_len: i32) -> i32 {
    if dest.is_null() || max_len <= 0 {
        return 0;
    }

    let buffer = SERIALIZE_BUFFER.lock().unwrap();
    let len = buffer.len();

    if len > max_len as usize {
        eprintln!(
            "msgpack_copy_buffer: buffer size {} exceeds max_len {}",
            len, max_len
        );
        return 0;
    }

    std::ptr::copy_nonoverlapping(buffer.as_ptr(), dest, len);
    len as i32
}
