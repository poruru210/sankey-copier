use anyhow::{Context, Result};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZeroMqConfig {
    pub receiver_port: u16,
    pub sender_port: u16,
    pub config_sender_port: u16,
    pub timeout_seconds: i64,
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
    pub fn zmq_receiver_address(&self) -> String {
        format!("tcp://*:{}", self.zeromq.receiver_port)
    }

    /// Get ZMQ sender address
    pub fn zmq_sender_address(&self) -> String {
        format!("tcp://*:{}", self.zeromq.sender_port)
    }

    /// Get ZMQ config sender address
    pub fn zmq_config_sender_address(&self) -> String {
        format!("tcp://*:{}", self.zeromq.config_sender_port)
    }

    /// Get all allowed CORS origins
    /// Auto-generates origins from webui port and includes additional custom origins
    pub fn allowed_origins(&self) -> Vec<String> {
        let mut origins = vec![
            format!("http://localhost:{}", self.webui.port),
            format!("http://127.0.0.1:{}", self.webui.port),
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
                config_sender_port: 5557,
                timeout_seconds: 30,
            },
            cors: CorsConfig::default(),
            logging: LoggingConfig::default(),
            installer: InstallerConfig::default(),
        }
    }
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
        assert_eq!(config.zeromq.config_sender_port, 5557);
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
        assert_eq!(config.zmq_config_sender_address(), "tcp://*:5557");
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
                config_sender_port: 6668,
                timeout_seconds: 60,
            },
            cors: CorsConfig::default(),
            logging: LoggingConfig::default(),
            installer: InstallerConfig::default(),
        };

        assert_eq!(config.server_address(), "127.0.0.1:9090");
        assert_eq!(config.zmq_receiver_address(), "tcp://*:6666");
        assert_eq!(config.zmq_sender_address(), "tcp://*:6667");
        assert_eq!(config.zmq_config_sender_address(), "tcp://*:6668");
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
        let toml_str = r#"
[server]
host = "127.0.0.1"
port = 9000

[database]
url = "sqlite://custom.db"

[zeromq]
receiver_port = 7777
sender_port = 7778
config_sender_port = 7779
timeout_seconds = 45
"#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 9000);
        assert_eq!(config.database.url, "sqlite://custom.db");
        assert_eq!(config.zeromq.receiver_port, 7777);
        assert_eq!(config.zeromq.timeout_seconds, 45);
    }
}
