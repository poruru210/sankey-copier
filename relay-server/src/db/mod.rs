//! Database module - Core database initialization and table management
//!
//! This module provides the main Database struct and initialization logic,
//! with CRUD operations split into separate submodules.

use anyhow::Result;
use sqlx::sqlite::SqlitePool;

// Submodule declarations
mod config_distribution;
mod trade_group_members;
mod trade_groups;

// Re-export all public items

// Test module
#[cfg(test)]
mod tests;

pub struct Database {
    pool: SqlitePool,
}

#[allow(dead_code)]
impl Database {
    /// Get a reference to the underlying connection pool
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = SqlitePool::connect(database_url).await?;

        // Drop old connections table (clean migration, no data preservation)
        sqlx::query("DROP TABLE IF EXISTS connections")
            .execute(&pool)
            .await?;

        // Create trade_groups table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS trade_groups (
                id TEXT PRIMARY KEY,
                master_settings TEXT NOT NULL DEFAULT '{}',
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await?;

        // Create trade_group_members table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS trade_group_members (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                trade_group_id TEXT NOT NULL,
                slave_account TEXT NOT NULL,
                slave_settings TEXT NOT NULL DEFAULT '{}',
                status INTEGER NOT NULL DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE (trade_group_id, slave_account),
                FOREIGN KEY (trade_group_id) REFERENCES trade_groups(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&pool)
        .await?;

        // Create indexes for performance
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_trade_group_members_slave
             ON trade_group_members(slave_account)",
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_trade_group_members_status
             ON trade_group_members(status)",
        )
        .execute(&pool)
        .await?;

        Ok(Self { pool })
    }
}
