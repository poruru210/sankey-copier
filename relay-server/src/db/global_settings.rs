//! Global Settings CRUD operations
//!
//! Implementation of Database methods for managing global settings,
//! including VictoriaLogs configuration.

use crate::models::VLogsGlobalSettings;
use anyhow::Result;
use sqlx::Row;

use super::Database;

/// Key used for VictoriaLogs settings in global_settings table
const VLOGS_SETTINGS_KEY: &str = "victoria_logs";

impl Database {
    // ============================================================================
    // VictoriaLogs Settings Operations
    // ============================================================================

    /// Get VictoriaLogs settings
    /// Returns default settings if not found in database
    pub async fn get_vlogs_settings(&self) -> Result<VLogsGlobalSettings> {
        let row = sqlx::query("SELECT value FROM global_settings WHERE key = ?")
            .bind(VLOGS_SETTINGS_KEY)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            let value: String = row.get("value");
            let settings: VLogsGlobalSettings = serde_json::from_str(&value)?;
            Ok(settings)
        } else {
            // Return default settings if not found
            Ok(VLogsGlobalSettings::default())
        }
    }

    /// Save VictoriaLogs settings
    /// Uses INSERT OR REPLACE for upsert behavior
    pub async fn save_vlogs_settings(&self, settings: &VLogsGlobalSettings) -> Result<()> {
        let value = serde_json::to_string(settings)?;

        sqlx::query(
            "INSERT OR REPLACE INTO global_settings (key, value, updated_at) VALUES (?, ?, CURRENT_TIMESTAMP)",
        )
        .bind(VLOGS_SETTINGS_KEY)
        .bind(&value)
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

    #[tokio::test]
    async fn test_get_default_vlogs_settings() {
        let db = create_test_db().await;

        let settings = db.get_vlogs_settings().await.unwrap();

        // Should return default settings when not set
        assert!(!settings.enabled);
        assert!(settings.endpoint.contains("localhost:9428"));
        assert_eq!(settings.batch_size, 100);
        assert_eq!(settings.flush_interval_secs, 5);
    }

    #[tokio::test]
    async fn test_save_and_get_vlogs_settings() {
        let db = create_test_db().await;

        let settings = VLogsGlobalSettings {
            enabled: true,
            endpoint: "http://vlogs.example.com:9428/insert/jsonline".to_string(),
            batch_size: 50,
            flush_interval_secs: 10,
            log_level: "INFO".to_string(),
        };

        db.save_vlogs_settings(&settings).await.unwrap();

        let retrieved = db.get_vlogs_settings().await.unwrap();

        assert!(retrieved.enabled);
        assert_eq!(retrieved.endpoint, settings.endpoint);
        assert_eq!(retrieved.batch_size, 50);
        assert_eq!(retrieved.flush_interval_secs, 10);
        assert_eq!(retrieved.log_level, "INFO");
    }

    #[tokio::test]
    async fn test_update_vlogs_settings() {
        let db = create_test_db().await;

        // First save
        let settings1 = VLogsGlobalSettings {
            enabled: true,
            endpoint: "http://first.example.com".to_string(),
            batch_size: 100,
            flush_interval_secs: 5,
            log_level: "DEBUG".to_string(),
        };
        db.save_vlogs_settings(&settings1).await.unwrap();

        // Update
        let settings2 = VLogsGlobalSettings {
            enabled: false,
            endpoint: "http://second.example.com".to_string(),
            batch_size: 200,
            flush_interval_secs: 15,
            log_level: "WARN".to_string(),
        };
        db.save_vlogs_settings(&settings2).await.unwrap();

        let retrieved = db.get_vlogs_settings().await.unwrap();

        assert!(!retrieved.enabled);
        assert_eq!(retrieved.endpoint, settings2.endpoint);
        assert_eq!(retrieved.batch_size, 200);
        assert_eq!(retrieved.flush_interval_secs, 15);
        assert_eq!(retrieved.log_level, "WARN");
    }
}
