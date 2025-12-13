use crate::ea_context::EaContext;
use crate::ffi::helpers::utf16_to_string;

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
