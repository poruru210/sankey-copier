// Location: mt-bridge/src/msgpack/tests/ffi_tests.rs
// Purpose: Tests for FFI functions used by MQL4/MQL5
// Why: Ensures FFI boundary works correctly with UTF-16 strings and handle management

use crate::msgpack::*;

#[test]
fn test_parse_master_config_ffi() {
    // Test the FFI function parse_master_config()
    let msg = MasterConfigMessage {
        account_id: "test_master_123".to_string(),
        symbol_prefix: Some("pro.".to_string()),
        symbol_suffix: Some(".m".to_string()),
        config_version: 5,
        timestamp: "2025-01-15T10:30:00Z".to_string(),
    };

    let serialized = rmp_serde::to_vec_named(&msg).expect("Failed to serialize");

    // Call FFI function
    unsafe {
        let handle = parse_master_config(serialized.as_ptr(), serialized.len() as i32);
        assert!(
            !handle.is_null(),
            "parse_master_config should return valid handle"
        );

        // Test master_config_get_string
        let account_id_utf16: Vec<u16> = "account_id".encode_utf16().chain(Some(0)).collect();
        let account_id_ptr = master_config_get_string(handle, account_id_utf16.as_ptr());
        assert!(
            !account_id_ptr.is_null(),
            "account_id should be retrievable"
        );

        let prefix_utf16: Vec<u16> = "symbol_prefix".encode_utf16().chain(Some(0)).collect();
        let prefix_ptr = master_config_get_string(handle, prefix_utf16.as_ptr());
        assert!(!prefix_ptr.is_null(), "symbol_prefix should be retrievable");

        let suffix_utf16: Vec<u16> = "symbol_suffix".encode_utf16().chain(Some(0)).collect();
        let suffix_ptr = master_config_get_string(handle, suffix_utf16.as_ptr());
        assert!(!suffix_ptr.is_null(), "symbol_suffix should be retrievable");

        // Test master_config_get_int
        let version_utf16: Vec<u16> = "config_version".encode_utf16().chain(Some(0)).collect();
        let version = master_config_get_int(handle, version_utf16.as_ptr());
        assert_eq!(version, 5, "config_version should be 5");

        // Free the handle
        master_config_free(handle);
    }
}

#[test]
fn test_parse_master_config_ffi_with_none_values() {
    // Test parsing with None values
    let msg = MasterConfigMessage {
        account_id: "test_master_789".to_string(),
        symbol_prefix: None,
        symbol_suffix: None,
        config_version: 0,
        timestamp: "2025-01-15T11:00:00Z".to_string(),
    };

    let serialized = rmp_serde::to_vec_named(&msg).expect("Failed to serialize");

    unsafe {
        let handle = parse_master_config(serialized.as_ptr(), serialized.len() as i32);
        assert!(
            !handle.is_null(),
            "parse_master_config should return valid handle"
        );

        // Get version (should be 0)
        let version_utf16: Vec<u16> = "config_version".encode_utf16().chain(Some(0)).collect();
        let version = master_config_get_int(handle, version_utf16.as_ptr());
        assert_eq!(version, 0, "config_version should be 0");

        // Get prefix (should return empty string for None)
        let prefix_utf16: Vec<u16> = "symbol_prefix".encode_utf16().chain(Some(0)).collect();
        let prefix_ptr = master_config_get_string(handle, prefix_utf16.as_ptr());
        assert!(
            !prefix_ptr.is_null(),
            "symbol_prefix should return valid pointer"
        );

        // Free the handle
        master_config_free(handle);
    }
}

#[test]
fn test_parse_master_config_ffi_invalid_data() {
    // Test with invalid MessagePack data
    let invalid_data: Vec<u8> = vec![0xFF, 0xFF, 0xFF, 0xFF];

    unsafe {
        let handle = parse_master_config(invalid_data.as_ptr(), invalid_data.len() as i32);
        assert!(
            handle.is_null(),
            "parse_master_config should return null for invalid data"
        );
    }
}
