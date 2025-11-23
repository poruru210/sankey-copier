pub mod msgpack;

#[cfg(test)]
mod symbol_filter_tests;

use std::os::raw::c_char;
use std::ptr;
use std::sync::{LazyLock, Mutex};

// Re-export message types for use in relay-server
pub use msgpack::{
    ConfigMessage, HeartbeatMessage, RequestConfigMessage, SymbolMapping, TradeFilters,
    TradeSignalMessage, UnregisterMessage,
};

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
        Ok(s) => Box::new(s),
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
