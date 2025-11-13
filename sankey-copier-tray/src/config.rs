//! Configuration management for SANKEY Copier tray application.
//!
//! This module handles loading and parsing configuration from config.toml files.

use serde::Deserialize;
use std::fs;

/// Main configuration structure
#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub webui: WebUIConfig,
}

/// Web UI configuration
#[derive(Debug, Deserialize)]
pub struct WebUIConfig {
    #[serde(default = "default_webui_port")]
    pub port: u16,
}

impl Default for WebUIConfig {
    fn default() -> Self {
        Self { port: 3000 }
    }
}

fn default_webui_port() -> u16 {
    3000
}

/// Load Web UI port number from config.toml
///
/// Searches for config.toml in multiple standard locations and returns
/// the Web UI port if found, otherwise returns None.
pub fn load_port_from_config() -> Option<u16> {
    let config_paths = [
        "config.toml",
        "../config.toml",
        "C:\\Program Files\\SANKEY Copier\\config.toml",
    ];

    for path in &config_paths {
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(config) = toml::from_str::<Config>(&content) {
                return Some(config.webui.port);
            }
        }
    }

    None
}

/// Get the Web URL based on configuration
pub fn get_web_url() -> String {
    let port = load_port_from_config().unwrap_or(8080);
    // Use 127.0.0.1 instead of localhost to force IPv4
    format!("http://127.0.0.1:{}", port)
}
