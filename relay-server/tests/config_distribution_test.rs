use sankey_copier_relay_server::db::Database;
use sankey_copier_relay_server::models::{
    ConfigMessage, CopySettings, SymbolMapping, TradeFilters,
};

/// Integration test for CONFIG message distribution workflow
///
/// This test verifies the end-to-end flow:
/// 1. Create a CopySettings record in the database
/// 2. Retrieve it from the database
/// 3. Convert to ConfigMessage (using From trait)
/// 4. Serialize to JSON
/// 5. Verify all fields are correctly serialized
#[tokio::test]
async fn test_config_message_distribution_flow() {
    // Setup: Create in-memory test database
    let db = Database::new("sqlite::memory:")
        .await
        .expect("Failed to create test database");

    // Step 1: Create a comprehensive CopySettings record
    let test_settings = CopySettings {
        id: 0,     // Will be assigned by DB
        status: 2, // STATUS_CONNECTED
        master_account: "MASTER_TEST_001".to_string(),
        slave_account: "SLAVE_TEST_001".to_string(),
        lot_multiplier: Some(2.5),
        reverse_trade: true,
        symbol_prefix: None,
        symbol_suffix: None,
        symbol_mappings: vec![
            SymbolMapping {
                source_symbol: "EURUSD".to_string(),
                target_symbol: "EURUSDm".to_string(),
            },
            SymbolMapping {
                source_symbol: "GBPUSD".to_string(),
                target_symbol: "GBPUSDm".to_string(),
            },
        ],
        filters: TradeFilters {
            allowed_symbols: Some(vec!["EURUSD".to_string(), "GBPUSD".to_string()]),
            blocked_symbols: Some(vec!["USDJPY".to_string()]),
            allowed_magic_numbers: Some(vec![123, 456, 789]),
            blocked_magic_numbers: Some(vec![999]),
        },
    };

    // Step 2: Save to database
    let settings_id = db
        .save_copy_settings(&test_settings)
        .await
        .expect("Failed to save test settings");

    assert!(settings_id > 0, "Settings ID should be positive");

    // Step 3: Retrieve from database
    let retrieved_settings = db
        .get_copy_settings(settings_id)
        .await
        .expect("Failed to retrieve settings")
        .expect("Settings should exist");

    // Verify retrieved settings match
    assert_eq!(retrieved_settings.master_account, "MASTER_TEST_001");
    assert_eq!(retrieved_settings.slave_account, "SLAVE_TEST_001");
    assert_eq!(retrieved_settings.lot_multiplier, Some(2.5));
    assert!(retrieved_settings.reverse_trade);
    assert_eq!(retrieved_settings.symbol_mappings.len(), 2);
    assert_eq!(
        retrieved_settings
            .filters
            .allowed_symbols
            .as_ref()
            .unwrap()
            .len(),
        2
    );

    // Step 4: Convert to ConfigMessage (simulating send_config_to_ea())
    let config_message: ConfigMessage = retrieved_settings.into();

    // Step 5: Verify ConfigMessage fields
    assert_eq!(config_message.account_id, "SLAVE_TEST_001");
    assert_eq!(config_message.master_account, "MASTER_TEST_001");
    assert_eq!(config_message.status, 2); // STATUS_CONNECTED
    assert_eq!(config_message.lot_multiplier, Some(2.5));
    assert!(config_message.reverse_trade);
    assert_eq!(config_message.symbol_mappings.len(), 2);
    assert_eq!(config_message.symbol_mappings[0].source_symbol, "EURUSD");
    assert_eq!(config_message.symbol_mappings[0].target_symbol, "EURUSDm");
    assert_eq!(config_message.config_version, 1);

    // Verify filters
    assert_eq!(
        config_message.filters.allowed_symbols.as_ref().unwrap(),
        &vec!["EURUSD".to_string(), "GBPUSD".to_string()]
    );
    assert_eq!(
        config_message.filters.blocked_symbols.as_ref().unwrap(),
        &vec!["USDJPY".to_string()]
    );
    assert_eq!(
        config_message
            .filters
            .allowed_magic_numbers
            .as_ref()
            .unwrap(),
        &vec![123, 456, 789]
    );
    assert_eq!(
        config_message
            .filters
            .blocked_magic_numbers
            .as_ref()
            .unwrap(),
        &vec![999]
    );

    // Step 6: Serialize to JSON (simulating ZMQ serialization)
    let json_string =
        serde_json::to_string(&config_message).expect("Failed to serialize ConfigMessage to JSON");

    println!(
        "Serialized ConfigMessage ({} bytes):\n{}",
        json_string.len(),
        json_string
    );

    // Step 7: Verify JSON contains all expected fields
    assert!(json_string.contains("\"account_id\""));
    assert!(json_string.contains("\"master_account\""));
    assert!(json_string.contains("\"status\""));
    assert!(json_string.contains("\"lot_multiplier\""));
    assert!(json_string.contains("\"reverse_trade\""));
    assert!(json_string.contains("\"symbol_mappings\""));
    assert!(json_string.contains("\"filters\""));
    assert!(json_string.contains("\"config_version\""));
    assert!(json_string.contains("\"timestamp\""));

    // Verify specific values in JSON
    assert!(json_string.contains("\"SLAVE_TEST_001\""));
    assert!(json_string.contains("\"MASTER_TEST_001\""));
    assert!(json_string.contains("2.5"));
    assert!(json_string.contains("\"EURUSD\""));
    assert!(json_string.contains("\"EURUSDm\""));

    // Step 8: Deserialize back to verify round-trip works
    let deserialized: ConfigMessage = serde_json::from_str(&json_string)
        .expect("Failed to deserialize JSON back to ConfigMessage");

    assert_eq!(deserialized.account_id, config_message.account_id);
    assert_eq!(deserialized.master_account, config_message.master_account);
    assert_eq!(deserialized.status, config_message.status);
    assert_eq!(deserialized.lot_multiplier, config_message.lot_multiplier);
    assert_eq!(deserialized.reverse_trade, config_message.reverse_trade);
    assert_eq!(deserialized.config_version, config_message.config_version);

    println!("✓ Integration test passed: CONFIG distribution workflow verified");
}

/// Test the get_settings_for_slave() method added in Phase 1 Task 5
#[tokio::test]
async fn test_get_settings_for_slave_method() {
    // Setup: Create in-memory test database
    let db = Database::new("sqlite::memory:")
        .await
        .expect("Failed to create test database");

    // Create multiple settings with different slave accounts
    let settings1 = CopySettings {
        id: 0,
        status: 2, // STATUS_CONNECTED
        master_account: "MASTER_001".to_string(),
        slave_account: "SLAVE_ACTIVE".to_string(),
        lot_multiplier: Some(1.5),
        reverse_trade: false,
        symbol_prefix: None,
        symbol_suffix: None,
        symbol_mappings: vec![],
        filters: TradeFilters {
            allowed_symbols: None,
            blocked_symbols: None,
            allowed_magic_numbers: None,
            blocked_magic_numbers: None,
        },
    };

    let settings2 = CopySettings {
        id: 0,
        status: 0, // STATUS_DISABLED
        master_account: "MASTER_002".to_string(),
        slave_account: "SLAVE_DISABLED".to_string(),
        lot_multiplier: Some(2.0),
        reverse_trade: false,
        symbol_prefix: None,
        symbol_suffix: None,
        symbol_mappings: vec![],
        filters: TradeFilters {
            allowed_symbols: None,
            blocked_symbols: None,
            allowed_magic_numbers: None,
            blocked_magic_numbers: None,
        },
    };

    // Save both settings
    db.save_copy_settings(&settings1)
        .await
        .expect("Failed to save settings1");
    db.save_copy_settings(&settings2)
        .await
        .expect("Failed to save settings2");

    // Test 1: Query for enabled slave - should find it
    let result = db
        .get_settings_for_slave("SLAVE_ACTIVE")
        .await
        .expect("Query failed");

    assert!(!result.is_empty(), "Should find active slave settings");
    let found_settings = &result[0];
    assert_eq!(found_settings.slave_account, "SLAVE_ACTIVE");
    assert_eq!(found_settings.master_account, "MASTER_001");
    assert_eq!(found_settings.status, 2); // STATUS_CONNECTED

    // Test 2: Query for disabled slave - should NOT find it (status filter)
    let result = db
        .get_settings_for_slave("SLAVE_DISABLED")
        .await
        .expect("Query failed");

    assert!(result.is_empty(), "Should not find disabled slave settings");

    // Test 3: Query for non-existent slave - should return None
    let result = db
        .get_settings_for_slave("SLAVE_NONEXISTENT")
        .await
        .expect("Query failed");

    assert!(result.is_empty(), "Should not find non-existent slave");

    println!("✓ Integration test passed: get_settings_for_slave() method verified");
}

/// Test ConfigMessage serialization with null/None values
#[tokio::test]
async fn test_config_message_with_null_values() {
    let db = Database::new("sqlite::memory:")
        .await
        .expect("Failed to create test database");

    // Create settings with minimal configuration (no lot multiplier, no mappings, no filters)
    let minimal_settings = CopySettings {
        id: 0,
        status: 2, // STATUS_CONNECTED
        master_account: "MASTER_MIN".to_string(),
        slave_account: "SLAVE_MIN".to_string(),
        lot_multiplier: None, // null
        reverse_trade: false,
        symbol_prefix: None,
        symbol_suffix: None,
        symbol_mappings: vec![], // empty
        filters: TradeFilters {
            allowed_symbols: None,
            blocked_symbols: None,
            allowed_magic_numbers: None,
            blocked_magic_numbers: None,
        },
    };

    let id = db
        .save_copy_settings(&minimal_settings)
        .await
        .expect("Failed to save minimal settings");

    let retrieved = db
        .get_copy_settings(id)
        .await
        .expect("Failed to retrieve")
        .expect("Should exist");

    // Convert to ConfigMessage
    let config: ConfigMessage = retrieved.into();

    // Serialize
    let json = serde_json::to_string(&config).expect("Failed to serialize");

    // Verify null is properly serialized
    assert!(json.contains("\"lot_multiplier\":null"));
    assert!(json.contains("\"allowed_symbols\":null"));
    assert!(json.contains("\"blocked_symbols\":null"));
    assert!(json.contains("\"allowed_magic_numbers\":null"));
    assert!(json.contains("\"blocked_magic_numbers\":null"));

    // Verify empty arrays
    assert!(json.contains("\"symbol_mappings\":[]"));

    // Deserialize and verify
    let deserialized: ConfigMessage = serde_json::from_str(&json).expect("Failed to deserialize");
    assert_eq!(deserialized.lot_multiplier, None);
    assert_eq!(deserialized.symbol_mappings.len(), 0);
    assert!(deserialized.filters.allowed_symbols.is_none());

    println!("✓ Integration test passed: Null value handling verified");
}
