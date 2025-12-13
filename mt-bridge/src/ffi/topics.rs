use crate::constants::{self, TOPIC_GLOBAL_CONFIG};
use crate::ffi::helpers::utf16_to_string;

/// Helper function to write a string to UTF-16 buffer
/// Returns the string length (excluding null terminator), 0 on error, -1 if buffer too small
#[inline]
unsafe fn write_string_to_utf16_buffer(s: &str, output: *mut u16, output_len: i32) -> i32 {
    let utf16: Vec<u16> = s.encode_utf16().chain(std::iter::once(0)).collect();

    if utf16.len() > output_len as usize {
        return -1; // Buffer too small
    }

    let output_slice = std::slice::from_raw_parts_mut(output, output_len as usize);
    output_slice[..utf16.len()].copy_from_slice(&utf16);

    (utf16.len() - 1) as i32 // Return length without null terminator
}

/// Build a config topic string: "config/{account_id}"
///
/// # Safety
/// - account_id must be a valid null-terminated UTF-16 string pointer
/// - output must be a valid buffer of at least output_len u16 elements
/// - output_len must be positive
#[no_mangle]
pub unsafe extern "C" fn build_config_topic(
    account_id: *const u16,
    output: *mut u16,
    output_len: i32,
) -> i32 {
    if account_id.is_null() || output.is_null() || output_len <= 0 {
        return 0;
    }

    let account = match utf16_to_string(account_id) {
        Some(s) => s,
        None => return 0,
    };

    let topic = constants::build_config_topic(&account);
    write_string_to_utf16_buffer(&topic, output, output_len)
}

/// Build a trade topic string: "trade/{master_id}/{slave_id}"
///
/// # Safety
/// - master_id must be a valid null-terminated UTF-16 string pointer
/// - slave_id must be a valid null-terminated UTF-16 string pointer
/// - output must be a valid buffer of at least output_len u16 elements
/// - output_len must be positive
#[no_mangle]
pub unsafe extern "C" fn build_trade_topic(
    master_id: *const u16,
    slave_id: *const u16,
    output: *mut u16,
    output_len: i32,
) -> i32 {
    if master_id.is_null() || slave_id.is_null() || output.is_null() || output_len <= 0 {
        return 0;
    }

    let master = match utf16_to_string(master_id) {
        Some(s) => s,
        None => return 0,
    };

    let slave = match utf16_to_string(slave_id) {
        Some(s) => s,
        None => return 0,
    };

    let topic = constants::build_trade_topic(&master, &slave);
    write_string_to_utf16_buffer(&topic, output, output_len)
}

/// Get the global config topic string: "config/global"
///
/// # Safety
/// - output must be a valid buffer of at least output_len u16 elements
/// - output_len must be positive
#[no_mangle]
pub unsafe extern "C" fn get_global_config_topic(output: *mut u16, output_len: i32) -> i32 {
    if output.is_null() || output_len <= 0 {
        return 0;
    }

    write_string_to_utf16_buffer(TOPIC_GLOBAL_CONFIG, output, output_len)
}

/// Build a sync topic string: "sync/{master_id}/{slave_id}"
///
/// Used for PositionSnapshot (Master → Slave) and SyncRequest (Slave → Master) messages.
///
/// # Safety
/// - master_id must be a valid null-terminated UTF-16 string pointer
/// - slave_id must be a valid null-terminated UTF-16 string pointer
/// - output must be a valid buffer of at least output_len u16 elements
/// - output_len must be positive
///
/// # Returns
/// Length of the generated topic string (not including null terminator), or 0 on error.
#[no_mangle]
pub unsafe extern "C" fn build_sync_topic_ffi(
    master_id: *const u16,
    slave_id: *const u16,
    output: *mut u16,
    output_len: i32,
) -> i32 {
    if master_id.is_null() || slave_id.is_null() || output.is_null() || output_len <= 0 {
        return 0;
    }

    let master = match utf16_to_string(master_id) {
        Some(s) => s,
        None => return 0,
    };

    let slave = match utf16_to_string(slave_id) {
        Some(s) => s,
        None => return 0,
    };

    let topic = constants::build_sync_topic(&master, &slave);
    write_string_to_utf16_buffer(&topic, output, output_len)
}

/// Get the sync topic prefix for a Master EA: "sync/{account_id}/"
///
/// Master EAs subscribe to this prefix to receive all SyncRequest messages
/// from any slave connected to them.
///
/// # Safety
/// - account_id must be a valid null-terminated UTF-16 string pointer
/// - output must be a valid buffer of at least output_len u16 elements
/// - output_len must be positive
///
/// # Returns
/// Length of the generated topic prefix (not including null terminator), or 0 on error.
#[no_mangle]
pub unsafe extern "C" fn get_sync_topic_prefix(
    account_id: *const u16,
    output: *mut u16,
    output_len: i32,
) -> i32 {
    if account_id.is_null() || output.is_null() || output_len <= 0 {
        return 0;
    }

    let account = match utf16_to_string(account_id) {
        Some(s) => s,
        None => return 0,
    };

    // Format: "sync/{account_id}/"
    let topic_prefix = format!("{}{}/", constants::TOPIC_SYNC_PREFIX, account);
    write_string_to_utf16_buffer(&topic_prefix, output, output_len)
}
