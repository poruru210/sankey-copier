// Location: mt-bridge/src/msgpack/tests/ffi_tests.rs
// Purpose: Tests for FFI functions used by MQL4/MQL5
// Why: Ensures FFI boundary works correctly with UTF-16 strings and handle management

use crate::ffi::*;
use crate::msgpack::*;

// Note: Tests for parse_master_config, parse_position_snapshot, parse_sync_request,
// and their corresponding free functions have been removed as those FFI functions
// are no longer exported. Handles are now obtained via ea_context_get_*() functions.

// =============================================================================
// PositionSnapshot Builder FFI Tests
// =============================================================================

#[test]
fn test_position_snapshot_builder_ffi() {
    // Test creating PositionSnapshot using builder FFI
    unsafe {
        let account_utf16: Vec<u16> = "BUILDER_MASTER_001".encode_utf16().chain(Some(0)).collect();
        let handle = create_position_snapshot_builder(account_utf16.as_ptr());
        assert!(
            !handle.is_null(),
            "create_position_snapshot_builder should return valid handle"
        );

        // Add first position
        let symbol1_utf16: Vec<u16> = "EURUSD".encode_utf16().chain(Some(0)).collect();
        let order_type1_utf16: Vec<u16> = "Buy".encode_utf16().chain(Some(0)).collect();
        let open_time1_utf16: Vec<u16> = "2025-01-15T10:00:00Z"
            .encode_utf16()
            .chain(Some(0))
            .collect();

        let result1 = position_snapshot_builder_add_position(
            handle,
            12345,                      // ticket
            symbol1_utf16.as_ptr(),     // symbol
            order_type1_utf16.as_ptr(), // order_type
            0.1,                        // lots
            1.08500,                    // open_price
            1.08000,                    // stop_loss
            1.09000,                    // take_profit
            100,                        // magic_number
            open_time1_utf16.as_ptr(),  // open_time
        );
        assert_eq!(result1, 1, "First position should be added successfully");

        // Add second position
        let symbol2_utf16: Vec<u16> = "USDJPY".encode_utf16().chain(Some(0)).collect();
        let order_type2_utf16: Vec<u16> = "Sell".encode_utf16().chain(Some(0)).collect();
        let open_time2_utf16: Vec<u16> = "2025-01-15T10:30:00Z"
            .encode_utf16()
            .chain(Some(0))
            .collect();

        let result2 = position_snapshot_builder_add_position(
            handle,
            12346,                      // ticket
            symbol2_utf16.as_ptr(),     // symbol
            order_type2_utf16.as_ptr(), // order_type
            0.2,                        // lots
            150.500,                    // open_price
            0.0,                        // stop_loss (0 = none)
            0.0,                        // take_profit (0 = none)
            0,                          // magic_number (0 = none)
            open_time2_utf16.as_ptr(),  // open_time
        );
        assert_eq!(result2, 1, "Second position should be added successfully");

        // Serialize into pre-allocated buffer
        let mut buffer: Vec<u8> = vec![0u8; 4096];
        let bytes_written =
            position_snapshot_builder_serialize(handle, buffer.as_mut_ptr(), buffer.len() as i32);
        assert!(bytes_written > 0, "Serialization should succeed");

        // Deserialize and verify
        let serialized = &buffer[..bytes_written as usize];
        let deserialized: PositionSnapshotMessage =
            rmp_serde::from_slice(serialized).expect("Failed to deserialize");

        assert_eq!(deserialized.source_account, "BUILDER_MASTER_001");
        assert_eq!(deserialized.positions.len(), 2);
        assert_eq!(deserialized.positions[0].symbol, "EURUSD");
        assert_eq!(deserialized.positions[0].ticket, 12345);
        assert!((deserialized.positions[0].lots - 0.1).abs() < 0.001);
        assert_eq!(deserialized.positions[1].symbol, "USDJPY");
        assert_eq!(deserialized.positions[1].ticket, 12346);

        // Free the handle
        position_snapshot_builder_free(handle);
    }
}

#[test]
fn test_position_snapshot_builder_ffi_empty() {
    // Test creating empty PositionSnapshot
    unsafe {
        let account_utf16: Vec<u16> = "EMPTY_MASTER".encode_utf16().chain(Some(0)).collect();
        let handle = create_position_snapshot_builder(account_utf16.as_ptr());
        assert!(!handle.is_null(), "Should create valid handle");

        // Serialize without adding any positions
        let mut buffer: Vec<u8> = vec![0u8; 1024];
        let bytes_written =
            position_snapshot_builder_serialize(handle, buffer.as_mut_ptr(), buffer.len() as i32);
        assert!(bytes_written > 0, "Serialization should succeed");

        // Deserialize and verify
        let serialized = &buffer[..bytes_written as usize];
        let deserialized: PositionSnapshotMessage =
            rmp_serde::from_slice(serialized).expect("Failed to deserialize");

        assert_eq!(deserialized.source_account, "EMPTY_MASTER");
        assert_eq!(deserialized.positions.len(), 0);

        position_snapshot_builder_free(handle);
    }
}
