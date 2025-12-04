//! TradeGroupMember CRUD operations
//!
//! Implementation of Database methods for managing TradeGroupMembers,
//! which represent Slave EA accounts and their relationship to Master accounts.

use crate::models::{SlaveSettings, TradeGroupMember};
use anyhow::Result;
use sqlx::Row;

use super::Database;

impl Database {
    // ============================================================================
    // TradeGroupMember CRUD Operations
    // ============================================================================

    /// Add a member (Slave) to a TradeGroup
    ///
    /// # Arguments
    /// * `trade_group_id` - The master account ID (TradeGroup ID)
    /// * `slave_account` - The slave account ID
    /// * `settings` - Slave-specific settings for this connection
    /// * `status` - Initial status (0 = DISABLED, 2 = CONNECTED/enabled)
    pub async fn add_member(
        &self,
        trade_group_id: &str,
        slave_account: &str,
        settings: SlaveSettings,
        status: i32,
    ) -> Result<()> {
        let settings_json = serde_json::to_string(&settings)?;

        let enabled_flag = if status > 0 { 1 } else { 0 };

        sqlx::query(
            "INSERT INTO trade_group_members (
                trade_group_id,
                slave_account,
                slave_settings,
                status,
                enabled_flag,
                runtime_status
            ) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(trade_group_id)
        .bind(slave_account)
        .bind(&settings_json)
        .bind(status)
        .bind(enabled_flag)
        .bind(status)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get all members for a TradeGroup
    pub async fn get_members(&self, trade_group_id: &str) -> Result<Vec<TradeGroupMember>> {
        let rows = sqlx::query(
            "SELECT id, trade_group_id, slave_account, slave_settings, status, enabled_flag, runtime_status, created_at, updated_at
             FROM trade_group_members
             WHERE trade_group_id = ?
             ORDER BY slave_account"
        )
        .bind(trade_group_id)
        .fetch_all(&self.pool)
        .await?;

        let mut members = Vec::new();
        for row in rows {
            let id: i32 = row.get("id");
            let trade_group_id: String = row.get("trade_group_id");
            let slave_account: String = row.get("slave_account");
            let settings_json: String = row.get("slave_settings");
            let slave_settings: SlaveSettings = serde_json::from_str(&settings_json)?;
            let status: i32 = row.get("status");
            let enabled_flag: bool = row.get::<i64, _>("enabled_flag") != 0;
            let runtime_status: i32 = row.try_get("runtime_status").unwrap_or(status);
            let created_at: String = row.get("created_at");
            let updated_at: String = row.get("updated_at");

            members.push(TradeGroupMember {
                id,
                trade_group_id,
                slave_account,
                slave_settings,
                status: runtime_status,
                runtime_status,
                warning_codes: Vec::new(),
                enabled_flag,
                created_at,
                updated_at,
            });
        }

        Ok(members)
    }

    /// Get a specific member
    pub async fn get_member(
        &self,
        trade_group_id: &str,
        slave_account: &str,
    ) -> Result<Option<TradeGroupMember>> {
        let row = sqlx::query(
            "SELECT id, trade_group_id, slave_account, slave_settings, status, enabled_flag, runtime_status, created_at, updated_at
             FROM trade_group_members
             WHERE trade_group_id = ? AND slave_account = ?"
        )
        .bind(trade_group_id)
        .bind(slave_account)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let id: i32 = row.get("id");
            let trade_group_id: String = row.get("trade_group_id");
            let slave_account: String = row.get("slave_account");
            let settings_json: String = row.get("slave_settings");
            let slave_settings: SlaveSettings = serde_json::from_str(&settings_json)?;
            let status: i32 = row.get("status");
            let enabled_flag: bool = row.get::<i64, _>("enabled_flag") != 0;
            let runtime_status: i32 = row.try_get("runtime_status").unwrap_or(status);
            let created_at: String = row.get("created_at");
            let updated_at: String = row.get("updated_at");

            Ok(Some(TradeGroupMember {
                id,
                trade_group_id,
                slave_account,
                slave_settings,
                status: runtime_status,
                runtime_status,
                warning_codes: Vec::new(),
                enabled_flag,
                created_at,
                updated_at,
            }))
        } else {
            Ok(None)
        }
    }

    /// Update member settings
    pub async fn update_member_settings(
        &self,
        trade_group_id: &str,
        slave_account: &str,
        settings: SlaveSettings,
    ) -> Result<()> {
        let settings_json = serde_json::to_string(&settings)?;

        let result = sqlx::query(
            "UPDATE trade_group_members
             SET slave_settings = ?, updated_at = CURRENT_TIMESTAMP
             WHERE trade_group_id = ? AND slave_account = ?",
        )
        .bind(&settings_json)
        .bind(trade_group_id)
        .bind(slave_account)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            anyhow::bail!(
                "Member not found: trade_group_id={}, slave_account={}",
                trade_group_id,
                slave_account
            );
        }

        Ok(())
    }

    /// Update the user intent flag for a member
    pub async fn update_member_enabled_flag(
        &self,
        trade_group_id: &str,
        slave_account: &str,
        enabled: bool,
    ) -> Result<()> {
        let flag = if enabled { 1 } else { 0 };
        let result = sqlx::query(
            "UPDATE trade_group_members
             SET enabled_flag = ?, updated_at = CURRENT_TIMESTAMP
             WHERE trade_group_id = ? AND slave_account = ?",
        )
        .bind(flag)
        .bind(trade_group_id)
        .bind(slave_account)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            anyhow::bail!(
                "Member not found: trade_group_id={}, slave_account={}",
                trade_group_id,
                slave_account
            );
        }

        Ok(())
    }

    /// Update runtime status (calculated by the status engine)
    pub async fn update_member_runtime_status(
        &self,
        trade_group_id: &str,
        slave_account: &str,
        runtime_status: i32,
    ) -> Result<()> {
        let result = sqlx::query(
            "UPDATE trade_group_members
             SET runtime_status = ?, status = ?, updated_at = CURRENT_TIMESTAMP
             WHERE trade_group_id = ? AND slave_account = ?",
        )
        .bind(runtime_status)
        .bind(runtime_status)
        .bind(trade_group_id)
        .bind(slave_account)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            anyhow::bail!(
                "Member not found: trade_group_id={}, slave_account={}",
                trade_group_id,
                slave_account
            );
        }

        Ok(())
    }

    /// Get all Masters (trade_group_ids) that a Slave is connected to
    ///
    /// # Arguments
    /// * `slave_account` - The slave account ID
    ///
    /// # Returns
    /// Vector of master account IDs (trade_group_ids) that this Slave is connected to
    #[allow(dead_code)]
    pub async fn get_masters_for_slave(&self, slave_account: &str) -> Result<Vec<String>> {
        let rows = sqlx::query(
            "SELECT DISTINCT trade_group_id
             FROM trade_group_members
             WHERE slave_account = ?
             ORDER BY trade_group_id",
        )
        .bind(slave_account)
        .fetch_all(&self.pool)
        .await?;

        let masters: Vec<String> = rows.iter().map(|row| row.get("trade_group_id")).collect();
        Ok(masters)
    }

    /// Delete a member
    pub async fn delete_member(&self, trade_group_id: &str, slave_account: &str) -> Result<()> {
        sqlx::query(
            "DELETE FROM trade_group_members
             WHERE trade_group_id = ? AND slave_account = ?",
        )
        .bind(trade_group_id)
        .bind(slave_account)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
