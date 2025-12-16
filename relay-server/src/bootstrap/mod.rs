use anyhow::Result;
use axum::Router;
use axum_server::tls_rustls::RustlsConfig;
use std::net::SocketAddr;

pub mod logging;
pub mod server;
pub mod services;

pub struct Application {
    pub router: Router,
    pub tls_config: RustlsConfig,
    pub bind_address: String,
    pub socket_addr: SocketAddr,
}

pub async fn setup() -> Result<Application> {
    // Initialize rustls with ring crypto provider
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    // 1. Load Configuration
    let config = load_config();

    // 2. Setup Logging
    // 2. Setup Logging
    let (log_buffer, vlogs_enabled_flag) = logging::setup(&config);

    // 3. Setup Services & Background Tasks
    let service_registry =
        services::setup(&config, log_buffer.clone(), vlogs_enabled_flag.clone()).await?;

    // 4. Setup Server (API & TLS)
    server::setup(config, service_registry).await
}

fn load_config() -> crate::config::Config {
    use crate::config::Config;

    // Determine config directory
    let config_dir = std::env::var("CONFIG_DIR").unwrap_or_else(|_| {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_string_lossy().into_owned()))
            .unwrap_or_else(|| ".".to_string())
    });
    let config_base = format!("{}/config", config_dir);

    eprintln!(
        "Config directory: {}, config base: {}",
        config_dir, config_base
    );

    // Load configuration
    match Config::from_file(&config_base) {
        Ok(cfg) => {
            eprintln!("Configuration loaded successfully from {}", config_base);
            cfg
        }
        Err(e) => {
            eprintln!("Failed to load configuration: {}, using defaults", e);
            Config::default()
        }
    }
}
