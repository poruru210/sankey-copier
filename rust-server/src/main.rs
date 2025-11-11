mod api;
mod config;
mod connection_manager;
mod db;
mod engine;
mod log_buffer;
mod message_handler;
mod models;
mod mt_detector;
mod mt_installer;
mod zeromq;

use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, RwLock};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use api::{create_router, AppState};
use config::{Config, LoggingConfig};
use connection_manager::ConnectionManager;
use db::Database;
use engine::CopyEngine;
use log_buffer::{create_log_buffer, LogBufferLayer};
use message_handler::MessageHandler;
use zeromq::{ZmqMessage, ZmqSender, ZmqServer, ZmqConfigPublisher};

/// Clean up old log files based on retention policy
fn cleanup_old_logs(logging_config: &LoggingConfig) {
    use std::fs;
    use std::time::SystemTime;

    // Skip cleanup if both max_files and max_age_days are 0 (unlimited)
    if logging_config.max_files == 0 && logging_config.max_age_days == 0 {
        return;
    }

    let log_dir = std::path::Path::new(&logging_config.directory);
    if !log_dir.exists() {
        return;
    }

    // Read all files in the log directory
    let mut log_files: Vec<_> = match fs::read_dir(log_dir) {
        Ok(entries) => entries
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                // Only consider files that start with the configured prefix
                entry.file_name()
                    .to_str()
                    .map(|name| name.starts_with(&logging_config.file_prefix))
                    .unwrap_or(false)
            })
            .filter_map(|entry| {
                // Get file metadata and modified time
                let metadata = entry.metadata().ok()?;
                let modified = metadata.modified().ok()?;
                Some((entry.path(), modified))
            })
            .collect(),
        Err(e) => {
            eprintln!("Failed to read log directory: {}", e);
            return;
        }
    };

    // Sort by modified time (newest first)
    log_files.sort_by(|a, b| b.1.cmp(&a.1));

    let now = SystemTime::now();
    let max_age_duration = Duration::from_secs((logging_config.max_age_days as u64) * 24 * 60 * 60);
    let mut deleted_count = 0;

    // Delete old files based on retention policy
    for (idx, (path, modified)) in log_files.iter().enumerate() {
        let mut should_delete = false;

        // Check if exceeds max file count
        if logging_config.max_files > 0 && idx >= logging_config.max_files as usize {
            should_delete = true;
        }

        // Check if exceeds max age
        if logging_config.max_age_days > 0 {
            if let Ok(age) = now.duration_since(*modified) {
                if age > max_age_duration {
                    should_delete = true;
                }
            }
        }

        if should_delete {
            match fs::remove_file(path) {
                Ok(_) => {
                    deleted_count += 1;
                    eprintln!("Deleted old log file: {:?}", path);
                }
                Err(e) => {
                    eprintln!("Failed to delete log file {:?}: {}", path, e);
                }
            }
        }
    }

    if deleted_count > 0 {
        eprintln!("Cleaned up {} old log file(s)", deleted_count);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration first (needed for file logging setup)
    let config = match Config::from_file("config.toml") {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Failed to load config.toml: {}, using defaults", e);
            Config::default()
        }
    };

    // Create log buffer
    let log_buffer = create_log_buffer();

    // Initialize logging with log buffer layer and optional file output
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "sankey_copier_server=debug,tower_http=debug".into());

    let subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .with(LogBufferLayer::new(log_buffer.clone()));

    // Add file logging layer if enabled in config
    if config.logging.enabled {
        use std::fs;
        use tracing_appender::rolling;

        // Create log directory if it doesn't exist
        if let Err(e) = fs::create_dir_all(&config.logging.directory) {
            eprintln!("Failed to create log directory {}: {}", config.logging.directory, e);
        }

        // Clean up old log files based on retention policy
        cleanup_old_logs(&config.logging);

        // Create file appender based on rotation strategy
        let file_appender = match config.logging.rotation.as_str() {
            "hourly" => rolling::hourly(&config.logging.directory, &config.logging.file_prefix),
            "never" => rolling::never(&config.logging.directory, &config.logging.file_prefix),
            _ => rolling::daily(&config.logging.directory, &config.logging.file_prefix), // default to daily
        };

        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

        subscriber
            .with(tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)) // Disable ANSI colors in file output
            .init();

        // Store guard to prevent it from being dropped
        // In a real application, you'd want to keep this alive for the entire program lifetime
        std::mem::forget(_guard);
    } else {
        subscriber.init();
    }

    tracing::info!("Starting SANKEY Copier Server...");
    tracing::info!("Server Version: {}", env!("BUILD_INFO"));
    tracing::info!("Loaded configuration from config.toml");

    if config.logging.enabled {
        tracing::info!(
            "File logging enabled: directory={}, prefix={}, rotation={}",
            config.logging.directory,
            config.logging.file_prefix,
            config.logging.rotation
        );
    }

    tracing::info!("Server will listen on: {}", config.server_address());
    tracing::info!("ZMQ Receiver: {}", config.zmq_receiver_address());
    tracing::info!("ZMQ Sender: {}", config.zmq_sender_address());
    tracing::info!("ZMQ Config Sender: {}", config.zmq_config_sender_address());

    // Initialize database
    let db = Arc::new(Database::new(&config.database.url).await?);
    tracing::info!("Database initialized");

    // Initialize ConnectionManager
    let connection_manager = Arc::new(ConnectionManager::new(config.zeromq.timeout_seconds));
    tracing::info!("Connection manager initialized with {}s timeout", config.zeromq.timeout_seconds);

    // Create channels
    let (zmq_tx, mut zmq_rx) = mpsc::unbounded_channel::<ZmqMessage>();
    let (broadcast_tx, _) = broadcast::channel::<String>(100);

    // Initialize ZeroMQ server
    let zmq_server = ZmqServer::new(zmq_tx)?;
    zmq_server.start_receiver(&config.zmq_receiver_address()).await?;
    tracing::info!("ZeroMQ receiver started on {}", config.zmq_receiver_address());

    // Initialize ZeroMQ sender (PUB socket)
    let zmq_sender = Arc::new(ZmqSender::new(&config.zmq_sender_address())?);

    // Initialize ZeroMQ config sender (PUB socket with MessagePack)
    let zmq_config_sender = Arc::new(ZmqConfigPublisher::new(&config.zmq_config_sender_address())?);

    // Initialize copy engine
    let copy_engine = Arc::new(CopyEngine::new());

    // Settings cache
    let settings_cache = Arc::new(RwLock::new(Vec::new()));

    // Load initial settings
    {
        let settings = db.list_copy_settings().await?;
        *settings_cache.write().await = settings;
        tracing::info!("Loaded {} copy settings", settings_cache.read().await.len());
    }

    // Spawn ZeroMQ message processing task
    tracing::info!("Creating MessageHandler...");
    {
        let handler = MessageHandler::new(
            connection_manager.clone(),
            copy_engine.clone(),
            zmq_sender.clone(),
            settings_cache.clone(),
            broadcast_tx.clone(),
            db.clone(),
            zmq_config_sender.clone(),
        );
        tracing::info!("MessageHandler created, spawning message processing task...");

        tokio::spawn(async move {
            while let Some(msg) = zmq_rx.recv().await {
                handler.handle_message(msg).await;
            }
        });
        tracing::info!("Message processing task spawned");
    }

    // Spawn timeout checker task
    tracing::info!("Spawning timeout checker task...");
    {
        let conn_mgr = connection_manager.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            loop {
                interval.tick().await;
                conn_mgr.check_timeouts().await;
            }
        });
        tracing::info!("Timeout checker task spawned");
    }

    // Create API state
    tracing::info!("Creating API state...");
    let allowed_origins = config.allowed_origins();
    let app_state = AppState {
        db: db.clone(),
        tx: broadcast_tx,
        settings_cache: settings_cache.clone(),
        connection_manager: connection_manager.clone(),
        config_sender: zmq_config_sender.clone(),
        log_buffer: log_buffer.clone(),
        allowed_origins: allowed_origins.clone(),
    };
    tracing::info!("API state created with CORS origins (auto-generated from webui port {}): {:?}", config.webui.port, allowed_origins);

    // Build API router
    tracing::info!("Building API router...");
    let app = create_router(app_state);
    tracing::info!("API router built");

    // Start HTTP server
    tracing::info!("Getting bind address...");
    let bind_address = config.server_address();
    tracing::info!("Bind address: {}, attempting to bind with 5 second timeout...", bind_address);

    let listener = match tokio::time::timeout(
        Duration::from_secs(5),
        tokio::net::TcpListener::bind(&bind_address)
    ).await {
        Ok(Ok(listener)) => {
            tracing::info!("Successfully bound to {}", bind_address);
            listener
        }
        Ok(Err(e)) => {
            tracing::error!("Failed to bind: {}", e);
            return Err(e.into());
        }
        Err(_) => {
            tracing::error!("Bind operation timed out after 5 seconds");
            anyhow::bail!("Failed to bind to {}: timeout", bind_address);
        }
    };

    tracing::info!("HTTP server listening on http://{}", bind_address);

    axum::serve(listener, app).await?;

    Ok(())
}
