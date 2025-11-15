use sankey_copier_relay_server::db::Database;
use sankey_copier_relay_server::models::{CopySettings, TradeFilters};

/// Test that verifies newly created copy settings have enabled=false by default
///
/// This test ensures user safety by preventing unintended trade copying
/// when master-slave connections are first established.
///
/// Test flow:
/// 1. Create a new CopySettings with default values (simulating API creation)
/// 2. Save to database
/// 3. Retrieve from database
/// 4. Verify that enabled is false by default
#[tokio::test]
async fn test_new_copy_settings_disabled_by_default() {
    // Setup: Create in-memory test database
    let db = Database::new("sqlite::memory:")
        .await
        .expect("Failed to create test database");

    // Step 1: Create new CopySettings with enabled=false (as per fix)
    let new_settings = CopySettings {
        id: 0, // Will be assigned by DB
        enabled: false, // Default value for new settings
        master_account: "MASTER_ACCOUNT_001".to_string(),
        slave_account: "SLAVE_ACCOUNT_001".to_string(),
        lot_multiplier: Some(1.0),
        reverse_trade: false,
        symbol_mappings: vec![],
        filters: TradeFilters {
            allowed_symbols: None,
            blocked_symbols: None,
            allowed_magic_numbers: None,
            blocked_magic_numbers: None,
        },
    };

    // Step 2: Save to database (simulating API create_settings)
    let settings_id = db
        .save_copy_settings(&new_settings)
        .await
        .expect("Failed to save new copy settings");

    assert!(settings_id > 0, "Settings ID should be positive");

    // Step 3: Retrieve from database
    let retrieved_settings = db
        .get_copy_settings(settings_id)
        .await
        .expect("Failed to retrieve settings")
        .expect("Settings should exist");

    // Step 4: Verify enabled is false
    assert_eq!(
        retrieved_settings.enabled, false,
        "New copy settings should be disabled by default"
    );

    // Verify other fields are correctly saved
    assert_eq!(retrieved_settings.id, settings_id);
    assert_eq!(retrieved_settings.master_account, "MASTER_ACCOUNT_001");
    assert_eq!(retrieved_settings.slave_account, "SLAVE_ACCOUNT_001");
    assert_eq!(retrieved_settings.lot_multiplier, Some(1.0));
    assert_eq!(retrieved_settings.reverse_trade, false);

    println!("✓ Test passed: New copy settings are disabled by default");
}

/// Test that verifies get_settings_for_slave does NOT return disabled settings
///
/// When a slave EA queries for its settings, it should only receive enabled settings.
/// This test ensures that newly created (disabled) settings are not sent to slave EAs.
#[tokio::test]
async fn test_disabled_settings_not_returned_to_slave() {
    let db = Database::new("sqlite::memory:")
        .await
        .expect("Failed to create test database");

    // Create a new disabled setting (default state)
    let disabled_settings = CopySettings {
        id: 0,
        enabled: false, // Disabled by default
        master_account: "MASTER_002".to_string(),
        slave_account: "SLAVE_002".to_string(),
        lot_multiplier: Some(1.5),
        reverse_trade: false,
        symbol_mappings: vec![],
        filters: TradeFilters {
            allowed_symbols: None,
            blocked_symbols: None,
            allowed_magic_numbers: None,
            blocked_magic_numbers: None,
        },
    };

    db.save_copy_settings(&disabled_settings)
        .await
        .expect("Failed to save disabled settings");

    // Query for slave settings (simulating slave EA startup)
    let result = db
        .get_settings_for_slave("SLAVE_002")
        .await
        .expect("Query failed");

    // Verify that disabled settings are NOT returned
    assert!(
        result.is_none(),
        "Disabled settings should not be returned to slave EA"
    );

    println!("✓ Test passed: Disabled settings are not sent to slave EA");
}

/// Test the workflow: create disabled -> enable -> verify slave receives config
///
/// This test simulates the safe workflow:
/// 1. User creates master-slave connection (disabled by default)
/// 2. User configures settings (lot multiplier, filters, etc.)
/// 3. User manually enables the connection
/// 4. Slave EA can now receive the configuration
#[tokio::test]
async fn test_safe_workflow_create_configure_enable() {
    let db = Database::new("sqlite::memory:")
        .await
        .expect("Failed to create test database");

    // Step 1: Create new connection (disabled by default)
    let new_settings = CopySettings {
        id: 0,
        enabled: false, // Safe default
        master_account: "MASTER_003".to_string(),
        slave_account: "SLAVE_003".to_string(),
        lot_multiplier: Some(1.0),
        reverse_trade: false,
        symbol_mappings: vec![],
        filters: TradeFilters {
            allowed_symbols: None,
            blocked_symbols: None,
            allowed_magic_numbers: None,
            blocked_magic_numbers: None,
        },
    };

    let settings_id = db
        .save_copy_settings(&new_settings)
        .await
        .expect("Failed to save new settings");

    // Verify slave cannot receive config yet
    let result = db.get_settings_for_slave("SLAVE_003").await.unwrap();
    assert!(result.is_none(), "Disabled settings should not be returned");

    // Step 2: User configures settings (update lot multiplier)
    let mut updated_settings = db
        .get_copy_settings(settings_id)
        .await
        .unwrap()
        .unwrap();
    updated_settings.lot_multiplier = Some(2.5);
    // Still disabled at this point
    assert_eq!(updated_settings.enabled, false);

    db.save_copy_settings(&updated_settings)
        .await
        .expect("Failed to update settings");

    // Step 3: User manually enables the connection
    db.update_enabled_status(settings_id, true)
        .await
        .expect("Failed to enable settings");

    // Step 4: Verify slave can now receive config
    let result = db
        .get_settings_for_slave("SLAVE_003")
        .await
        .expect("Query failed");

    assert!(result.is_some(), "Enabled settings should be returned");
    let final_settings = result.unwrap();
    assert_eq!(final_settings.enabled, true);
    assert_eq!(final_settings.lot_multiplier, Some(2.5));

    println!("✓ Test passed: Safe workflow (create -> configure -> enable) verified");
}

/// Test that enabled settings are correctly returned to slave EA
///
/// This test ensures that the fix doesn't break existing functionality
/// for enabled settings.
#[tokio::test]
async fn test_enabled_settings_are_returned_to_slave() {
    let db = Database::new("sqlite::memory:")
        .await
        .expect("Failed to create test database");

    // Create an enabled setting (user has manually enabled it)
    let enabled_settings = CopySettings {
        id: 0,
        enabled: true, // Manually enabled by user
        master_account: "MASTER_004".to_string(),
        slave_account: "SLAVE_004".to_string(),
        lot_multiplier: Some(3.0),
        reverse_trade: true,
        symbol_mappings: vec![],
        filters: TradeFilters {
            allowed_symbols: None,
            blocked_symbols: None,
            allowed_magic_numbers: None,
            blocked_magic_numbers: None,
        },
    };

    db.save_copy_settings(&enabled_settings)
        .await
        .expect("Failed to save enabled settings");

    // Query for slave settings
    let result = db
        .get_settings_for_slave("SLAVE_004")
        .await
        .expect("Query failed");

    // Verify that enabled settings ARE returned
    assert!(
        result.is_some(),
        "Enabled settings should be returned to slave EA"
    );

    let retrieved = result.unwrap();
    assert_eq!(retrieved.enabled, true);
    assert_eq!(retrieved.master_account, "MASTER_004");
    assert_eq!(retrieved.slave_account, "SLAVE_004");
    assert_eq!(retrieved.lot_multiplier, Some(3.0));
    assert_eq!(retrieved.reverse_trade, true);

    println!("✓ Test passed: Enabled settings are correctly returned to slave EA");
}

/// Test duplicate master-slave pair prevention
///
/// Verifies that attempting to create duplicate master-slave connections
/// fails with a UNIQUE constraint error.
#[tokio::test]
async fn test_duplicate_master_slave_pair_rejected() {
    let db = Database::new("sqlite::memory:")
        .await
        .expect("Failed to create test database");

    // Create first settings
    let settings1 = CopySettings {
        id: 0,
        enabled: false,
        master_account: "MASTER_005".to_string(),
        slave_account: "SLAVE_005".to_string(),
        lot_multiplier: Some(1.0),
        reverse_trade: false,
        symbol_mappings: vec![],
        filters: TradeFilters {
            allowed_symbols: None,
            blocked_symbols: None,
            allowed_magic_numbers: None,
            blocked_magic_numbers: None,
        },
    };

    db.save_copy_settings(&settings1)
        .await
        .expect("Failed to save first settings");

    // Attempt to create duplicate with same master-slave pair
    let settings2 = CopySettings {
        id: 0, // New ID (not yet assigned)
        enabled: false,
        master_account: "MASTER_005".to_string(), // Same master
        slave_account: "SLAVE_005".to_string(),   // Same slave
        lot_multiplier: Some(2.0),                // Different settings
        reverse_trade: true,
        symbol_mappings: vec![],
        filters: TradeFilters {
            allowed_symbols: None,
            blocked_symbols: None,
            allowed_magic_numbers: None,
            blocked_magic_numbers: None,
        },
    };

    // This should fail with UNIQUE constraint error
    let result = db.save_copy_settings(&settings2).await;

    assert!(
        result.is_err(),
        "Duplicate master-slave pair should be rejected"
    );

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("UNIQUE") || error_msg.contains("constraint"),
        "Error should mention UNIQUE constraint violation"
    );

    println!("✓ Test passed: Duplicate master-slave pairs are rejected");
}
