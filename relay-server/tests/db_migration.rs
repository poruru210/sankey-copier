// relay-server/tests/db_migration.rs
//
// Tests for database migration to the new trade_groups schema.
// Validates that the new tables and indexes are created correctly.

use sankey_copier_relay_server::adapters::outbound::persistence::Database;

/// Helper to create an in-memory test database
async fn create_test_db() -> Database {
    Database::new("sqlite::memory:").await.unwrap()
}

#[tokio::test]
async fn test_migration_creates_trade_groups_table() {
    let db = create_test_db().await;

    // Query sqlite_master to check if trade_groups table exists
    let result =
        sqlx::query("SELECT name FROM sqlite_master WHERE type='table' AND name='trade_groups'")
            .fetch_optional(db.pool())
            .await
            .unwrap();

    assert!(result.is_some(), "trade_groups table should exist");
}

#[tokio::test]
async fn test_migration_creates_trade_group_members_table() {
    let db = create_test_db().await;

    // Query sqlite_master to check if trade_group_members table exists
    let result = sqlx::query(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='trade_group_members'",
    )
    .fetch_optional(db.pool())
    .await
    .unwrap();

    assert!(result.is_some(), "trade_group_members table should exist");
}

#[tokio::test]
async fn test_migration_creates_indexes() {
    let db = create_test_db().await;

    // Check for slave_account index
    let result1 = sqlx::query(
        "SELECT name FROM sqlite_master WHERE type='index' AND name='idx_trade_group_members_slave'"
    )
    .fetch_optional(db.pool())
    .await
    .unwrap();

    assert!(
        result1.is_some(),
        "idx_trade_group_members_slave index should exist"
    );

    // Check for status index
    let result2 = sqlx::query(
        "SELECT name FROM sqlite_master WHERE type='index' AND name='idx_trade_group_members_status'"
    )
    .fetch_optional(db.pool())
    .await
    .unwrap();

    assert!(
        result2.is_some(),
        "idx_trade_group_members_status index should exist"
    );
}

#[tokio::test]
async fn test_migration_drops_old_connections_table() {
    let db = create_test_db().await;

    // Query sqlite_master to check if connections table does NOT exist
    let result =
        sqlx::query("SELECT name FROM sqlite_master WHERE type='table' AND name='connections'")
            .fetch_optional(db.pool())
            .await
            .unwrap();

    assert!(
        result.is_none(),
        "connections table should not exist after migration"
    );
}

#[tokio::test]
async fn test_trade_groups_table_schema() {
    let db = create_test_db().await;

    // Validate table schema by attempting to query with expected columns
    let result =
        sqlx::query("SELECT id, master_settings, created_at, updated_at FROM trade_groups LIMIT 0")
            .fetch_optional(db.pool())
            .await;

    assert!(
        result.is_ok(),
        "trade_groups table should have expected columns"
    );
}

#[tokio::test]
async fn test_trade_group_members_table_schema() {
    let db = create_test_db().await;

    // Validate table schema by attempting to query with expected columns
    let result = sqlx::query(
        "SELECT trade_group_id, slave_account, slave_settings, status, created_at, updated_at
         FROM trade_group_members LIMIT 0",
    )
    .fetch_optional(db.pool())
    .await;

    assert!(
        result.is_ok(),
        "trade_group_members table should have expected columns"
    );
}

#[tokio::test]
async fn test_foreign_key_constraint() {
    let db = create_test_db().await;

    // Enable foreign key enforcement (disabled by default in SQLite)
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(db.pool())
        .await
        .unwrap();

    // Try to insert a member without a corresponding trade_group
    let result = sqlx::query(
        "INSERT INTO trade_group_members (trade_group_id, slave_account, slave_settings, status)
         VALUES ('NONEXISTENT_MASTER', 'SLAVE_001', '{}', 1)",
    )
    .execute(db.pool())
    .await;

    // Should fail due to foreign key constraint
    assert!(
        result.is_err(),
        "Foreign key constraint should prevent insertion of orphan member"
    );
}

#[tokio::test]
async fn test_cascade_delete() {
    let db = create_test_db().await;

    // Enable foreign key enforcement
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(db.pool())
        .await
        .unwrap();

    // Create a trade_group
    sqlx::query("INSERT INTO trade_groups (id, master_settings) VALUES ('MASTER_001', '{}')")
        .execute(db.pool())
        .await
        .unwrap();

    // Create a member
    sqlx::query(
        "INSERT INTO trade_group_members (trade_group_id, slave_account, slave_settings, status)
         VALUES ('MASTER_001', 'SLAVE_001', '{}', 1)",
    )
    .execute(db.pool())
    .await
    .unwrap();

    // Delete the trade_group
    sqlx::query("DELETE FROM trade_groups WHERE id = 'MASTER_001'")
        .execute(db.pool())
        .await
        .unwrap();

    // Check that the member was also deleted (cascade)
    let result =
        sqlx::query("SELECT * FROM trade_group_members WHERE trade_group_id = 'MASTER_001'")
            .fetch_optional(db.pool())
            .await
            .unwrap();

    assert!(
        result.is_none(),
        "CASCADE DELETE should remove associated members"
    );
}
