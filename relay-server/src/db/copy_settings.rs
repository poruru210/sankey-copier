//! Old schema methods for CopySettings (deprecated)
//!
//! Implementation of Database methods for backward compatibility with existing tests.
//! These methods map the old CopySettings API to the new trade_group_members table.
//! Will be removed in future versions.

use crate::models::{CopySettings, SlaveSettings};
use anyhow::Result;
use sqlx::Row;

use super::Database;

impl Database {
    // ============================================================================
    // Old schema methods (deprecated - will be removed)
    // These methods are kept temporarily for backward compatibility with existing tests
    // ============================================================================

    pub async fn get_copy_settings(&self, id: i32) -> Result<Option<CopySettings>> {
        let row = sqlx::query(
            "SELECT id, trade_group_id, slave_account, slave_settings, status
             FROM trade_group_members WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let id: i32 = row.get("id");
            let master_account: String = row.get("trade_group_id");
            let slave_account: String = row.get("slave_account");
            let status: i32 = row.get("status");
            let settings_json: String = row.get("slave_settings");
            let slave_settings: SlaveSettings = serde_json::from_str(&settings_json)?;

            Ok(Some(CopySettings {
                id,
                status,
                master_account,
                slave_account,
                lot_multiplier: slave_settings.lot_multiplier,
                reverse_trade: slave_settings.reverse_trade,
                symbol_prefix: slave_settings.symbol_prefix,
                symbol_suffix: slave_settings.symbol_suffix,
                symbol_mappings: slave_settings.symbol_mappings,
                filters: slave_settings.filters,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn list_copy_settings(&self) -> Result<Vec<CopySettings>> {
        let rows = sqlx::query(
            "SELECT id, trade_group_id, slave_account, slave_settings, status
             FROM trade_group_members ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut result = Vec::new();
        for row in rows {
            let id: i32 = row.get("id");
            let master_account: String = row.get("trade_group_id");
            let slave_account: String = row.get("slave_account");
            let status: i32 = row.get("status");
            let settings_json: String = row.get("slave_settings");
            let slave_settings: SlaveSettings = serde_json::from_str(&settings_json)?;

            result.push(CopySettings {
                id,
                status,
                master_account,
                slave_account,
                lot_multiplier: slave_settings.lot_multiplier,
                reverse_trade: slave_settings.reverse_trade,
                symbol_prefix: slave_settings.symbol_prefix,
                symbol_suffix: slave_settings.symbol_suffix,
                symbol_mappings: slave_settings.symbol_mappings,
                filters: slave_settings.filters,
            });
        }

        Ok(result)
    }

    pub async fn save_copy_settings(&self, settings: &CopySettings) -> Result<i32> {
        // Ensure trade_group exists for this master
        let trade_group_exists = sqlx::query("SELECT 1 FROM trade_groups WHERE id = ?")
            .bind(&settings.master_account)
            .fetch_optional(&self.pool)
            .await?
            .is_some();

        if !trade_group_exists {
            // Create trade_group with default master settings
            sqlx::query("INSERT INTO trade_groups (id, master_settings) VALUES (?, '{}')")
                .bind(&settings.master_account)
                .execute(&self.pool)
                .await?;
        }

        // Convert CopySettings to SlaveSettings
        let slave_settings = SlaveSettings {
            lot_multiplier: settings.lot_multiplier,
            reverse_trade: settings.reverse_trade,
            symbol_prefix: settings.symbol_prefix.clone(),
            symbol_suffix: settings.symbol_suffix.clone(),
            symbol_mappings: settings.symbol_mappings.clone(),
            filters: settings.filters.clone(),
            config_version: 1,
        };
        let settings_json = serde_json::to_string(&slave_settings)?;

        let id = if settings.id == 0 {
            // New record - INSERT
            let result = sqlx::query(
                "INSERT INTO trade_group_members (trade_group_id, slave_account, slave_settings, status)
                 VALUES (?, ?, ?, ?)",
            )
            .bind(&settings.master_account)
            .bind(&settings.slave_account)
            .bind(&settings_json)
            .bind(settings.status)
            .execute(&self.pool)
            .await?;

            result.last_insert_rowid() as i32
        } else {
            // Existing record - UPDATE
            sqlx::query(
                "UPDATE trade_group_members SET
                    trade_group_id = ?,
                    slave_account = ?,
                    slave_settings = ?,
                    status = ?,
                    updated_at = CURRENT_TIMESTAMP
                 WHERE id = ?",
            )
            .bind(&settings.master_account)
            .bind(&settings.slave_account)
            .bind(&settings_json)
            .bind(settings.status)
            .bind(settings.id)
            .execute(&self.pool)
            .await?;

            settings.id
        };

        Ok(id)
    }

    pub async fn update_status(&self, id: i32, status: i32) -> Result<()> {
        sqlx::query(
            "UPDATE trade_group_members SET status = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(status)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete_copy_settings(&self, id: i32) -> Result<()> {
        sqlx::query("DELETE FROM trade_group_members WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
