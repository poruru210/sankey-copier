// Location: mt-bridge/src/ffi_helpers.rs
// Purpose: UTF-16 conversion helper functions and FFI utilities for MQL4/MQL5
// Why: MQL uses UTF-16 strings, so we need bidirectional conversion utilities
//      Also provides generic helpers to reduce FFI boilerplate code

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

/// Helper function to copy a Rust string into a fixed-size byte array (null-terminated)
/// Safely truncates if string is too long, ensuring a null terminator exists (or at least valid cut).
///
/// This is intended for C-compatible structs with fixed size char arrays (e.g. `[u8; 64]`).
/// The output will be valid UTF-8, truncated at character boundaries.
pub fn copy_string_to_fixed_array<const N: usize>(s: &str, arr: &mut [u8; N]) {
    let max_len = N - 1; // Leave room for null terminator

    let bytes = if s.len() <= max_len {
        s.as_bytes()
    } else {
        // Find safe cut point (UTF-8 char boundary)
        let mut end = max_len;
        while end > 0 && !is_char_boundary(s, end) {
            end -= 1;
        }
        &s.as_bytes()[..end]
    };

    arr[..bytes.len()].copy_from_slice(bytes);
    // Fill the rest with 0s (ensures null termination)
    arr[bytes.len()..].fill(0);
}

// Internal helper for character boundary check - Made public for use in ffi.rs
pub fn is_char_boundary(s: &str, index: usize) -> bool {
    if index == 0 {
        return true;
    }
    match s.as_bytes().get(index) {
        // 10xxxxxx (0x80 .. 0xBF) means continuation byte.
        // So valid boundary is NOT (0x80 & b != 0)
        // i.e. (b & 0xC0) != 0x80
        Some(&b) => (b & 0xC0) != 0x80,
        None => true, // End of string is boundary
    }
}

// =============================================================================
// Tests for helper functions
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copy_string_to_fixed_array() {
        let mut arr = [0u8; 10];
        copy_string_to_fixed_array("Hello", &mut arr);
        assert_eq!(&arr[..6], b"Hello\0");

        let mut arr2 = [0u8; 5];
        copy_string_to_fixed_array("Hello World", &mut arr2);
        // "Hell" + null
        assert_eq!(&arr2[..5], b"Hell\0");

        let mut arr3 = [0u8; 5];
        // Japanese: "こんにちは" (3 bytes each)
        // [227, 129, 147] (こ)
        // [227, 129, 147, 227, 129, 147] > 5
        // Safe cut for 5 bytes (actually 4 bytes for content)
        // "こ" takes 3 bytes. Next is "ん" (3 bytes).
        // 3 bytes fit. 4 bytes don't fit 6 bytes.
        // So it should contain "こ" and null.
        copy_string_to_fixed_array("こんにちは", &mut arr3);
        let s = std::str::from_utf8(&arr3[..3]).unwrap();
        assert_eq!(s, "こ");
        assert_eq!(arr3[3], 0);
    }
}
