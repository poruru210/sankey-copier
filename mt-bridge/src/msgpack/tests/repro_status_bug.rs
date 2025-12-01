#[cfg(test)]
mod tests {
    use crate::ffi::master_config_get_int;
    use crate::types::MasterConfigMessage;

    #[test]
    fn test_master_config_get_int_status() {
        // Create a MasterConfigMessage with a specific status
        let config = MasterConfigMessage {
            account_id: "test_account".to_string(),
            status: 2, // STATUS_CONNECTED
            symbol_prefix: None,
            symbol_suffix: None,
            config_version: 1,
            timestamp: "2023-01-01".to_string(),
        };

        // Serialize it (simulating what happens when EA receives it)
        // In FFI tests we usually work with the struct directly if we can,
        // but here we are testing the FFI function which takes a pointer to the struct.
        // Wait, parse_master_config returns a *mut MasterConfigMessage.
        // Let's just create the struct and pass a pointer to it,
        // assuming the FFI function expects a pointer to the struct (which it does).

        let config_ptr = &config as *const MasterConfigMessage;

        // Create UTF-16 string for "status"
        let field_name = "status";
        let field_name_utf16: Vec<u16> = field_name
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            let status = master_config_get_int(config_ptr, field_name_utf16.as_ptr());
            assert_eq!(status, 2, "Should return status 2");
        }
    }
}
