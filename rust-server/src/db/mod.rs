use sqlx::{sqlite::SqlitePool, Row};
use anyhow::Result;
use crate::models::{CopySettings, SymbolMapping, TradeFilters};

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
                enabled BOOLEAN NOT NULL DEFAULT 1,
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
            "SELECT id, enabled, master_account, slave_account, lot_multiplier, reverse_trade
             FROM copy_settings WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let setting_id: i32 = row.get("id");

            // Get symbol mappings
            let mappings = sqlx::query_as::<_, (String, String)>(
                "SELECT source_symbol, target_symbol FROM symbol_mappings WHERE setting_id = ?"
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
                    allowed_symbols: f.get::<Option<String>, _>("allowed_symbols")
                        .and_then(|s| serde_json::from_str(&s).ok()),
                    blocked_symbols: f.get::<Option<String>, _>("blocked_symbols")
                        .and_then(|s| serde_json::from_str(&s).ok()),
                    allowed_magic_numbers: f.get::<Option<String>, _>("allowed_magic_numbers")
                        .and_then(|s| serde_json::from_str(&s).ok()),
                    blocked_magic_numbers: f.get::<Option<String>, _>("blocked_magic_numbers")
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
                enabled: row.get("enabled"),
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
    pub async fn get_settings_for_slave(&self, slave_account: &str) -> Result<Option<CopySettings>> {
        let row = sqlx::query(
            "SELECT id, enabled, master_account, slave_account, lot_multiplier, reverse_trade
             FROM copy_settings WHERE slave_account = ? AND enabled = 1 LIMIT 1"
        )
        .bind(slave_account)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let setting_id: i32 = row.get("id");

            // Get symbol mappings
            let mappings = sqlx::query_as::<_, (String, String)>(
                "SELECT source_symbol, target_symbol FROM symbol_mappings WHERE setting_id = ?"
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
                    allowed_symbols: f.get::<Option<String>, _>("allowed_symbols")
                        .and_then(|s| serde_json::from_str(&s).ok()),
                    blocked_symbols: f.get::<Option<String>, _>("blocked_symbols")
                        .and_then(|s| serde_json::from_str(&s).ok()),
                    allowed_magic_numbers: f.get::<Option<String>, _>("allowed_magic_numbers")
                        .and_then(|s| serde_json::from_str(&s).ok()),
                    blocked_magic_numbers: f.get::<Option<String>, _>("blocked_magic_numbers")
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
                enabled: row.get("enabled"),
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

    pub async fn list_copy_settings(&self) -> Result<Vec<CopySettings>> {
        // Fetch all copy_settings
        let settings_rows = sqlx::query(
            "SELECT id, enabled, master_account, slave_account, lot_multiplier, reverse_trade
             FROM copy_settings ORDER BY id"
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
             ORDER BY setting_id, id"
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
        let mut mappings_map: std::collections::HashMap<i32, Vec<SymbolMapping>> = std::collections::HashMap::new();
        for (setting_id, source, target) in mappings_rows {
            mappings_map
                .entry(setting_id)
                .or_insert_with(Vec::new)
                .push(SymbolMapping {
                    source_symbol: source,
                    target_symbol: target,
                });
        }

        let mut filters_map: std::collections::HashMap<i32, TradeFilters> = std::collections::HashMap::new();
        for row in filters_rows {
            let setting_id: i32 = row.get("setting_id");
            let filters = TradeFilters {
                allowed_symbols: row.get::<Option<String>, _>("allowed_symbols")
                    .and_then(|s| serde_json::from_str(&s).ok()),
                blocked_symbols: row.get::<Option<String>, _>("blocked_symbols")
                    .and_then(|s| serde_json::from_str(&s).ok()),
                allowed_magic_numbers: row.get::<Option<String>, _>("allowed_magic_numbers")
                    .and_then(|s| serde_json::from_str(&s).ok()),
                blocked_magic_numbers: row.get::<Option<String>, _>("blocked_magic_numbers")
                    .and_then(|s| serde_json::from_str(&s).ok()),
            };
            filters_map.insert(setting_id, filters);
        }

        // Assemble CopySettings
        let mut settings = Vec::new();
        for row in settings_rows {
            let id: i32 = row.get("id");
            let symbol_mappings = mappings_map.remove(&id).unwrap_or_default();
            let filters = filters_map.remove(&id).unwrap_or_else(|| TradeFilters {
                allowed_symbols: None,
                blocked_symbols: None,
                allowed_magic_numbers: None,
                blocked_magic_numbers: None,
            });

            settings.push(CopySettings {
                id: row.get("id"),
                enabled: row.get("enabled"),
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
                "INSERT INTO copy_settings (enabled, master_account, slave_account, lot_multiplier, reverse_trade)
                 VALUES (?, ?, ?, ?, ?)"
            )
            .bind(settings.enabled)
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
                    enabled = ?,
                    master_account = ?,
                    slave_account = ?,
                    lot_multiplier = ?,
                    reverse_trade = ?,
                    updated_at = CURRENT_TIMESTAMP
                 WHERE id = ?"
            )
            .bind(settings.enabled)
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

    pub async fn update_enabled_status(&self, id: i32, enabled: bool) -> Result<()> {
        sqlx::query("UPDATE copy_settings SET enabled = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(enabled)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_copy_settings(&self, id: i32) -> Result<()> {
        sqlx::query("DELETE FROM copy_settings WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
