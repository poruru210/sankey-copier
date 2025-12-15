// relay-server/src/models/global_settings.rs
//
// Global settings model for VictoriaLogs configuration.
// These settings are shared across all EAs (Master and Slave).

use serde::{Deserialize, Serialize};

/// VictoriaLogs global settings
/// Stored in the global_settings table with key "victoria_logs"
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VLogsGlobalSettings {
    /// Whether VictoriaLogs logging is enabled
    pub enabled: bool,

    /// VictoriaLogs endpoint URL
    /// Default: "http://localhost:9428/insert/jsonline?_stream_fields=source"
    pub endpoint: String,

    /// Number of log entries to batch before sending
    /// Default: 100
    pub batch_size: i32,

    /// Interval in seconds between automatic flushes
    /// Default: 5
    pub flush_interval_secs: i32,

    /// Minimum log level to output: "DEBUG", "INFO", "WARN", "ERROR"
    /// Logs below this level will be ignored by EAs
    /// Default: "DEBUG" (all logs)
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

/// Default log level (DEBUG = all logs)
fn default_log_level() -> String {
    "DEBUG".to_string()
}

impl Default for VLogsGlobalSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: "http://localhost:9428/insert/jsonline?_stream_fields=source".to_string(),
            batch_size: 100,
            flush_interval_secs: 5,
            log_level: default_log_level(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = VLogsGlobalSettings::default();

        assert!(!settings.enabled);
        assert!(settings.endpoint.contains("localhost:9428"));
        assert!(settings.endpoint.contains("_stream_fields=source"));
        assert_eq!(settings.batch_size, 100);
        assert_eq!(settings.flush_interval_secs, 5);
        assert_eq!(settings.log_level, "DEBUG");
    }

    #[test]
    fn test_serialization() {
        let settings = VLogsGlobalSettings {
            enabled: true,
            endpoint: "http://vlogs.example.com:9428/insert/jsonline".to_string(),
            batch_size: 50,
            flush_interval_secs: 10,
            log_level: "INFO".to_string(),
        };

        let json = serde_json::to_string(&settings).unwrap();
        let deserialized: VLogsGlobalSettings = serde_json::from_str(&json).unwrap();

        assert!(deserialized.enabled);
        assert_eq!(deserialized.endpoint, settings.endpoint);
        assert_eq!(deserialized.batch_size, 50);
        assert_eq!(deserialized.flush_interval_secs, 10);
        assert_eq!(deserialized.log_level, "INFO");
    }
}
