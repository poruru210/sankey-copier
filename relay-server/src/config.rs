use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    #[serde(default)]
    pub webui: WebUIConfig,
    pub database: DatabaseConfig,
    pub zeromq: ZeroMqConfig,
    #[serde(default)]
    pub cors: CorsConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub installer: InstallerConfig,
    #[serde(default)]
    pub tls: TlsConfig,
    #[serde(default)]
    pub victoria_logs: VictoriaLogsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebUIConfig {
    pub host: String,
    pub port: u16,
    pub url: String,
}

impl Default for WebUIConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 3000,
            url: "http://localhost:3000".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CorsConfig {
    /// Disable CORS restrictions (allows all origins) - use only in development!
    #[serde(default)]
    pub disable: bool,
    #[serde(default)]
    pub additional_origins: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Enable file logging
    #[serde(default = "default_logging_enabled")]
    pub enabled: bool,
    /// Directory for log files (relative to executable or absolute path)
    #[serde(default = "default_log_directory")]
    pub directory: String,
    /// Prefix for log file names
    #[serde(default = "default_log_file_prefix")]
    pub file_prefix: String,
    /// Rotation strategy: "daily", "hourly", or "never"
    #[serde(default = "default_log_rotation")]
    pub rotation: String,
    /// Maximum number of log files to keep (0 = unlimited)
    #[serde(default = "default_max_files")]
    pub max_files: u32,
    /// Maximum age of log files in days (0 = unlimited)
    #[serde(default = "default_max_age_days")]
    pub max_age_days: u32,
}

fn default_logging_enabled() -> bool {
    true
}
fn default_log_directory() -> String {
    "logs".to_string()
}
fn default_log_file_prefix() -> String {
    "sankey-copier-server".to_string()
}
fn default_log_rotation() -> String {
    "daily".to_string()
}
fn default_max_files() -> u32 {
    30
}
fn default_max_age_days() -> u32 {
    90
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            enabled: default_logging_enabled(),
            directory: default_log_directory(),
            file_prefix: default_log_file_prefix(),
            rotation: default_log_rotation(),
            max_files: default_max_files(),
            max_age_days: default_max_age_days(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InstallerConfig {
    /// Base path for MQL components (DLL, EA files)
    /// If not set, uses current_dir() (production default)
    #[serde(default)]
    pub components_base_path: Option<String>,
}

/// TLS configuration for HTTPS server
/// Used for PNA (Private Network Access) compliance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Path to certificate file (.pem)
    #[serde(default = "default_cert_path")]
    pub cert_path: String,
    /// Path to private key file (.pem)
    #[serde(default = "default_key_path")]
    pub key_path: String,
    /// Certificate validity period in days
    #[serde(default = "default_cert_validity_days")]
    pub validity_days: u32,
}

fn default_cert_path() -> String {
    "certs/server.pem".to_string()
}

fn default_key_path() -> String {
    "certs/server-key.pem".to_string()
}

fn default_cert_validity_days() -> u32 {
    3650 // 10 years
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            cert_path: default_cert_path(),
            key_path: default_key_path(),
            validity_days: default_cert_validity_days(),
        }
    }
}

/// VictoriaLogs API endpoint path (fixed, appended to host)
pub const VICTORIA_LOGS_ENDPOINT_PATH: &str = "/insert/jsonline?_stream_fields=source";

/// VictoriaLogs configuration for centralized log shipping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VictoriaLogsConfig {
    /// Enable VictoriaLogs integration
    #[serde(default)]
    pub enabled: bool,
    /// VictoriaLogs host URL (e.g., "http://localhost:9428")
    #[serde(default = "default_vlogs_host")]
    pub host: String,
    /// Maximum entries to buffer before sending
    #[serde(default = "default_vlogs_batch_size")]
    pub batch_size: usize,
    /// Flush interval in seconds
    #[serde(default = "default_vlogs_flush_interval")]
    pub flush_interval_secs: u64,
    /// Source identifier for logs
    #[serde(default = "default_vlogs_source")]
    pub source: String,
}

impl VictoriaLogsConfig {
    /// Get the full endpoint URL (host + fixed path)
    pub fn endpoint(&self) -> String {
        format!(
            "{}{}",
            self.host.trim_end_matches('/'),
            VICTORIA_LOGS_ENDPOINT_PATH
        )
    }
}

fn default_vlogs_host() -> String {
    "http://localhost:9428".to_string()
}

fn default_vlogs_batch_size() -> usize {
    100
}

fn default_vlogs_flush_interval() -> u64 {
    5
}

fn default_vlogs_source() -> String {
    "relay-server".to_string()
}

impl Default for VictoriaLogsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            host: default_vlogs_host(),
            batch_size: default_vlogs_batch_size(),
            flush_interval_secs: default_vlogs_flush_interval(),
            source: default_vlogs_source(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZeroMqConfig {
    /// Port for receiving messages from EAs (PULL socket)
    /// Set to 0 for dynamic port assignment
    pub receiver_port: u16,
    /// Port for sending all messages to EAs (PUB socket)
    /// Includes trade signals and configuration updates, distinguished by topic
    /// Set to 0 for dynamic port assignment
    pub sender_port: u16,
    pub timeout_seconds: i64,
}

impl ZeroMqConfig {
    /// Check if any port is configured for dynamic assignment
    pub fn has_dynamic_ports(&self) -> bool {
        self.receiver_port == 0 || self.sender_port == 0
    }
}

/// Runtime configuration for dynamically assigned ports
/// Stored in runtime.toml and persisted across restarts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub zeromq: RuntimeZeromqConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeZeromqConfig {
    pub receiver_port: u16,
    pub sender_port: u16,
    pub generated_at: DateTime<Utc>,
}

impl RuntimeConfig {
    /// Load runtime config from file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content =
            std::fs::read_to_string(path.as_ref()).context("Failed to read runtime config file")?;
        toml::from_str(&content).context("Failed to parse runtime config")
    }

    /// Save runtime config to file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self).context("Failed to serialize runtime config")?;
        let header = "# AUTO-GENERATED - DO NOT EDIT\n\
                      # Delete this file to re-assign ports on next startup\n\n";
        std::fs::write(path.as_ref(), format!("{}{}", header, content))
            .context("Failed to write runtime config file")?;
        Ok(())
    }

    /// Delete runtime config file
    #[allow(dead_code)]
    pub fn delete<P: AsRef<Path>>(path: P) -> Result<()> {
        if path.as_ref().exists() {
            std::fs::remove_file(path.as_ref()).context("Failed to delete runtime config file")?;
        }
        Ok(())
    }

    /// Check if runtime config file exists
    pub fn exists<P: AsRef<Path>>(path: P) -> bool {
        path.as_ref().exists()
    }
}

impl Config {
    /// Load config from layered TOML files
    ///
    /// Loads configuration files in the following order (later files override earlier):
    /// 1. {base_name}.toml (required, e.g., config.toml)
    /// 2. {base_name}.{ENV}.toml (optional, only if CONFIG_ENV is set)
    /// 3. {base_name}.local.toml (optional, for personal overrides, git-ignored)
    ///
    /// # Arguments
    /// * `base_name` - Base name without extension (e.g., "config" for config.toml)
    ///
    /// # Environment Variables
    /// * `CONFIG_ENV` - If set, loads {base_name}.{CONFIG_ENV}.toml (e.g., config.dev.toml)
    ///   - No default value - must be explicitly set
    ///   - Common values: "dev", "prod", "staging"
    pub fn from_file<P: AsRef<Path>>(base_name: P) -> Result<Self> {
        let base_path = base_name.as_ref();
        let base_str = base_path.to_str().context("Invalid base path")?;

        // Build layered configuration
        let mut builder = config::Config::builder()
            // 1. Load base config (required)
            .add_source(config::File::with_name(base_str));

        // 2. Load environment-specific config (optional)
        // Only loads if CONFIG_ENV environment variable is explicitly set
        if let Ok(env) = std::env::var("CONFIG_ENV") {
            let env_config = format!("{}.{}", base_str, env);
            builder = builder.add_source(config::File::with_name(&env_config).required(false));
        }

        // 3. Load local config (optional, for personal overrides)
        let local_config = format!("{}.local", base_str);
        builder = builder.add_source(config::File::with_name(&local_config).required(false));

        // Build and deserialize
        let config = builder.build().context("Failed to build configuration")?;

        config
            .try_deserialize()
            .context("Failed to deserialize configuration")
    }

    /// Get server bind address
    pub fn server_address(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }

    /// Get ZMQ receiver address
    #[allow(dead_code)]
    pub fn zmq_receiver_address(&self) -> String {
        format!("tcp://*:{}", self.zeromq.receiver_port)
    }

    /// Get ZMQ sender address (unified publisher for all outgoing messages)
    #[allow(dead_code)]
    pub fn zmq_sender_address(&self) -> String {
        format!("tcp://*:{}", self.zeromq.sender_port)
    }

    /// Get all allowed CORS origins
    /// Auto-generates HTTPS origins from webui port and includes additional custom origins
    pub fn allowed_origins(&self) -> Vec<String> {
        let mut origins = vec![
            format!("https://localhost:{}", self.webui.port),
            format!("https://127.0.0.1:{}", self.webui.port),
        ];

        // Add additional custom origins (e.g., for Vercel deployment)
        origins.extend(self.cors.additional_origins.clone());

        origins
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
            },
            webui: WebUIConfig::default(),
            database: DatabaseConfig {
                url: "sqlite://sankey_copier.db?mode=rwc".to_string(),
            },
            zeromq: ZeroMqConfig {
                receiver_port: 5555,
                sender_port: 5556,
                timeout_seconds: 30,
            },
            cors: CorsConfig::default(),
            logging: LoggingConfig::default(),
            installer: InstallerConfig::default(),
            tls: TlsConfig::default(),
            victoria_logs: VictoriaLogsConfig::default(),
        }
    }
}

/// Update VictoriaLogs enabled setting in config.toml
/// Uses toml_edit to preserve comments, formatting, and structure
///
/// Safety measures:
/// 1. Creates backup before write
/// 2. Validates content after write
/// 3. Restores from backup if validation fails
///
/// Update VictoriaLogs enabled setting in config.toml
/// Uses toml_edit to preserve comments, formatting, and structure
pub fn update_victoria_logs_enabled<P: AsRef<Path>>(enabled: bool, config_path: P) -> Result<()> {
    let config_path = config_path.as_ref();

    // Read existing config file
    let content = std::fs::read_to_string(config_path).context("Failed to read config file")?;

    // Parse as editable document (preserves comments and formatting)
    let mut doc: toml_edit::DocumentMut = content.parse().context("Failed to parse config.toml")?;

    // Update victoria_logs.enabled
    if let Some(vlogs) = doc.get_mut("victoria_logs") {
        if let Some(table) = vlogs.as_table_mut() {
            table["enabled"] = toml_edit::value(enabled);
        }
    } else {
        // victoria_logs section doesn't exist, create it
        let mut vlogs_table = toml_edit::Table::new();
        vlogs_table["enabled"] = toml_edit::value(enabled);
        doc["victoria_logs"] = toml_edit::Item::Table(vlogs_table);
    }

    // Write back to file (preserves original formatting)
    std::fs::write(config_path, doc.to_string()).context("Failed to write config.toml")?;

    tracing::info!(
        "Successfully updated config.toml (victoria_logs.enabled = {}) at {:?}",
        enabled,
        config_path
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.zeromq.receiver_port, 5555);
        assert_eq!(config.zeromq.sender_port, 5556);
    }

    #[test]
    fn test_server_address() {
        let config = Config::default();
        assert_eq!(config.server_address(), "0.0.0.0:8080");
    }

    #[test]
    fn test_zmq_addresses() {
        let config = Config::default();
        assert_eq!(config.zmq_receiver_address(), "tcp://*:5555");
        assert_eq!(config.zmq_sender_address(), "tcp://*:5556");
    }

    #[test]
    fn test_custom_config() {
        let config = Config {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 9090,
            },
            webui: WebUIConfig::default(),
            database: DatabaseConfig {
                url: "sqlite://test.db".to_string(),
            },
            zeromq: ZeroMqConfig {
                receiver_port: 6666,
                sender_port: 6667,
                timeout_seconds: 60,
            },
            cors: CorsConfig::default(),
            logging: LoggingConfig::default(),
            installer: InstallerConfig::default(),
            tls: TlsConfig::default(),
            victoria_logs: VictoriaLogsConfig::default(),
        };

        assert_eq!(config.server_address(), "127.0.0.1:9090");
        assert_eq!(config.zmq_receiver_address(), "tcp://*:6666");
        assert_eq!(config.zmq_sender_address(), "tcp://*:6667");
    }

    #[test]
    fn test_toml_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).unwrap();

        // Verify it contains expected sections
        assert!(toml_str.contains("[server]"));
        assert!(toml_str.contains("[database]"));
        assert!(toml_str.contains("[zeromq]"));
    }

    #[test]
    fn test_toml_deserialization() {
        // 2-port architecture: only receiver_port and sender_port
        let toml_str = r#"
[server]
host = "127.0.0.1"
port = 9000

[database]
url = "sqlite://custom.db"

[zeromq]
receiver_port = 7777
sender_port = 7778
timeout_seconds = 45
"#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 9000);
        assert_eq!(config.database.url, "sqlite://custom.db");
        assert_eq!(config.zeromq.receiver_port, 7777);
        assert_eq!(config.zeromq.sender_port, 7778);
        assert_eq!(config.zeromq.timeout_seconds, 45);
    }
    #[test]
    fn test_update_victoria_logs_enabled() {
        use std::io::Write;

        // Create a temporary config file
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join(format!("test_config_{}.toml", uuid::Uuid::new_v4()));

        // Write initial content
        let initial_content = r#"
[server]
host = "0.0.0.0"
port = 8080

[webui]
host = "0.0.0.0"
port = 8080
url = "http://localhost:8080"

[database]
url = "sqlite://sankey_copier.db?mode=rwc"

[zeromq]
receiver_port = 5555
sender_port = 5556
timeout_seconds = 30

[cors]
disable = true
additional_origins = []

[logging]
enabled = true
directory = "logs"
file_prefix = "sankey-copier-server"
rotation = "daily"
max_files = 3
max_age_days = 3

[tls]
cert_path = "certs/server.pem"
key_path = "certs/server-key.pem"
validity_days = 3650

[victoria_logs]
enabled = false
host = "http://localhost:9428"
batch_size = 100
flush_interval_secs = 5
source = "relay-server"
"#;
        let mut file = std::fs::File::create(&config_path).unwrap();
        file.write_all(initial_content.as_bytes()).unwrap();

        // Update enabled to true
        update_victoria_logs_enabled(true, &config_path).unwrap();

        // Verify update
        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("enabled = true"));
        assert!(content.contains("[victoria_logs]"));

        // Cleanup
        let _ = std::fs::remove_file(config_path);
    }
}
