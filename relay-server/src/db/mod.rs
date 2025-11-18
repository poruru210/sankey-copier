use crate::models::{CopySettings, SymbolMapping, TradeFilters};
use anyhow::Result;
use sqlx::{sqlite::SqlitePool, Row};

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = SqlitePool::connect(database_url).await?;

        // Create tables
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS copy_settings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                status INTEGER NOT NULL DEFAULT 0,
                master_account TEXT NOT NULL,
                slave_account TEXT NOT NULL,
                lot_multiplier REAL,
                reverse_trade BOOLEAN NOT NULL DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(master_account, slave_account)
            )
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS symbol_mappings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                setting_id INTEGER NOT NULL,
                source_symbol TEXT NOT NULL,
                target_symbol TEXT NOT NULL,
                FOREIGN KEY (setting_id) REFERENCES copy_settings(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS trade_filters (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                setting_id INTEGER NOT NULL,
                allowed_symbols TEXT,
                blocked_symbols TEXT,
                allowed_magic_numbers TEXT,
                blocked_magic_numbers TEXT,
                FOREIGN KEY (setting_id) REFERENCES copy_settings(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&pool)
        .await?;

        Ok(Self { pool })
    }

    pub async fn get_copy_settings(&self, id: i32) -> Result<Option<CopySettings>> {
        let row = sqlx::query(
            "SELECT id, status, master_account, slave_account, lot_multiplier, reverse_trade
             FROM copy_settings WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let setting_id: i32 = row.get("id");

            // Get symbol mappings
            let mappings = sqlx::query_as::<_, (String, String)>(
                "SELECT source_symbol, target_symbol FROM symbol_mappings WHERE setting_id = ?",
            )
            .bind(setting_id)
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|(source, target)| SymbolMapping {
                source_symbol: source,
                target_symbol: target,
            })
            .collect();

            // Get filters
            let filter_row = sqlx::query(
                "SELECT allowed_symbols, blocked_symbols, allowed_magic_numbers, blocked_magic_numbers
                 FROM trade_filters WHERE setting_id = ?"
            )
            .bind(setting_id)
            .fetch_optional(&self.pool)
            .await?;

            let filters = if let Some(f) = filter_row {
                TradeFilters {
                    allowed_symbols: f
                        .get::<Option<String>, _>("allowed_symbols")
                        .and_then(|s| serde_json::from_str(&s).ok()),
                    blocked_symbols: f
                        .get::<Option<String>, _>("blocked_symbols")
                        .and_then(|s| serde_json::from_str(&s).ok()),
                    allowed_magic_numbers: f
                        .get::<Option<String>, _>("allowed_magic_numbers")
                        .and_then(|s| serde_json::from_str(&s).ok()),
                    blocked_magic_numbers: f
                        .get::<Option<String>, _>("blocked_magic_numbers")
                        .and_then(|s| serde_json::from_str(&s).ok()),
                }
            } else {
                TradeFilters {
                    allowed_symbols: None,
                    blocked_symbols: None,
                    allowed_magic_numbers: None,
                    blocked_magic_numbers: None,
                }
            };

            Ok(Some(CopySettings {
                id: row.get("id"),
                status: row.get("status"),
                master_account: row.get("master_account"),
                slave_account: row.get("slave_account"),
                lot_multiplier: row.get("lot_multiplier"),
                reverse_trade: row.get("reverse_trade"),
                symbol_mappings: mappings,
                filters,
            }))
        } else {
            Ok(None)
        }
    }

    /// Get enabled copy settings for a specific slave account
    /// Used in Phase 2 for registration-triggered CONFIG distribution
    pub async fn get_settings_for_slave(
        &self,
        slave_account: &str,
    ) -> Result<Option<CopySettings>> {
        let row = sqlx::query(
            "SELECT id, status, master_account, slave_account, lot_multiplier, reverse_trade
             FROM copy_settings WHERE slave_account = ? AND status > 0 LIMIT 1",
        )
        .bind(slave_account)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let setting_id: i32 = row.get("id");

            // Get symbol mappings
            let mappings = sqlx::query_as::<_, (String, String)>(
                "SELECT source_symbol, target_symbol FROM symbol_mappings WHERE setting_id = ?",
            )
            .bind(setting_id)
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|(source, target)| SymbolMapping {
                source_symbol: source,
                target_symbol: target,
            })
            .collect();

            // Get filters
            let filter_row = sqlx::query(
                "SELECT allowed_symbols, blocked_symbols, allowed_magic_numbers, blocked_magic_numbers
                 FROM trade_filters WHERE setting_id = ?"
            )
            .bind(setting_id)
            .fetch_optional(&self.pool)
            .await?;

            let filters = if let Some(f) = filter_row {
                TradeFilters {
                    allowed_symbols: f
                        .get::<Option<String>, _>("allowed_symbols")
                        .and_then(|s| serde_json::from_str(&s).ok()),
                    blocked_symbols: f
                        .get::<Option<String>, _>("blocked_symbols")
                        .and_then(|s| serde_json::from_str(&s).ok()),
                    allowed_magic_numbers: f
                        .get::<Option<String>, _>("allowed_magic_numbers")
                        .and_then(|s| serde_json::from_str(&s).ok()),
                    blocked_magic_numbers: f
                        .get::<Option<String>, _>("blocked_magic_numbers")
                        .and_then(|s| serde_json::from_str(&s).ok()),
                }
            } else {
                TradeFilters {
                    allowed_symbols: None,
                    blocked_symbols: None,
                    allowed_magic_numbers: None,
                    blocked_magic_numbers: None,
                }
            };

            Ok(Some(CopySettings {
                id: row.get("id"),
                status: row.get("status"),
                master_account: row.get("master_account"),
                slave_account: row.get("slave_account"),
                lot_multiplier: row.get("lot_multiplier"),
                reverse_trade: row.get("reverse_trade"),
                symbol_mappings: mappings,
                filters,
            }))
        } else {
            Ok(None)
        }
    }

    /// Get all copy settings for a specific master account
    /// Used to notify all slaves when master's is_trade_allowed changes
    pub async fn get_settings_for_master(&self, master_account: &str) -> Result<Vec<CopySettings>> {
        // Fetch all copy_settings for this master
        let settings_rows = sqlx::query(
            "SELECT id, status, master_account, slave_account, lot_multiplier, reverse_trade
             FROM copy_settings WHERE master_account = ? ORDER BY id",
        )
        .bind(master_account)
        .fetch_all(&self.pool)
        .await?;

        if settings_rows.is_empty() {
            return Ok(Vec::new());
        }

        // Collect setting IDs
        let setting_ids: Vec<i32> = settings_rows.iter().map(|row| row.get("id")).collect();

        // Fetch all symbol_mappings for these settings
        let placeholders = setting_ids
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");
        let query_str = format!(
            "SELECT setting_id, source_symbol, target_symbol
             FROM symbol_mappings
             WHERE setting_id IN ({})
             ORDER BY setting_id, id",
            placeholders
        );

        let mut query = sqlx::query_as::<_, (i32, String, String)>(&query_str);
        for id in &setting_ids {
            query = query.bind(id);
        }
        let mappings_rows = query.fetch_all(&self.pool).await?;

        // Fetch all trade_filters for these settings
        let filters_query_str = format!(
            "SELECT setting_id, allowed_symbols, blocked_symbols, allowed_magic_numbers, blocked_magic_numbers
             FROM trade_filters
             WHERE setting_id IN ({})",
            placeholders
        );

        let mut filters_query = sqlx::query(&filters_query_str);
        for id in &setting_ids {
            filters_query = filters_query.bind(id);
        }
        let filters_rows = filters_query.fetch_all(&self.pool).await?;

        // Build lookup maps
        let mut mappings_map: std::collections::HashMap<i32, Vec<SymbolMapping>> =
            std::collections::HashMap::new();
        for (setting_id, source, target) in mappings_rows {
            mappings_map
                .entry(setting_id)
                .or_default()
                .push(SymbolMapping {
                    source_symbol: source,
                    target_symbol: target,
                });
        }

        let mut filters_map: std::collections::HashMap<i32, TradeFilters> =
            std::collections::HashMap::new();
        for row in filters_rows {
            let setting_id: i32 = row.get("setting_id");
            let filters = TradeFilters {
                allowed_symbols: row
                    .get::<Option<String>, _>("allowed_symbols")
                    .and_then(|s| serde_json::from_str(&s).ok()),
                blocked_symbols: row
                    .get::<Option<String>, _>("blocked_symbols")
                    .and_then(|s| serde_json::from_str(&s).ok()),
                allowed_magic_numbers: row
                    .get::<Option<String>, _>("allowed_magic_numbers")
                    .and_then(|s| serde_json::from_str(&s).ok()),
                blocked_magic_numbers: row
                    .get::<Option<String>, _>("blocked_magic_numbers")
                    .and_then(|s| serde_json::from_str(&s).ok()),
            };
            filters_map.insert(setting_id, filters);
        }

        // Assemble CopySettings
        let mut settings = Vec::new();
        for row in settings_rows {
            let id: i32 = row.get("id");
            let symbol_mappings = mappings_map.remove(&id).unwrap_or_default();
            let filters = filters_map.remove(&id).unwrap_or(TradeFilters {
                allowed_symbols: None,
                blocked_symbols: None,
                allowed_magic_numbers: None,
                blocked_magic_numbers: None,
            });

            settings.push(CopySettings {
                id: row.get("id"),
                status: row.get("status"),
                master_account: row.get("master_account"),
                slave_account: row.get("slave_account"),
                lot_multiplier: row.get("lot_multiplier"),
                reverse_trade: row.get("reverse_trade"),
                symbol_mappings,
                filters,
            });
        }

        Ok(settings)
    }

    pub async fn list_copy_settings(&self) -> Result<Vec<CopySettings>> {
        // Fetch all copy_settings
        let settings_rows = sqlx::query(
            "SELECT id, status, master_account, slave_account, lot_multiplier, reverse_trade
             FROM copy_settings ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await?;

        if settings_rows.is_empty() {
            return Ok(Vec::new());
        }

        // Fetch all symbol_mappings in one query
        let mappings_rows = sqlx::query_as::<_, (i32, String, String)>(
            "SELECT setting_id, source_symbol, target_symbol
             FROM symbol_mappings
             ORDER BY setting_id, id",
        )
        .fetch_all(&self.pool)
        .await?;

        // Fetch all trade_filters in one query
        let filters_rows = sqlx::query(
            "SELECT setting_id, allowed_symbols, blocked_symbols, allowed_magic_numbers, blocked_magic_numbers
             FROM trade_filters"
        )
        .fetch_all(&self.pool)
        .await?;

        // Build lookup maps
        let mut mappings_map: std::collections::HashMap<i32, Vec<SymbolMapping>> =
            std::collections::HashMap::new();
        for (setting_id, source, target) in mappings_rows {
            mappings_map
                .entry(setting_id)
                .or_default()
                .push(SymbolMapping {
                    source_symbol: source,
                    target_symbol: target,
                });
        }

        let mut filters_map: std::collections::HashMap<i32, TradeFilters> =
            std::collections::HashMap::new();
        for row in filters_rows {
            let setting_id: i32 = row.get("setting_id");
            let filters = TradeFilters {
                allowed_symbols: row
                    .get::<Option<String>, _>("allowed_symbols")
                    .and_then(|s| serde_json::from_str(&s).ok()),
                blocked_symbols: row
                    .get::<Option<String>, _>("blocked_symbols")
                    .and_then(|s| serde_json::from_str(&s).ok()),
                allowed_magic_numbers: row
                    .get::<Option<String>, _>("allowed_magic_numbers")
                    .and_then(|s| serde_json::from_str(&s).ok()),
                blocked_magic_numbers: row
                    .get::<Option<String>, _>("blocked_magic_numbers")
                    .and_then(|s| serde_json::from_str(&s).ok()),
            };
            filters_map.insert(setting_id, filters);
        }

        // Assemble CopySettings
        let mut settings = Vec::new();
        for row in settings_rows {
            let id: i32 = row.get("id");
            let symbol_mappings = mappings_map.remove(&id).unwrap_or_default();
            let filters = filters_map.remove(&id).unwrap_or(TradeFilters {
                allowed_symbols: None,
                blocked_symbols: None,
                allowed_magic_numbers: None,
                blocked_magic_numbers: None,
            });

            settings.push(CopySettings {
                id: row.get("id"),
                status: row.get("status"),
                master_account: row.get("master_account"),
                slave_account: row.get("slave_account"),
                lot_multiplier: row.get("lot_multiplier"),
                reverse_trade: row.get("reverse_trade"),
                symbol_mappings,
                filters,
            });
        }

        Ok(settings)
    }

    pub async fn save_copy_settings(&self, settings: &CopySettings) -> Result<i32> {
        let id = if settings.id == 0 {
            // New record - INSERT
            let result = sqlx::query(
                "INSERT INTO copy_settings (status, master_account, slave_account, lot_multiplier, reverse_trade)
                 VALUES (?, ?, ?, ?, ?)"
            )
            .bind(settings.status)
            .bind(&settings.master_account)
            .bind(&settings.slave_account)
            .bind(settings.lot_multiplier)
            .bind(settings.reverse_trade)
            .execute(&self.pool)
            .await?;

            result.last_insert_rowid() as i32
        } else {
            // Existing record - UPDATE
            sqlx::query(
                "UPDATE copy_settings SET
                    status = ?,
                    master_account = ?,
                    slave_account = ?,
                    lot_multiplier = ?,
                    reverse_trade = ?,
                    updated_at = CURRENT_TIMESTAMP
                 WHERE id = ?",
            )
            .bind(settings.status)
            .bind(&settings.master_account)
            .bind(&settings.slave_account)
            .bind(settings.lot_multiplier)
            .bind(settings.reverse_trade)
            .bind(settings.id)
            .execute(&self.pool)
            .await?;

            settings.id
        };

        // Clear and insert symbol mappings
        sqlx::query("DELETE FROM symbol_mappings WHERE setting_id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        for mapping in &settings.symbol_mappings {
            sqlx::query("INSERT INTO symbol_mappings (setting_id, source_symbol, target_symbol) VALUES (?, ?, ?)")
                .bind(id)
                .bind(&mapping.source_symbol)
                .bind(&mapping.target_symbol)
                .execute(&self.pool)
                .await?;
        }

        // Clear and insert filters
        sqlx::query("DELETE FROM trade_filters WHERE setting_id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        sqlx::query(
            "INSERT INTO trade_filters (setting_id, allowed_symbols, blocked_symbols, allowed_magic_numbers, blocked_magic_numbers)
             VALUES (?, ?, ?, ?, ?)"
        )
        .bind(id)
        .bind(serde_json::to_string(&settings.filters.allowed_symbols)?)
        .bind(serde_json::to_string(&settings.filters.blocked_symbols)?)
        .bind(serde_json::to_string(&settings.filters.allowed_magic_numbers)?)
        .bind(serde_json::to_string(&settings.filters.blocked_magic_numbers)?)
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    pub async fn update_status(&self, id: i32, status: i32) -> Result<()> {
        sqlx::query(
            "UPDATE copy_settings SET status = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(status)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Update all enabled settings for a master to CONNECTED (2) when master comes online
    pub async fn update_master_statuses_connected(&self, master_account: &str) -> Result<usize> {
        let result = sqlx::query(
            "UPDATE copy_settings
             SET status = 2, updated_at = CURRENT_TIMESTAMP
             WHERE master_account = ? AND status > 0",
        )
        .bind(master_account)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() as usize)
    }

    /// Update all connected settings for a master to ENABLED (1) when master goes offline
    pub async fn update_master_statuses_disconnected(&self, master_account: &str) -> Result<usize> {
        let result = sqlx::query(
            "UPDATE copy_settings
             SET status = 1, updated_at = CURRENT_TIMESTAMP
             WHERE master_account = ? AND status = 2",
        )
        .bind(master_account)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() as usize)
    }

    pub async fn delete_copy_settings(&self, id: i32) -> Result<()> {
        sqlx::query("DELETE FROM copy_settings WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        assert!(result.is_none());
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
    async fn test_delete_cascades_to_related_tables() {
        let db = create_test_db().await;

        let mut settings = create_test_settings();
        settings.symbol_mappings = vec![SymbolMapping {
            source_symbol: "EURUSD".to_string(),
            target_symbol: "EURUSDm".to_string(),
        }];
        settings.filters = TradeFilters {
            allowed_symbols: Some(vec!["EURUSD".to_string()]),
            blocked_symbols: None,
            allowed_magic_numbers: None,
            blocked_magic_numbers: None,
        };

        let id = db.save_copy_settings(&settings).await.unwrap();

        // Delete parent
        db.delete_copy_settings(id).await.unwrap();

        // Related records should also be deleted (CASCADE)
        let mappings_count: i32 =
            sqlx::query_scalar("SELECT COUNT(*) FROM symbol_mappings WHERE setting_id = ?")
                .bind(id)
                .fetch_one(&db.pool)
                .await
                .unwrap();

        assert_eq!(mappings_count, 0);

        let filters_count: i32 =
            sqlx::query_scalar("SELECT COUNT(*) FROM trade_filters WHERE setting_id = ?")
                .bind(id)
                .fetch_one(&db.pool)
                .await
                .unwrap();

        assert_eq!(filters_count, 0);
    }

    #[tokio::test]
    async fn test_get_settings_for_slave_disabled() {
        let db = create_test_db().await;

        let mut settings = create_test_settings();
        settings.status = 0; // STATUS_DISABLED

        db.save_copy_settings(&settings).await.unwrap();

        // Should not return disabled settings
        let result = db.get_settings_for_slave("SLAVE_001").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_settings_for_slave_enabled() {
        let db = create_test_db().await;

        let settings = create_test_settings();
        db.save_copy_settings(&settings).await.unwrap();

        // Should return enabled settings
        let result = db.get_settings_for_slave("SLAVE_001").await.unwrap();
        assert!(result.is_some());

        let retrieved = result.unwrap();
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
