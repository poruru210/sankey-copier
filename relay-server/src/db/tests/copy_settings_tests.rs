//! Tests for CopySettings (old schema) operations
//!
//! Tests for backward compatibility with the deprecated CopySettings API

use super::*;
use sankey_copier_zmq::SymbolMapping;

#[tokio::test]
async fn test_get_nonexistent_copy_settings() {
    let db = create_test_db().await;

    // Try to get settings that don't exist
    let result = db.get_copy_settings(999).await.unwrap();

    assert!(result.is_none());
}

#[tokio::test]
async fn test_get_settings_for_nonexistent_slave() {
    let db = create_test_db().await;

    // Try to get settings for a slave that doesn't exist
    let result = db.get_settings_for_slave("NONEXISTENT").await.unwrap();

    assert!(result.is_empty());
}

#[tokio::test]
async fn test_list_copy_settings_empty() {
    let db = create_test_db().await;

    let settings = db.list_copy_settings().await.unwrap();

    assert_eq!(settings.len(), 0);
}

#[tokio::test]
async fn test_duplicate_master_slave_pair() {
    let db = create_test_db().await;

    let settings1 = create_test_settings();
    db.save_copy_settings(&settings1).await.unwrap();

    // Try to insert the same master-slave pair again
    let settings2 = create_test_settings();
    let result = db.save_copy_settings(&settings2).await;

    // Should fail due to UNIQUE constraint
    assert!(result.is_err());
}

#[tokio::test]
async fn test_save_and_retrieve_with_null_lot_multiplier() {
    let db = create_test_db().await;

    let mut settings = create_test_settings();
    settings.lot_multiplier = None;

    let id = db.save_copy_settings(&settings).await.unwrap();

    let retrieved = db.get_copy_settings(id).await.unwrap().unwrap();

    assert!(retrieved.lot_multiplier.is_none());
}

#[tokio::test]
async fn test_save_and_retrieve_with_symbol_mappings() {
    let db = create_test_db().await;

    let mut settings = create_test_settings();
    settings.symbol_mappings = vec![
        SymbolMapping {
            source_symbol: "EURUSD".to_string(),
            target_symbol: "EURUSDm".to_string(),
        },
        SymbolMapping {
            source_symbol: "GBPUSD".to_string(),
            target_symbol: "GBPUSDm".to_string(),
        },
    ];

    let id = db.save_copy_settings(&settings).await.unwrap();

    let retrieved = db.get_copy_settings(id).await.unwrap().unwrap();

    assert_eq!(retrieved.symbol_mappings.len(), 2);
    assert_eq!(retrieved.symbol_mappings[0].source_symbol, "EURUSD");
    assert_eq!(retrieved.symbol_mappings[0].target_symbol, "EURUSDm");
}

#[tokio::test]
async fn test_save_and_retrieve_with_filters() {
    let db = create_test_db().await;

    let mut settings = create_test_settings();
    settings.filters = sankey_copier_zmq::TradeFilters {
        allowed_symbols: Some(vec!["EURUSD".to_string(), "GBPUSD".to_string()]),
        blocked_symbols: Some(vec!["USDJPY".to_string()]),
        allowed_magic_numbers: Some(vec![100, 200]),
        blocked_magic_numbers: Some(vec![999]),
    };

    let id = db.save_copy_settings(&settings).await.unwrap();

    let retrieved = db.get_copy_settings(id).await.unwrap().unwrap();

    assert_eq!(
        retrieved.filters.allowed_symbols,
        Some(vec!["EURUSD".to_string(), "GBPUSD".to_string()])
    );
    assert_eq!(
        retrieved.filters.blocked_symbols,
        Some(vec!["USDJPY".to_string()])
    );
    assert_eq!(
        retrieved.filters.allowed_magic_numbers,
        Some(vec![100, 200])
    );
    assert_eq!(retrieved.filters.blocked_magic_numbers, Some(vec![999]));
}

#[tokio::test]
async fn test_update_existing_settings() {
    let db = create_test_db().await;

    // Create initial settings
    let settings = create_test_settings();
    let id = db.save_copy_settings(&settings).await.unwrap();

    // Update settings
    let mut updated_settings = db.get_copy_settings(id).await.unwrap().unwrap();
    updated_settings.lot_multiplier = Some(2.0);
    updated_settings.reverse_trade = true;

    db.save_copy_settings(&updated_settings).await.unwrap();

    // Retrieve and verify
    let retrieved = db.get_copy_settings(id).await.unwrap().unwrap();

    assert_eq!(retrieved.lot_multiplier, Some(2.0));
    assert!(retrieved.reverse_trade);
}

#[tokio::test]
async fn test_update_status() {
    let db = create_test_db().await;

    let settings = create_test_settings();
    let id = db.save_copy_settings(&settings).await.unwrap();

    // Set to DISABLED (0)
    db.update_status(id, 0).await.unwrap();

    let retrieved = db.get_copy_settings(id).await.unwrap().unwrap();
    assert_eq!(retrieved.status, 0);

    // Set to ENABLED (1)
    db.update_status(id, 1).await.unwrap();

    let retrieved = db.get_copy_settings(id).await.unwrap().unwrap();
    assert_eq!(retrieved.status, 1);

    // Set to CONNECTED (2)
    db.update_status(id, 2).await.unwrap();

    let retrieved = db.get_copy_settings(id).await.unwrap().unwrap();
    assert_eq!(retrieved.status, 2);
}

#[tokio::test]
async fn test_delete_copy_settings() {
    let db = create_test_db().await;

    let settings = create_test_settings();
    let id = db.save_copy_settings(&settings).await.unwrap();

    // Delete
    db.delete_copy_settings(id).await.unwrap();

    // Should no longer exist
    let retrieved = db.get_copy_settings(id).await.unwrap();
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_get_settings_for_slave_disabled() {
    let db = create_test_db().await;

    let mut settings = create_test_settings();
    settings.status = 0; // STATUS_DISABLED

    db.save_copy_settings(&settings).await.unwrap();

    // Should not return disabled settings
    let result = db.get_settings_for_slave("SLAVE_001").await.unwrap();
    assert!(result.is_empty());
}

#[tokio::test]
async fn test_get_settings_for_slave_enabled() {
    let db = create_test_db().await;

    let settings = create_test_settings();
    db.save_copy_settings(&settings).await.unwrap();

    // Should return enabled settings
    let result = db.get_settings_for_slave("SLAVE_001").await.unwrap();
    assert!(!result.is_empty());

    let retrieved = &result[0];
    assert_eq!(retrieved.slave_account, "SLAVE_001");
    assert_eq!(retrieved.master_account, "MASTER_001");
}

#[tokio::test]
async fn test_list_multiple_settings() {
    let db = create_test_db().await;

    // Create multiple settings
    for i in 1..=3 {
        let mut settings = create_test_settings();
        settings.master_account = format!("MASTER_{:03}", i);
        settings.slave_account = format!("SLAVE_{:03}", i);
        db.save_copy_settings(&settings).await.unwrap();
    }

    let all_settings = db.list_copy_settings().await.unwrap();

    assert_eq!(all_settings.len(), 3);
    assert_eq!(all_settings[0].master_account, "MASTER_001");
    assert_eq!(all_settings[1].master_account, "MASTER_002");
    assert_eq!(all_settings[2].master_account, "MASTER_003");
}

#[tokio::test]
async fn test_update_clears_old_mappings() {
    let db = create_test_db().await;

    // Create settings with mappings
    let mut settings = create_test_settings();
    settings.symbol_mappings = vec![
        SymbolMapping {
            source_symbol: "EURUSD".to_string(),
            target_symbol: "EURUSDm".to_string(),
        },
        SymbolMapping {
            source_symbol: "GBPUSD".to_string(),
            target_symbol: "GBPUSDm".to_string(),
        },
    ];

    let id = db.save_copy_settings(&settings).await.unwrap();

    // Update with different mappings
    let mut updated = db.get_copy_settings(id).await.unwrap().unwrap();
    updated.symbol_mappings = vec![SymbolMapping {
        source_symbol: "USDJPY".to_string(),
        target_symbol: "USDJPYm".to_string(),
    }];

    db.save_copy_settings(&updated).await.unwrap();

    // Retrieve and verify
    let retrieved = db.get_copy_settings(id).await.unwrap().unwrap();

    assert_eq!(retrieved.symbol_mappings.len(), 1);
    assert_eq!(retrieved.symbol_mappings[0].source_symbol, "USDJPY");
}

#[tokio::test]
async fn test_empty_filters_serialization() {
    let db = create_test_db().await;

    let settings = create_test_settings();
    let id = db.save_copy_settings(&settings).await.unwrap();

    let retrieved = db.get_copy_settings(id).await.unwrap().unwrap();

    // All filter fields should be None
    assert!(retrieved.filters.allowed_symbols.is_none());
    assert!(retrieved.filters.blocked_symbols.is_none());
    assert!(retrieved.filters.allowed_magic_numbers.is_none());
    assert!(retrieved.filters.blocked_magic_numbers.is_none());
}

#[tokio::test]
async fn test_delete_nonexistent_settings() {
    let db = create_test_db().await;

    // Deleting non-existent settings should not error
    let result = db.delete_copy_settings(999).await;
    assert!(result.is_ok());
}
