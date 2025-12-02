//! Database module - Core database initialization and table management
//!
//! This module provides the main Database struct and initialization logic,
//! with CRUD operations split into separate submodules.

use anyhow::Result;
use sqlx::{sqlite::SqlitePool, Row};

// Submodule declarations
mod config_distribution;
mod global_settings;
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
                enabled_flag INTEGER NOT NULL DEFAULT 0,
                runtime_status INTEGER NOT NULL DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE (trade_group_id, slave_account),
                FOREIGN KEY (trade_group_id) REFERENCES trade_groups(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&pool)
        .await?;

        let enabled_added = Self::ensure_column(
            &pool,
            "trade_group_members",
            "enabled_flag",
            "INTEGER NOT NULL DEFAULT 0",
        )
        .await?;

        if enabled_added {
            sqlx::query(
                "UPDATE trade_group_members
                 SET enabled_flag = CASE WHEN status > 0 THEN 1 ELSE 0 END",
            )
            .execute(&pool)
            .await?;
        }

        let runtime_added = Self::ensure_column(
            &pool,
            "trade_group_members",
            "runtime_status",
            "INTEGER NOT NULL DEFAULT 0",
        )
        .await?;

        if runtime_added {
            sqlx::query(
                "UPDATE trade_group_members
                 SET runtime_status = status",
            )
            .execute(&pool)
            .await?;
        }

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

        // Create global_settings table for system-wide configuration
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS global_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await?;

        Ok(Self { pool })
    }
}

impl Database {
    async fn ensure_column(
        pool: &SqlitePool,
        table: &str,
        column: &str,
        definition: &str,
    ) -> Result<bool> {
        if Self::column_exists(pool, table, column).await? {
            return Ok(false);
        }

        let sql = format!("ALTER TABLE {} ADD COLUMN {} {}", table, column, definition);
        sqlx::query(&sql).execute(pool).await?;
        Ok(true)
    }

    async fn column_exists(pool: &SqlitePool, table: &str, column: &str) -> Result<bool> {
        let pragma = format!("PRAGMA table_info({})", table);
        let rows = sqlx::query(&pragma).fetch_all(pool).await?;
        Ok(rows
            .iter()
            .any(|row| row.get::<String, _>("name") == column))
    }
}
