// relay-server/tests/db_trade_groups.rs
//
// Tests for TradeGroup and TradeGroupMember database operations.
// Following TDD principles: write tests first, then implement.

use sankey_copier_relay_server::db::Database;
use sankey_copier_relay_server::models::{
    MasterSettings, SlaveSettings, TradeGroup, TradeGroupMember,
};
use sankey_copier_zmq::{SymbolMapping, TradeFilters};

/// Helper to create an in-memory test database
async fn create_test_db() -> Database {
    Database::new("sqlite::memory:").await.unwrap()
}

// ============================================================================
// TradeGroup CRUD Operations Tests
// ============================================================================

#[tokio::test]
async fn test_create_trade_group() {
    let db = create_test_db().await;

    let result = db.create_trade_group("MASTER_001").await;

    assert!(result.is_ok());
    let trade_group = result.unwrap();
    assert_eq!(trade_group.id, "MASTER_001");
    assert_eq!(trade_group.master_settings.config_version, 0);
}

#[tokio::test]
async fn test_create_duplicate_trade_group() {
    let db = create_test_db().await;

    db.create_trade_group("MASTER_001").await.unwrap();
    let result = db.create_trade_group("MASTER_001").await;

    // Should fail due to PRIMARY KEY constraint
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_trade_group() {
    let db = create_test_db().await;

    db.create_trade_group("MASTER_001").await.unwrap();
    let result = db.get_trade_group("MASTER_001").await.unwrap();

    assert!(result.is_some());
    let trade_group = result.unwrap();
    assert_eq!(trade_group.id, "MASTER_001");
}

#[tokio::test]
async fn test_get_nonexistent_trade_group() {
    let db = create_test_db().await;

    let result = db.get_trade_group("NONEXISTENT").await.unwrap();

    assert!(result.is_none());
}

#[tokio::test]
async fn test_update_master_settings() {
    let db = create_test_db().await;

    db.create_trade_group("MASTER_001").await.unwrap();

    let new_settings = MasterSettings {
        symbol_prefix: Some("pro.".to_string()),
        symbol_suffix: Some(".m".to_string()),
        config_version: 1,
    };

    db.update_master_settings("MASTER_001", new_settings.clone())
        .await
        .unwrap();

    let trade_group = db.get_trade_group("MASTER_001").await.unwrap().unwrap();
    assert_eq!(
        trade_group.master_settings.symbol_prefix,
        Some("pro.".to_string())
    );
    assert_eq!(
        trade_group.master_settings.symbol_suffix,
        Some(".m".to_string())
    );
    assert_eq!(trade_group.master_settings.config_version, 1);
}

#[tokio::test]
async fn test_delete_trade_group() {
    let db = create_test_db().await;

    db.create_trade_group("MASTER_001").await.unwrap();
    db.delete_trade_group("MASTER_001").await.unwrap();

    let result = db.get_trade_group("MASTER_001").await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_delete_nonexistent_trade_group() {
    let db = create_test_db().await;

    let result = db.delete_trade_group("NONEXISTENT").await;
    assert!(result.is_ok()); // Should not error
}

// ============================================================================
// TradeGroupMember CRUD Operations Tests
// ============================================================================

#[tokio::test]
async fn test_add_member() {
    let db = create_test_db().await;

    db.create_trade_group("MASTER_001").await.unwrap();

    let settings = SlaveSettings {
        lot_multiplier: Some(1.5),
        reverse_trade: false,
        symbol_prefix: None,
        symbol_suffix: None,
        symbol_mappings: vec![],
        filters: TradeFilters::default(),
        config_version: 0,
    };

    let result = db.add_member("MASTER_001", "SLAVE_001", settings).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_add_member_without_trade_group() {
    let db = create_test_db().await;

    let settings = SlaveSettings::default();
    let result = db
        .add_member("NONEXISTENT_MASTER", "SLAVE_001", settings)
        .await;

    // Should fail due to foreign key constraint
    assert!(result.is_err());
}

#[tokio::test]
async fn test_add_duplicate_member() {
    let db = create_test_db().await;

    db.create_trade_group("MASTER_001").await.unwrap();
    let settings = SlaveSettings::default();

    db.add_member("MASTER_001", "SLAVE_001", settings.clone())
        .await
        .unwrap();
    let result = db.add_member("MASTER_001", "SLAVE_001", settings).await;

    // Should fail due to PRIMARY KEY constraint
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_members() {
    let db = create_test_db().await;

    db.create_trade_group("MASTER_001").await.unwrap();

    db.add_member("MASTER_001", "SLAVE_001", SlaveSettings::default())
        .await
        .unwrap();
    db.add_member("MASTER_001", "SLAVE_002", SlaveSettings::default())
        .await
        .unwrap();

    let members = db.get_members("MASTER_001").await.unwrap();

    assert_eq!(members.len(), 2);
    assert_eq!(members[0].slave_account, "SLAVE_001");
    assert_eq!(members[1].slave_account, "SLAVE_002");
}

#[tokio::test]
async fn test_get_members_empty() {
    let db = create_test_db().await;

    db.create_trade_group("MASTER_001").await.unwrap();

    let members = db.get_members("MASTER_001").await.unwrap();

    assert!(members.is_empty());
}

#[tokio::test]
async fn test_get_member() {
    let db = create_test_db().await;

    db.create_trade_group("MASTER_001").await.unwrap();
    db.add_member("MASTER_001", "SLAVE_001", SlaveSettings::default())
        .await
        .unwrap();

    let result = db.get_member("MASTER_001", "SLAVE_001").await.unwrap();

    assert!(result.is_some());
    let member = result.unwrap();
    assert_eq!(member.slave_account, "SLAVE_001");
}

#[tokio::test]
async fn test_get_nonexistent_member() {
    let db = create_test_db().await;

    db.create_trade_group("MASTER_001").await.unwrap();

    let result = db.get_member("MASTER_001", "NONEXISTENT").await.unwrap();

    assert!(result.is_none());
}

#[tokio::test]
async fn test_update_member_settings() {
    let db = create_test_db().await;

    db.create_trade_group("MASTER_001").await.unwrap();
    db.add_member("MASTER_001", "SLAVE_001", SlaveSettings::default())
        .await
        .unwrap();

    let new_settings = SlaveSettings {
        lot_multiplier: Some(2.0),
        reverse_trade: true,
        symbol_prefix: None,
        symbol_suffix: None,
        symbol_mappings: vec![],
        filters: TradeFilters::default(),
        config_version: 1,
    };

    db.update_member_settings("MASTER_001", "SLAVE_001", new_settings)
        .await
        .unwrap();

    let member = db
        .get_member("MASTER_001", "SLAVE_001")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(member.slave_settings.lot_multiplier, Some(2.0));
    assert!(member.slave_settings.reverse_trade);
    assert_eq!(member.slave_settings.config_version, 1);
}

#[tokio::test]
async fn test_update_member_status() {
    let db = create_test_db().await;

    db.create_trade_group("MASTER_001").await.unwrap();
    db.add_member("MASTER_001", "SLAVE_001", SlaveSettings::default())
        .await
        .unwrap();

    db.update_member_status("MASTER_001", "SLAVE_001", 2)
        .await
        .unwrap();

    let member = db
        .get_member("MASTER_001", "SLAVE_001")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(member.status, 2);
}

#[tokio::test]
async fn test_delete_member() {
    let db = create_test_db().await;

    db.create_trade_group("MASTER_001").await.unwrap();
    db.add_member("MASTER_001", "SLAVE_001", SlaveSettings::default())
        .await
        .unwrap();

    db.delete_member("MASTER_001", "SLAVE_001").await.unwrap();

    let result = db.get_member("MASTER_001", "SLAVE_001").await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_cascade_delete_members() {
    let db = create_test_db().await;

    db.create_trade_group("MASTER_001").await.unwrap();
    db.add_member("MASTER_001", "SLAVE_001", SlaveSettings::default())
        .await
        .unwrap();
    db.add_member("MASTER_001", "SLAVE_002", SlaveSettings::default())
        .await
        .unwrap();

    db.delete_trade_group("MASTER_001").await.unwrap();

    let members = db.get_members("MASTER_001").await.unwrap();
    assert!(members.is_empty());
}

// ============================================================================
// Config Distribution Tests
// ============================================================================

#[tokio::test]
async fn test_get_settings_for_master() {
    let db = create_test_db().await;

    let settings = MasterSettings {
        symbol_prefix: Some("pro.".to_string()),
        symbol_suffix: Some(".m".to_string()),
        config_version: 1,
    };

    db.create_trade_group("MASTER_001").await.unwrap();
    db.update_master_settings("MASTER_001", settings.clone())
        .await
        .unwrap();

    let result = db.get_settings_for_master("MASTER_001").await.unwrap();

    assert_eq!(result.symbol_prefix, Some("pro.".to_string()));
    assert_eq!(result.symbol_suffix, Some(".m".to_string()));
}

#[tokio::test]
async fn test_get_settings_for_nonexistent_master() {
    let db = create_test_db().await;

    let result = db.get_settings_for_master("NONEXISTENT").await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_settings_for_slave() {
    let db = create_test_db().await;

    // Setup: Create two masters with the same slave
    db.create_trade_group("MASTER_001").await.unwrap();
    db.create_trade_group("MASTER_002").await.unwrap();

    let settings1 = SlaveSettings {
        lot_multiplier: Some(1.0),
        reverse_trade: false,
        symbol_prefix: None,
        symbol_suffix: None,
        symbol_mappings: vec![],
        filters: TradeFilters::default(),
        config_version: 0,
    };

    let settings2 = SlaveSettings {
        lot_multiplier: Some(2.0),
        reverse_trade: true,
        symbol_prefix: None,
        symbol_suffix: None,
        symbol_mappings: vec![],
        filters: TradeFilters::default(),
        config_version: 0,
    };

    db.add_member("MASTER_001", "SLAVE_001", settings1)
        .await
        .unwrap();
    db.add_member("MASTER_002", "SLAVE_001", settings2)
        .await
        .unwrap();

    let result = db.get_settings_for_slave("SLAVE_001").await.unwrap();

    assert_eq!(result.len(), 2);
    assert_eq!(result[0].master_account, "MASTER_001");
    assert_eq!(result[1].master_account, "MASTER_002");
}

#[tokio::test]
async fn test_get_settings_for_slave_only_enabled() {
    let db = create_test_db().await;

    db.create_trade_group("MASTER_001").await.unwrap();
    db.add_member("MASTER_001", "SLAVE_001", SlaveSettings::default())
        .await
        .unwrap();

    // Set status to DISABLED (0)
    db.update_member_status("MASTER_001", "SLAVE_001", 0)
        .await
        .unwrap();

    let result = db.get_settings_for_slave("SLAVE_001").await.unwrap();

    // Should return empty because status is DISABLED
    assert!(result.is_empty());
}

#[tokio::test]
async fn test_update_master_statuses_connected() {
    let db = create_test_db().await;

    db.create_trade_group("MASTER_001").await.unwrap();

    // Add members with different statuses
    db.add_member("MASTER_001", "SLAVE_001", SlaveSettings::default())
        .await
        .unwrap();
    db.add_member("MASTER_001", "SLAVE_002", SlaveSettings::default())
        .await
        .unwrap();
    db.add_member("MASTER_001", "SLAVE_003", SlaveSettings::default())
        .await
        .unwrap();

    // Set statuses: DISABLED, ENABLED, ENABLED
    db.update_member_status("MASTER_001", "SLAVE_001", 0)
        .await
        .unwrap();
    db.update_member_status("MASTER_001", "SLAVE_002", 1)
        .await
        .unwrap();
    db.update_member_status("MASTER_001", "SLAVE_003", 1)
        .await
        .unwrap();

    // Update to CONNECTED
    let count = db
        .update_master_statuses_connected("MASTER_001")
        .await
        .unwrap();

    // Should update 2 members (ENABLED → CONNECTED)
    assert_eq!(count, 2);

    let member1 = db
        .get_member("MASTER_001", "SLAVE_001")
        .await
        .unwrap()
        .unwrap();
    let member2 = db
        .get_member("MASTER_001", "SLAVE_002")
        .await
        .unwrap()
        .unwrap();
    let member3 = db
        .get_member("MASTER_001", "SLAVE_003")
        .await
        .unwrap()
        .unwrap();

    assert_eq!(member1.status, 0); // Still DISABLED
    assert_eq!(member2.status, 2); // Now CONNECTED
    assert_eq!(member3.status, 2); // Now CONNECTED
}

#[tokio::test]
async fn test_update_master_statuses_disconnected() {
    let db = create_test_db().await;

    db.create_trade_group("MASTER_001").await.unwrap();

    // Add members with different statuses
    db.add_member("MASTER_001", "SLAVE_001", SlaveSettings::default())
        .await
        .unwrap();
    db.add_member("MASTER_001", "SLAVE_002", SlaveSettings::default())
        .await
        .unwrap();
    db.add_member("MASTER_001", "SLAVE_003", SlaveSettings::default())
        .await
        .unwrap();

    // Set statuses: DISABLED, ENABLED, CONNECTED
    db.update_member_status("MASTER_001", "SLAVE_001", 0)
        .await
        .unwrap();
    db.update_member_status("MASTER_001", "SLAVE_002", 1)
        .await
        .unwrap();
    db.update_member_status("MASTER_001", "SLAVE_003", 2)
        .await
        .unwrap();

    // Update to ENABLED (disconnected)
    let count = db
        .update_master_statuses_disconnected("MASTER_001")
        .await
        .unwrap();

    // Should update 1 member (CONNECTED → ENABLED)
    assert_eq!(count, 1);

    let member1 = db
        .get_member("MASTER_001", "SLAVE_001")
        .await
        .unwrap()
        .unwrap();
    let member2 = db
        .get_member("MASTER_001", "SLAVE_002")
        .await
        .unwrap()
        .unwrap();
    let member3 = db
        .get_member("MASTER_001", "SLAVE_003")
        .await
        .unwrap()
        .unwrap();

    assert_eq!(member1.status, 0); // Still DISABLED
    assert_eq!(member2.status, 1); // Still ENABLED
    assert_eq!(member3.status, 1); // Now ENABLED (was CONNECTED)
}

// ============================================================================
// Symbol Mappings and Filters Tests
// ============================================================================

#[tokio::test]
async fn test_member_with_symbol_mappings() {
    let db = create_test_db().await;

    db.create_trade_group("MASTER_001").await.unwrap();

    let settings = SlaveSettings {
        lot_multiplier: Some(1.0),
        reverse_trade: false,
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
        filters: TradeFilters::default(),
        config_version: 0,
    };

    db.add_member("MASTER_001", "SLAVE_001", settings)
        .await
        .unwrap();

    let member = db
        .get_member("MASTER_001", "SLAVE_001")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(member.slave_settings.symbol_mappings.len(), 2);
    assert_eq!(
        member.slave_settings.symbol_mappings[0].source_symbol,
        "EURUSD"
    );
}

#[tokio::test]
async fn test_member_with_filters() {
    let db = create_test_db().await;

    db.create_trade_group("MASTER_001").await.unwrap();

    let settings = SlaveSettings {
        lot_multiplier: Some(1.0),
        reverse_trade: false,
        symbol_prefix: None,
        symbol_suffix: None,
        symbol_mappings: vec![],
        filters: TradeFilters {
            allowed_symbols: Some(vec!["EURUSD".to_string(), "GBPUSD".to_string()]),
            blocked_symbols: Some(vec!["USDJPY".to_string()]),
            allowed_magic_numbers: Some(vec![100, 200]),
            blocked_magic_numbers: Some(vec![999]),
        },
        config_version: 0,
    };

    db.add_member("MASTER_001", "SLAVE_001", settings)
        .await
        .unwrap();

    let member = db
        .get_member("MASTER_001", "SLAVE_001")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        member.slave_settings.filters.allowed_symbols,
        Some(vec!["EURUSD".to_string(), "GBPUSD".to_string()])
    );
    assert_eq!(
        member.slave_settings.filters.blocked_magic_numbers,
        Some(vec![999])
    );
}
