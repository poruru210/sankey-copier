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

// =============================================================================
// PositionSnapshot FFI Tests
// =============================================================================

#[test]
fn test_parse_position_snapshot_ffi() {
    // Test parsing PositionSnapshotMessage via FFI
    let msg = PositionSnapshotMessage {
        message_type: "PositionSnapshot".to_string(),
        source_account: "MASTER_ACCOUNT_001".to_string(),
        positions: vec![
            PositionInfo {
                ticket: 12345,
                symbol: "EURUSD".to_string(),
                order_type: "Buy".to_string(),
                lots: 0.1,
                open_price: 1.08500,
                open_time: "2025-01-15T10:00:00Z".to_string(),
                stop_loss: Some(1.08000),
                take_profit: Some(1.09000),
                magic_number: Some(100),
                comment: Some("Test order".to_string()),
            },
            PositionInfo {
                ticket: 12346,
                symbol: "USDJPY".to_string(),
                order_type: "Sell".to_string(),
                lots: 0.2,
                open_price: 150.500,
                open_time: "2025-01-15T10:30:00Z".to_string(),
                stop_loss: None,
                take_profit: None,
                magic_number: None,
                comment: None,
            },
        ],
        timestamp: "2025-01-15T11:00:00Z".to_string(),
    };

    let serialized = rmp_serde::to_vec_named(&msg).expect("Failed to serialize");

    unsafe {
        let handle = parse_position_snapshot(serialized.as_ptr(), serialized.len() as i32);
        assert!(
            !handle.is_null(),
            "parse_position_snapshot should return valid handle"
        );

        // Test position_snapshot_get_string for source_account
        let source_utf16: Vec<u16> = "source_account".encode_utf16().chain(Some(0)).collect();
        let source_ptr = position_snapshot_get_string(handle, source_utf16.as_ptr());
        assert!(
            !source_ptr.is_null(),
            "source_account should be retrievable"
        );

        // Test position_snapshot_get_positions_count
        let count = position_snapshot_get_positions_count(handle);
        assert_eq!(count, 2, "Should have 2 positions");

        // Test position_snapshot_get_position_string
        let symbol_utf16: Vec<u16> = "symbol".encode_utf16().chain(Some(0)).collect();
        let symbol_ptr = position_snapshot_get_position_string(handle, 0, symbol_utf16.as_ptr());
        assert!(!symbol_ptr.is_null(), "symbol should be retrievable");

        // Test position_snapshot_get_position_double
        let lots_utf16: Vec<u16> = "lots".encode_utf16().chain(Some(0)).collect();
        let lots = position_snapshot_get_position_double(handle, 0, lots_utf16.as_ptr());
        assert!((lots - 0.1).abs() < 0.001, "lots should be 0.1");

        let open_price_utf16: Vec<u16> = "open_price".encode_utf16().chain(Some(0)).collect();
        let open_price =
            position_snapshot_get_position_double(handle, 0, open_price_utf16.as_ptr());
        assert!(
            (open_price - 1.08500).abs() < 0.00001,
            "open_price should be 1.08500"
        );

        // Test position_snapshot_get_position_int
        let ticket_utf16: Vec<u16> = "ticket".encode_utf16().chain(Some(0)).collect();
        let ticket = position_snapshot_get_position_int(handle, 0, ticket_utf16.as_ptr());
        assert_eq!(ticket, 12345, "ticket should be 12345");

        // Test second position
        let symbol_ptr2 = position_snapshot_get_position_string(handle, 1, symbol_utf16.as_ptr());
        assert!(
            !symbol_ptr2.is_null(),
            "second position symbol should be retrievable"
        );

        let lots2 = position_snapshot_get_position_double(handle, 1, lots_utf16.as_ptr());
        assert!(
            (lots2 - 0.2).abs() < 0.001,
            "second position lots should be 0.2"
        );

        // Free the handle
        position_snapshot_free(handle);
    }
}

#[test]
fn test_parse_position_snapshot_ffi_empty_positions() {
    // Test parsing with empty positions array
    let msg = PositionSnapshotMessage {
        message_type: "PositionSnapshot".to_string(),
        source_account: "EMPTY_MASTER".to_string(),
        positions: vec![],
        timestamp: "2025-01-15T12:00:00Z".to_string(),
    };

    let serialized = rmp_serde::to_vec_named(&msg).expect("Failed to serialize");

    unsafe {
        let handle = parse_position_snapshot(serialized.as_ptr(), serialized.len() as i32);
        assert!(
            !handle.is_null(),
            "parse_position_snapshot should return valid handle for empty positions"
        );

        let count = position_snapshot_get_positions_count(handle);
        assert_eq!(count, 0, "Should have 0 positions");

        position_snapshot_free(handle);
    }
}

#[test]
fn test_parse_position_snapshot_ffi_invalid_data() {
    // Test with invalid MessagePack data
    let invalid_data: Vec<u8> = vec![0xFF, 0xFF, 0xFF, 0xFF];

    unsafe {
        let handle = parse_position_snapshot(invalid_data.as_ptr(), invalid_data.len() as i32);
        assert!(
            handle.is_null(),
            "parse_position_snapshot should return null for invalid data"
        );
    }
}

// =============================================================================
// SyncRequest FFI Tests
// =============================================================================

#[test]
fn test_parse_sync_request_ffi() {
    // Test parsing SyncRequestMessage via FFI
    let msg = SyncRequestMessage {
        message_type: "SyncRequest".to_string(),
        slave_account: "SLAVE_ACCOUNT_001".to_string(),
        master_account: "MASTER_ACCOUNT_001".to_string(),
        last_sync_time: Some("2025-01-15T09:00:00Z".to_string()),
        timestamp: "2025-01-15T10:00:00Z".to_string(),
    };

    let serialized = rmp_serde::to_vec_named(&msg).expect("Failed to serialize");

    unsafe {
        let handle = parse_sync_request(serialized.as_ptr(), serialized.len() as i32);
        assert!(
            !handle.is_null(),
            "parse_sync_request should return valid handle"
        );

        // Test sync_request_get_string for slave_account
        let slave_utf16: Vec<u16> = "slave_account".encode_utf16().chain(Some(0)).collect();
        let slave_ptr = sync_request_get_string(handle, slave_utf16.as_ptr());
        assert!(!slave_ptr.is_null(), "slave_account should be retrievable");

        // Test sync_request_get_string for master_account
        let master_utf16: Vec<u16> = "master_account".encode_utf16().chain(Some(0)).collect();
        let master_ptr = sync_request_get_string(handle, master_utf16.as_ptr());
        assert!(
            !master_ptr.is_null(),
            "master_account should be retrievable"
        );

        // Test sync_request_get_string for last_sync_time
        let last_sync_utf16: Vec<u16> = "last_sync_time".encode_utf16().chain(Some(0)).collect();
        let last_sync_ptr = sync_request_get_string(handle, last_sync_utf16.as_ptr());
        assert!(
            !last_sync_ptr.is_null(),
            "last_sync_time should be retrievable"
        );

        // Free the handle
        sync_request_free(handle);
    }
}

#[test]
fn test_parse_sync_request_ffi_no_last_sync() {
    // Test parsing without last_sync_time
    let msg = SyncRequestMessage {
        message_type: "SyncRequest".to_string(),
        slave_account: "SLAVE_ACCOUNT_002".to_string(),
        master_account: "MASTER_ACCOUNT_002".to_string(),
        last_sync_time: None,
        timestamp: "2025-01-15T10:00:00Z".to_string(),
    };

    let serialized = rmp_serde::to_vec_named(&msg).expect("Failed to serialize");

    unsafe {
        let handle = parse_sync_request(serialized.as_ptr(), serialized.len() as i32);
        assert!(
            !handle.is_null(),
            "parse_sync_request should return valid handle"
        );

        // last_sync_time should return empty string for None
        let last_sync_utf16: Vec<u16> = "last_sync_time".encode_utf16().chain(Some(0)).collect();
        let last_sync_ptr = sync_request_get_string(handle, last_sync_utf16.as_ptr());
        assert!(
            !last_sync_ptr.is_null(),
            "last_sync_time should return valid pointer even for None"
        );

        sync_request_free(handle);
    }
}

#[test]
fn test_parse_sync_request_ffi_invalid_data() {
    // Test with invalid MessagePack data
    let invalid_data: Vec<u8> = vec![0xFF, 0xFF, 0xFF, 0xFF];

    unsafe {
        let handle = parse_sync_request(invalid_data.as_ptr(), invalid_data.len() as i32);
        assert!(
            handle.is_null(),
            "parse_sync_request should return null for invalid data"
        );
    }
}

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

#[test]
fn test_create_sync_request_ffi() {
    // Test creating SyncRequest via FFI
    unsafe {
        let slave_utf16: Vec<u16> = "SLAVE_001".encode_utf16().chain(Some(0)).collect();
        let master_utf16: Vec<u16> = "MASTER_001".encode_utf16().chain(Some(0)).collect();

        let mut buffer: Vec<u8> = vec![0u8; 1024];
        let bytes_written = create_sync_request(
            slave_utf16.as_ptr(),
            master_utf16.as_ptr(),
            buffer.as_mut_ptr(),
            buffer.len() as i32,
        );

        assert!(bytes_written > 0, "create_sync_request should succeed");

        // Deserialize and verify
        let serialized = &buffer[..bytes_written as usize];
        let deserialized: SyncRequestMessage =
            rmp_serde::from_slice(serialized).expect("Failed to deserialize");

        assert_eq!(deserialized.message_type, "SyncRequest");
        assert_eq!(deserialized.slave_account, "SLAVE_001");
        assert_eq!(deserialized.master_account, "MASTER_001");
    }
}
