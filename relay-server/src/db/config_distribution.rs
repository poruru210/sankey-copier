//! Config distribution methods
//!
//! Implementation of Database methods for distributing configuration
//! to Master and Slave EAs, including connection status management.

use crate::models::{MasterSettings, SlaveConfigWithMaster, SlaveSettings};
use anyhow::{anyhow, Result};
use sqlx::Row;

use super::Database;

impl Database {
    // ============================================================================
    // Config Distribution Methods
    // ============================================================================

    /// Get Master settings for config distribution to Master EA
    pub async fn get_settings_for_master(&self, master_account: &str) -> Result<MasterSettings> {
        let trade_group = self
            .get_trade_group(master_account)
            .await?
            .ok_or_else(|| anyhow!("TradeGroup not found for master: {}", master_account))?;

        Ok(trade_group.master_settings)
    }

    /// Get Slave settings for config distribution to Slave EA
    /// Returns all enabled settings for the given slave_account
    pub async fn get_settings_for_slave(
        &self,
        slave_account: &str,
    ) -> Result<Vec<SlaveConfigWithMaster>> {
        let rows = sqlx::query(
            "SELECT trade_group_id, slave_account, slave_settings, status
             FROM trade_group_members
             WHERE slave_account = ? AND status > 0
             ORDER BY trade_group_id",
        )
        .bind(slave_account)
        .fetch_all(&self.pool)
        .await?;

        let mut configs = Vec::new();
        for row in rows {
            let master_account: String = row.get("trade_group_id");
            let slave_account: String = row.get("slave_account");
            let settings_json: String = row.get("slave_settings");
            let slave_settings: SlaveSettings = serde_json::from_str(&settings_json)?;
            let status: i32 = row.get("status");

            configs.push(SlaveConfigWithMaster {
                master_account,
                slave_account,
                status,
                slave_settings,
            });
        }

        Ok(configs)
    }

    /// Update all enabled members for a master to CONNECTED (2) when master comes online
    pub async fn update_master_statuses_connected(&self, master_account: &str) -> Result<usize> {
        let result = sqlx::query(
            "UPDATE trade_group_members
             SET status = 2, updated_at = CURRENT_TIMESTAMP
             WHERE trade_group_id = ? AND status > 0",
        )
        .bind(master_account)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as usize)
    }

    /// Update all connected members for a master to ENABLED (1) when master goes offline
    pub async fn update_master_statuses_disconnected(&self, master_account: &str) -> Result<usize> {
        let result = sqlx::query(
            "UPDATE trade_group_members
             SET status = 1, updated_at = CURRENT_TIMESTAMP
             WHERE trade_group_id = ? AND status = 2",
        )
        .bind(master_account)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as usize)
    }
}
