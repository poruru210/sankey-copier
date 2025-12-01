// Location: mt-bridge/src/ffi_helpers.rs
// Purpose: UTF-16 conversion helper functions and FFI utilities for MQL4/MQL5
// Why: MQL uses UTF-16 strings, so we need bidirectional conversion utilities
//      Also provides generic helpers to reduce FFI boilerplate code

use serde::de::DeserializeOwned;
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
///
/// # Safety
/// - ptr must be a valid pointer to a null-terminated UTF-16 string, or null
/// - The UTF-16 string must remain valid for the duration of this function call
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
///
/// # Safety
/// - ptr must be a valid pointer to a null-terminated UTF-16 string, or null
/// - The UTF-16 string must remain valid for the duration of this function call
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
///
/// # Safety
/// - The returned pointer is valid until 4 more calls to this function
/// - Caller must not modify the data pointed to by the returned pointer
/// - This function is marked unsafe because it returns a raw pointer to static data
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

// =============================================================================
// Generic FFI Helpers
// =============================================================================

/// Generic MessagePack parser for FFI
/// Reduces boilerplate in parse_* functions by providing type-safe deserialization
///
/// # Safety
/// - data must be a valid pointer to a buffer of at least data_len bytes
/// - Caller must free the returned pointer with the appropriate *_free() function
///
/// # Returns
/// - Valid pointer to heap-allocated T on success
/// - Null pointer on failure (null input, invalid length, or parse error)
pub(crate) unsafe fn parse_msgpack<T: DeserializeOwned>(data: *const u8, data_len: i32) -> *mut T {
    if data.is_null() || data_len <= 0 {
        return std::ptr::null_mut();
    }

    let slice = std::slice::from_raw_parts(data, data_len as usize);
    match rmp_serde::from_slice::<T>(slice) {
        Ok(msg) => Box::into_raw(Box::new(msg)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Generic handle free function
/// Reduces boilerplate in *_free() functions by providing type-safe cleanup
///
/// # Safety
/// - handle must be a valid pointer created by parse_msgpack or null
/// - handle must not be used after calling this function
pub(crate) unsafe fn free_handle<T>(handle: *mut T) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

// =============================================================================
// Tests for helper functions
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestMessage {
        id: i32,
        name: String,
        value: Option<f64>,
    }

    #[test]
    fn test_parse_msgpack_valid_data() {
        let msg = TestMessage {
            id: 42,
            name: "test".to_string(),
            value: Some(3.15),
        };
        let serialized = rmp_serde::to_vec_named(&msg).expect("Failed to serialize");

        unsafe {
            let handle: *mut TestMessage =
                parse_msgpack(serialized.as_ptr(), serialized.len() as i32);
            assert!(!handle.is_null(), "Should return valid handle");

            let parsed = &*handle;
            assert_eq!(parsed.id, 42);
            assert_eq!(parsed.name, "test");
            assert_eq!(parsed.value, Some(3.15));

            free_handle(handle);
        }
    }

    #[test]
    fn test_parse_msgpack_null_data() {
        unsafe {
            let handle: *mut TestMessage = parse_msgpack(std::ptr::null(), 10);
            assert!(handle.is_null(), "Should return null for null input");
        }
    }

    #[test]
    fn test_parse_msgpack_zero_length() {
        let data = [0u8; 10];
        unsafe {
            let handle: *mut TestMessage = parse_msgpack(data.as_ptr(), 0);
            assert!(handle.is_null(), "Should return null for zero length");
        }
    }

    #[test]
    fn test_parse_msgpack_negative_length() {
        let data = [0u8; 10];
        unsafe {
            let handle: *mut TestMessage = parse_msgpack(data.as_ptr(), -1);
            assert!(handle.is_null(), "Should return null for negative length");
        }
    }

    #[test]
    fn test_parse_msgpack_invalid_data() {
        let invalid_data = [0xFF, 0xFF, 0xFF, 0xFF];
        unsafe {
            let handle: *mut TestMessage =
                parse_msgpack(invalid_data.as_ptr(), invalid_data.len() as i32);
            assert!(handle.is_null(), "Should return null for invalid data");
        }
    }

    #[test]
    fn test_free_handle_null() {
        // Should not panic when freeing null handle
        unsafe {
            free_handle::<TestMessage>(std::ptr::null_mut());
        }
    }

    #[test]
    fn test_utf16_to_string_valid() {
        let test_str = "Hello, 世界!";
        let utf16: Vec<u16> = test_str.encode_utf16().chain(Some(0)).collect();

        unsafe {
            let result = utf16_to_string(utf16.as_ptr());
            assert_eq!(result, Some(test_str.to_string()));
        }
    }

    #[test]
    fn test_utf16_to_string_empty() {
        let utf16: Vec<u16> = vec![0]; // Just null terminator

        unsafe {
            let result = utf16_to_string(utf16.as_ptr());
            assert_eq!(result, Some(String::new()));
        }
    }

    #[test]
    fn test_utf16_to_string_null() {
        unsafe {
            let result = utf16_to_string(std::ptr::null());
            assert_eq!(result, None);
        }
    }

    #[test]
    fn test_string_to_utf16_buffer_roundtrip() {
        let test_cases = vec!["Hello", "日本語テスト", "Mixed: 123 and αβγ", ""];

        for test_str in test_cases {
            unsafe {
                let ptr = string_to_utf16_buffer(test_str);
                let result = utf16_to_string(ptr);
                assert_eq!(
                    result,
                    Some(test_str.to_string()),
                    "Roundtrip failed for: {}",
                    test_str
                );
            }
        }
    }

    #[test]
    fn test_utf16_to_string_opt_empty_becomes_none() {
        let utf16: Vec<u16> = vec![0]; // Just null terminator

        unsafe {
            let result = utf16_to_string_opt(utf16.as_ptr());
            assert_eq!(result, None, "Empty string should become None");
        }
    }

    #[test]
    fn test_utf16_to_string_opt_non_empty() {
        let test_str = "non-empty";
        let utf16: Vec<u16> = test_str.encode_utf16().chain(Some(0)).collect();

        unsafe {
            let result = utf16_to_string_opt(utf16.as_ptr());
            assert_eq!(result, Some(test_str.to_string()));
        }
    }
}
