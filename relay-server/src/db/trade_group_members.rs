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
    pub async fn add_member(
        &self,
        trade_group_id: &str,
        slave_account: &str,
        settings: SlaveSettings,
    ) -> Result<()> {
        let settings_json = serde_json::to_string(&settings)?;

        sqlx::query(
            "INSERT INTO trade_group_members (trade_group_id, slave_account, slave_settings, status)
             VALUES (?, ?, ?, 1)" // default status = ENABLED
        )
        .bind(trade_group_id)
        .bind(slave_account)
        .bind(&settings_json)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get all members for a TradeGroup
    pub async fn get_members(&self, trade_group_id: &str) -> Result<Vec<TradeGroupMember>> {
        let rows = sqlx::query(
            "SELECT id, trade_group_id, slave_account, slave_settings, status, created_at, updated_at
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
            let created_at: String = row.get("created_at");
            let updated_at: String = row.get("updated_at");

            members.push(TradeGroupMember {
                id,
                trade_group_id,
                slave_account,
                slave_settings,
                status,
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
            "SELECT id, trade_group_id, slave_account, slave_settings, status, created_at, updated_at
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
            let created_at: String = row.get("created_at");
            let updated_at: String = row.get("updated_at");

            Ok(Some(TradeGroupMember {
                id,
                trade_group_id,
                slave_account,
                slave_settings,
                status,
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
            anyhow::bail!("Member not found: trade_group_id={}, slave_account={}", trade_group_id, slave_account);
        }

        Ok(())
    }

    /// Update member status
    pub async fn update_member_status(
        &self,
        trade_group_id: &str,
        slave_account: &str,
        status: i32,
    ) -> Result<()> {
        let result = sqlx::query(
            "UPDATE trade_group_members
             SET status = ?, updated_at = CURRENT_TIMESTAMP
             WHERE trade_group_id = ? AND slave_account = ?",
        )
        .bind(status)
        .bind(trade_group_id)
        .bind(slave_account)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            anyhow::bail!("Member not found: trade_group_id={}, slave_account={}", trade_group_id, slave_account);
        }

        Ok(())
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
