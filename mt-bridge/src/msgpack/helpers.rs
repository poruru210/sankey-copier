// Location: mt-bridge/src/msgpack/helpers.rs
// Purpose: UTF-16 conversion helper functions for FFI interop with MQL4/MQL5
// Why: MQL uses UTF-16 strings, so we need bidirectional conversion utilities

use std::sync::{LazyLock, Mutex};

// Static buffers for returning UTF-16 strings to MQL5 (supports up to 4 concurrent strings)
pub(crate) const MAX_STRING_LEN: usize = 512;
pub(crate) static STRING_BUFFER_1: LazyLock<Mutex<Vec<u16>>> =
    LazyLock::new(|| Mutex::new(vec![0; MAX_STRING_LEN]));
pub(crate) static STRING_BUFFER_2: LazyLock<Mutex<Vec<u16>>> =
    LazyLock::new(|| Mutex::new(vec![0; MAX_STRING_LEN]));
pub(crate) static STRING_BUFFER_3: LazyLock<Mutex<Vec<u16>>> =
    LazyLock::new(|| Mutex::new(vec![0; MAX_STRING_LEN]));
pub(crate) static STRING_BUFFER_4: LazyLock<Mutex<Vec<u16>>> =
    LazyLock::new(|| Mutex::new(vec![0; MAX_STRING_LEN]));
pub(crate) static BUFFER_INDEX: LazyLock<Mutex<usize>> = LazyLock::new(|| Mutex::new(0));

/// Convert UTF-16 string to Rust String
/// Returns None if pointer is null or conversion fails
pub unsafe fn utf16_to_string(ptr: *const u16) -> Option<String> {
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
/// This is useful for optional fields where empty strings should be treated as None
pub unsafe fn utf16_to_string_opt(ptr: *const u16) -> Option<String> {
    let s = utf16_to_string(ptr)?;
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// Convert Rust String to UTF-16 buffer
/// Uses round-robin buffer allocation to support up to 4 concurrent string returns
/// Returns a pointer to the static buffer (valid until next 4 calls)
pub unsafe fn string_to_utf16_buffer(s: &str) -> *const u16 {
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
