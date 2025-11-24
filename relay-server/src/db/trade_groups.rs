//! TradeGroup CRUD operations
//!
//! Implementation of Database methods for managing TradeGroups,
//! which represent Master EA accounts and their settings.

use crate::models::{MasterSettings, TradeGroup};
use anyhow::{anyhow, Result};
use sqlx::Row;

use super::Database;

impl Database {
    // ============================================================================
    // TradeGroup CRUD Operations
    // ============================================================================

    /// Create a new TradeGroup with default settings
    pub async fn create_trade_group(&self, master_account: &str) -> Result<TradeGroup> {
        let master_settings = MasterSettings::default();
        let settings_json = serde_json::to_string(&master_settings)?;

        sqlx::query("INSERT INTO trade_groups (id, master_settings) VALUES (?, ?)")
            .bind(master_account)
            .bind(&settings_json)
            .execute(&self.pool)
            .await?;

        self.get_trade_group(master_account)
            .await?
            .ok_or_else(|| anyhow!("Failed to retrieve created trade_group"))
    }

    /// Get a TradeGroup by master_account
    pub async fn get_trade_group(&self, master_account: &str) -> Result<Option<TradeGroup>> {
        let row = sqlx::query(
            "SELECT id, master_settings, created_at, updated_at
             FROM trade_groups WHERE id = ?",
        )
        .bind(master_account)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let id: String = row.get("id");
            let settings_json: String = row.get("master_settings");
            let master_settings: MasterSettings = serde_json::from_str(&settings_json)?;
            let created_at: String = row.get("created_at");
            let updated_at: String = row.get("updated_at");

            Ok(Some(TradeGroup {
                id,
                master_settings,
                created_at,
                updated_at,
            }))
        } else {
            Ok(None)
        }
    }

    /// Update Master settings for a TradeGroup
    pub async fn update_master_settings(
        &self,
        master_account: &str,
        settings: MasterSettings,
    ) -> Result<()> {
        let settings_json = serde_json::to_string(&settings)?;

        sqlx::query(
            "UPDATE trade_groups
             SET master_settings = ?, updated_at = CURRENT_TIMESTAMP
             WHERE id = ?",
        )
        .bind(&settings_json)
        .bind(master_account)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete a TradeGroup (CASCADE deletes all members)
    pub async fn delete_trade_group(&self, master_account: &str) -> Result<()> {
        sqlx::query("DELETE FROM trade_groups WHERE id = ?")
            .bind(master_account)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// List all TradeGroups
    pub async fn list_trade_groups(&self) -> Result<Vec<TradeGroup>> {
        let rows = sqlx::query(
            "SELECT id, master_settings, created_at, updated_at
             FROM trade_groups ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut result = Vec::new();
        for row in rows {
            let id: String = row.get("id");
            let settings_json: String = row.get("master_settings");
            let master_settings: MasterSettings = serde_json::from_str(&settings_json)?;
            let created_at: String = row.get("created_at");
            let updated_at: String = row.get("updated_at");

            result.push(TradeGroup {
                id,
                master_settings,
                created_at,
                updated_at,
            });
        }

        Ok(result)
    }
}
