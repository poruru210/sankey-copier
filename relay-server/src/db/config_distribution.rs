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
    /// Returns all settings for the given slave_account (including DISABLED)
    /// NOTE: DISABLED members are included so Slave EA can show appropriate status
    pub async fn get_settings_for_slave(
        &self,
        slave_account: &str,
    ) -> Result<Vec<SlaveConfigWithMaster>> {
        let rows = sqlx::query(
            "SELECT trade_group_id, slave_account, slave_settings, status, enabled_flag
             FROM trade_group_members
             WHERE slave_account = ?
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
            let enabled_flag: bool = row.get::<i64, _>("enabled_flag") != 0;

            configs.push(SlaveConfigWithMaster {
                master_account,
                slave_account,
                status,
                enabled_flag,
                warning_codes: Vec::new(), // Populated by Status Engine in message handlers
                slave_settings,
            });
        }

        Ok(configs)
    }

    /// Update all connected members for a master to ENABLED (1) when master goes offline
    pub async fn update_master_statuses_enabled(&self, master_account: &str) -> Result<usize> {
        // Debug: count how many connected (status=2) rows exist for this master
        let connected_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM trade_group_members WHERE trade_group_id = ? AND status = 2",
        )
        .bind(master_account)
        .fetch_one(&self.pool)
        .await?;

        tracing::debug!(master_account = %master_account, connected_count = connected_count, "update_master_statuses_disconnected: connected rows before update");

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

    // NOTE: `update_master_statuses_disconnected` was removed — callers were
    // migrated to `update_master_statuses_enabled`. Keeping this implementation
    // in the git history if needed; removal avoids dead_code warnings.
}
