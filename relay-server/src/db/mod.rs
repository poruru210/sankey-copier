use crate::models::{
    CopySettings, MasterSettings, SlaveConfigWithMaster, SlaveSettings, TradeGroup,
    TradeGroupMember,
};
use anyhow::{anyhow, Result};
use sqlx::{sqlite::SqlitePool, Row};

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

        sqlx::query(
            "UPDATE trade_group_members
             SET slave_settings = ?, updated_at = CURRENT_TIMESTAMP
             WHERE trade_group_id = ? AND slave_account = ?",
        )
        .bind(&settings_json)
        .bind(trade_group_id)
        .bind(slave_account)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update member status
    pub async fn update_member_status(
        &self,
        trade_group_id: &str,
        slave_account: &str,
        status: i32,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE trade_group_members
             SET status = ?, updated_at = CURRENT_TIMESTAMP
             WHERE trade_group_id = ? AND slave_account = ?",
        )
        .bind(status)
        .bind(trade_group_id)
        .bind(slave_account)
        .execute(&self.pool)
        .await?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use sankey_copier_zmq::{SymbolMapping, TradeFilters};

    async fn create_test_db() -> Database {
        Database::new("sqlite::memory:").await.unwrap()
    }

    fn create_test_settings() -> CopySettings {
        CopySettings {
            id: 0,
            status: 2, // STATUS_CONNECTED
            master_account: "MASTER_001".to_string(),
            slave_account: "SLAVE_001".to_string(),
            lot_multiplier: Some(1.5),
            reverse_trade: false,
            symbol_prefix: None,
            symbol_suffix: None,
            symbol_mappings: vec![],
            filters: TradeFilters {
                allowed_symbols: None,
                blocked_symbols: None,
                allowed_magic_numbers: None,
                blocked_magic_numbers: None,
            },
        }
    }

    #[tokio::test]
    async fn test_get_nonexistent_copy_settings() {
        let db = create_test_db().await;

        // Try to get settings that don't exist
        let result = db.get_copy_settings(999).await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_settings_for_nonexistent_slave() {
        let db = create_test_db().await;

        // Try to get settings for a slave that doesn't exist
        let result = db.get_settings_for_slave("NONEXISTENT").await.unwrap();

        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_list_copy_settings_empty() {
        let db = create_test_db().await;

        let settings = db.list_copy_settings().await.unwrap();

        assert_eq!(settings.len(), 0);
    }

    #[tokio::test]
    async fn test_duplicate_master_slave_pair() {
        let db = create_test_db().await;

        let settings1 = create_test_settings();
        db.save_copy_settings(&settings1).await.unwrap();

        // Try to insert the same master-slave pair again
        let settings2 = create_test_settings();
        let result = db.save_copy_settings(&settings2).await;

        // Should fail due to UNIQUE constraint
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_save_and_retrieve_with_null_lot_multiplier() {
        let db = create_test_db().await;

        let mut settings = create_test_settings();
        settings.lot_multiplier = None;

        let id = db.save_copy_settings(&settings).await.unwrap();

        let retrieved = db.get_copy_settings(id).await.unwrap().unwrap();

        assert!(retrieved.lot_multiplier.is_none());
    }

    #[tokio::test]
    async fn test_save_and_retrieve_with_symbol_mappings() {
        let db = create_test_db().await;

        let mut settings = create_test_settings();
        settings.symbol_mappings = vec![
            SymbolMapping {
                source_symbol: "EURUSD".to_string(),
                target_symbol: "EURUSDm".to_string(),
            },
            SymbolMapping {
                source_symbol: "GBPUSD".to_string(),
                target_symbol: "GBPUSDm".to_string(),
            },
        ];

        let id = db.save_copy_settings(&settings).await.unwrap();

        let retrieved = db.get_copy_settings(id).await.unwrap().unwrap();

        assert_eq!(retrieved.symbol_mappings.len(), 2);
        assert_eq!(retrieved.symbol_mappings[0].source_symbol, "EURUSD");
        assert_eq!(retrieved.symbol_mappings[0].target_symbol, "EURUSDm");
    }

    #[tokio::test]
    async fn test_save_and_retrieve_with_filters() {
        let db = create_test_db().await;

        let mut settings = create_test_settings();
        settings.filters = TradeFilters {
            allowed_symbols: Some(vec!["EURUSD".to_string(), "GBPUSD".to_string()]),
            blocked_symbols: Some(vec!["USDJPY".to_string()]),
            allowed_magic_numbers: Some(vec![100, 200]),
            blocked_magic_numbers: Some(vec![999]),
        };

        let id = db.save_copy_settings(&settings).await.unwrap();

        let retrieved = db.get_copy_settings(id).await.unwrap().unwrap();

        assert_eq!(
            retrieved.filters.allowed_symbols,
            Some(vec!["EURUSD".to_string(), "GBPUSD".to_string()])
        );
        assert_eq!(
            retrieved.filters.blocked_symbols,
            Some(vec!["USDJPY".to_string()])
        );
        assert_eq!(
            retrieved.filters.allowed_magic_numbers,
            Some(vec![100, 200])
        );
        assert_eq!(retrieved.filters.blocked_magic_numbers, Some(vec![999]));
    }

    #[tokio::test]
    async fn test_update_existing_settings() {
        let db = create_test_db().await;

        // Create initial settings
        let settings = create_test_settings();
        let id = db.save_copy_settings(&settings).await.unwrap();

        // Update settings
        let mut updated_settings = db.get_copy_settings(id).await.unwrap().unwrap();
        updated_settings.lot_multiplier = Some(2.0);
        updated_settings.reverse_trade = true;

        db.save_copy_settings(&updated_settings).await.unwrap();

        // Retrieve and verify
        let retrieved = db.get_copy_settings(id).await.unwrap().unwrap();

        assert_eq!(retrieved.lot_multiplier, Some(2.0));
        assert!(retrieved.reverse_trade);
    }

    #[tokio::test]
    async fn test_update_status() {
        let db = create_test_db().await;

        let settings = create_test_settings();
        let id = db.save_copy_settings(&settings).await.unwrap();

        // Set to DISABLED (0)
        db.update_status(id, 0).await.unwrap();

        let retrieved = db.get_copy_settings(id).await.unwrap().unwrap();
        assert_eq!(retrieved.status, 0);

        // Set to ENABLED (1)
        db.update_status(id, 1).await.unwrap();

        let retrieved = db.get_copy_settings(id).await.unwrap().unwrap();
        assert_eq!(retrieved.status, 1);

        // Set to CONNECTED (2)
        db.update_status(id, 2).await.unwrap();

        let retrieved = db.get_copy_settings(id).await.unwrap().unwrap();
        assert_eq!(retrieved.status, 2);
    }

    #[tokio::test]
    async fn test_delete_copy_settings() {
        let db = create_test_db().await;

        let settings = create_test_settings();
        let id = db.save_copy_settings(&settings).await.unwrap();

        // Delete
        db.delete_copy_settings(id).await.unwrap();

        // Should no longer exist
        let retrieved = db.get_copy_settings(id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_get_settings_for_slave_disabled() {
        let db = create_test_db().await;

        let mut settings = create_test_settings();
        settings.status = 0; // STATUS_DISABLED

        db.save_copy_settings(&settings).await.unwrap();

        // Should not return disabled settings
        let result = db.get_settings_for_slave("SLAVE_001").await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_get_settings_for_slave_enabled() {
        let db = create_test_db().await;

        let settings = create_test_settings();
        db.save_copy_settings(&settings).await.unwrap();

        // Should return enabled settings
        let result = db.get_settings_for_slave("SLAVE_001").await.unwrap();
        assert!(!result.is_empty());

        let retrieved = &result[0];
        assert_eq!(retrieved.slave_account, "SLAVE_001");
        assert_eq!(retrieved.master_account, "MASTER_001");
    }

    #[tokio::test]
    async fn test_list_multiple_settings() {
        let db = create_test_db().await;

        // Create multiple settings
        for i in 1..=3 {
            let mut settings = create_test_settings();
            settings.master_account = format!("MASTER_{:03}", i);
            settings.slave_account = format!("SLAVE_{:03}", i);
            db.save_copy_settings(&settings).await.unwrap();
        }

        let all_settings = db.list_copy_settings().await.unwrap();

        assert_eq!(all_settings.len(), 3);
        assert_eq!(all_settings[0].master_account, "MASTER_001");
        assert_eq!(all_settings[1].master_account, "MASTER_002");
        assert_eq!(all_settings[2].master_account, "MASTER_003");
    }

    #[tokio::test]
    async fn test_update_clears_old_mappings() {
        let db = create_test_db().await;

        // Create settings with mappings
        let mut settings = create_test_settings();
        settings.symbol_mappings = vec![
            SymbolMapping {
                source_symbol: "EURUSD".to_string(),
                target_symbol: "EURUSDm".to_string(),
            },
            SymbolMapping {
                source_symbol: "GBPUSD".to_string(),
                target_symbol: "GBPUSDm".to_string(),
            },
        ];

        let id = db.save_copy_settings(&settings).await.unwrap();

        // Update with different mappings
        let mut updated = db.get_copy_settings(id).await.unwrap().unwrap();
        updated.symbol_mappings = vec![SymbolMapping {
            source_symbol: "USDJPY".to_string(),
            target_symbol: "USDJPYm".to_string(),
        }];

        db.save_copy_settings(&updated).await.unwrap();

        // Retrieve and verify
        let retrieved = db.get_copy_settings(id).await.unwrap().unwrap();

        assert_eq!(retrieved.symbol_mappings.len(), 1);
        assert_eq!(retrieved.symbol_mappings[0].source_symbol, "USDJPY");
    }

    #[tokio::test]
    async fn test_empty_filters_serialization() {
        let db = create_test_db().await;

        let settings = create_test_settings();
        let id = db.save_copy_settings(&settings).await.unwrap();

        let retrieved = db.get_copy_settings(id).await.unwrap().unwrap();

        // All filter fields should be None
        assert!(retrieved.filters.allowed_symbols.is_none());
        assert!(retrieved.filters.blocked_symbols.is_none());
        assert!(retrieved.filters.allowed_magic_numbers.is_none());
        assert!(retrieved.filters.blocked_magic_numbers.is_none());
    }

    #[tokio::test]
    async fn test_delete_nonexistent_settings() {
        let db = create_test_db().await;

        // Deleting non-existent settings should not error
        let result = db.delete_copy_settings(999).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_master_statuses_connected() {
        let db = create_test_db().await;

        // Create three settings for the same master: one disabled, two enabled
        let mut settings1 = create_test_settings();
        settings1.master_account = "MASTER_001".to_string();
        settings1.slave_account = "SLAVE_001".to_string();
        settings1.status = 0; // DISABLED
        let id1 = db.save_copy_settings(&settings1).await.unwrap();

        let mut settings2 = create_test_settings();
        settings2.master_account = "MASTER_001".to_string();
        settings2.slave_account = "SLAVE_002".to_string();
        settings2.status = 1; // ENABLED
        let id2 = db.save_copy_settings(&settings2).await.unwrap();

        let mut settings3 = create_test_settings();
        settings3.master_account = "MASTER_001".to_string();
        settings3.slave_account = "SLAVE_003".to_string();
        settings3.status = 1; // ENABLED
        let id3 = db.save_copy_settings(&settings3).await.unwrap();

        // Update master statuses to CONNECTED
        let count = db
            .update_master_statuses_connected("MASTER_001")
            .await
            .unwrap();

        // Should update 2 settings (the enabled ones)
        assert_eq!(count, 2);

        // Verify statuses
        let retrieved1 = db.get_copy_settings(id1).await.unwrap().unwrap();
        assert_eq!(retrieved1.status, 0); // Still DISABLED

        let retrieved2 = db.get_copy_settings(id2).await.unwrap().unwrap();
        assert_eq!(retrieved2.status, 2); // Now CONNECTED

        let retrieved3 = db.get_copy_settings(id3).await.unwrap().unwrap();
        assert_eq!(retrieved3.status, 2); // Now CONNECTED
    }

    #[tokio::test]
    async fn test_update_master_statuses_disconnected() {
        let db = create_test_db().await;

        // Create three settings for the same master with different statuses
        let mut settings1 = create_test_settings();
        settings1.master_account = "MASTER_001".to_string();
        settings1.slave_account = "SLAVE_001".to_string();
        settings1.status = 0; // DISABLED
        let id1 = db.save_copy_settings(&settings1).await.unwrap();

        let mut settings2 = create_test_settings();
        settings2.master_account = "MASTER_001".to_string();
        settings2.slave_account = "SLAVE_002".to_string();
        settings2.status = 1; // ENABLED
        let id2 = db.save_copy_settings(&settings2).await.unwrap();

        let mut settings3 = create_test_settings();
        settings3.master_account = "MASTER_001".to_string();
        settings3.slave_account = "SLAVE_003".to_string();
        settings3.status = 2; // CONNECTED
        let id3 = db.save_copy_settings(&settings3).await.unwrap();

        // Update master statuses to ENABLED (disconnected)
        let count = db
            .update_master_statuses_disconnected("MASTER_001")
            .await
            .unwrap();

        // Should update 1 setting (the connected one)
        assert_eq!(count, 1);

        // Verify statuses
        let retrieved1 = db.get_copy_settings(id1).await.unwrap().unwrap();
        assert_eq!(retrieved1.status, 0); // Still DISABLED

        let retrieved2 = db.get_copy_settings(id2).await.unwrap().unwrap();
        assert_eq!(retrieved2.status, 1); // Still ENABLED

        let retrieved3 = db.get_copy_settings(id3).await.unwrap().unwrap();
        assert_eq!(retrieved3.status, 1); // Now ENABLED (was CONNECTED)
    }

    #[tokio::test]
    async fn test_update_master_statuses_no_settings() {
        let db = create_test_db().await;

        // Try to update statuses for a master with no settings
        let count = db
            .update_master_statuses_connected("NONEXISTENT_MASTER")
            .await
            .unwrap();

        assert_eq!(count, 0);

        let count = db
            .update_master_statuses_disconnected("NONEXISTENT_MASTER")
            .await
            .unwrap();

        assert_eq!(count, 0);
    }
}
