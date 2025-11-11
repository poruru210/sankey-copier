use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    /// Disable CORS restrictions (allows all origins) - use only in development!
    #[serde(default)]
    pub disable: bool,
    #[serde(default)]
    pub additional_origins: Vec<String>,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            disable: false,
            additional_origins: vec![],
        }
    }
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
    /// Load config from TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path.as_ref())
            .context(format!("Failed to read config file: {:?}", path.as_ref()))?;

        let config: Config = toml::from_str(&content)
            .context("Failed to parse config file")?;

        Ok(config)
    }

    /// Create default config
    pub fn default() -> Self {
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
        }
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
