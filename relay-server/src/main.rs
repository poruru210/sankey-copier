mod api;
mod cert;
mod config;
mod config_builder;
mod connection_manager;
mod db;
mod engine;
mod log_buffer;
mod message_handler;
mod models;
mod mt_detector;
mod mt_installer;
mod port_resolver;
mod runtime_status_updater;
mod victoria_logs;
mod zeromq;

use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use chrono::TimeZone;
use tokio::sync::{broadcast, mpsc};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use api::{create_router, AppState};
use config::{Config, LoggingConfig};
use connection_manager::ConnectionManager;
use db::Database;
use engine::CopyEngine;
use log_buffer::{create_log_buffer, LogBufferLayer};
use message_handler::unregister::{notify_slave_offline, notify_slaves_master_offline};
use message_handler::MessageHandler;
use models::EaType;
use runtime_status_updater::RuntimeStatusMetrics;
use std::sync::atomic::AtomicBool;
use victoria_logs::VLogsController;
use zeromq::{ZmqConfigPublisher, ZmqMessage, ZmqServer, SendFailure};

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
                entry
                    .file_name()
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
    // Initialize rustls with ring crypto provider
    // This must be done before any TLS operations
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    // Determine config directory from CONFIG_DIR environment variable
    // If CONFIG_DIR is set, use that directory
    // Otherwise, use the directory containing the executable (for Windows service support)
    // Fallback to current directory if executable path cannot be determined
    let config_dir = std::env::var("CONFIG_DIR").unwrap_or_else(|_| {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_string_lossy().into_owned()))
            .unwrap_or_else(|| ".".to_string())
    });
    let config_base = format!("{}/config", config_dir);

    // Log the resolved config path for debugging
    eprintln!(
        "Config directory: {}, config base: {}",
        config_dir, config_base
    );

    // Load configuration first (needed for file logging setup)
    // Loads config.toml, config.dev.toml, and config.local.toml (if they exist)
    let config = match Config::from_file(&config_base) {
        Ok(cfg) => {
            eprintln!("Configuration loaded successfully from {}", config_base);
            cfg
        }
        Err(e) => {
            eprintln!("Failed to load configuration: {}, using defaults", e);
            Config::default()
        }
    };

    // Create log buffer
    let log_buffer = create_log_buffer();

    // Create VictoriaLogs layer if configured (endpoint is not empty)
    // The layer is controlled by a shared Arc<AtomicBool> for runtime enable/disable.
    // _vlogs_handle can be used for graceful shutdown (flush remaining logs).
    // vlogs_enabled_flag is shared with VLogsController for runtime toggle.
    let (vlogs_layer, _vlogs_handle, vlogs_enabled_flag) = if !config.victoria_logs.host.is_empty()
    {
        // Create shared enabled flag - initially from config.toml
        // Will be updated from DB after DB initialization
        let enabled_flag = Arc::new(AtomicBool::new(config.victoria_logs.enabled));
        let (layer, handle) = victoria_logs::VictoriaLogsLayer::new_with_enabled(
            &config.victoria_logs,
            enabled_flag.clone(),
        );
        let vlogs_handle = victoria_logs::VictoriaLogsHandle::new(&layer);
        (
            Some(layer),
            Some((vlogs_handle, handle)),
            Some(enabled_flag),
        )
    } else {
        (None, None, None)
    };

    // Initialize logging with log buffer layer and optional file output
    // Default to info level for all modules; can be overridden via RUST_LOG env var
    let env_filter =
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());

    let subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .with(LogBufferLayer::new(log_buffer.clone()))
        .with(vlogs_layer);

    // Add file logging layer if enabled in config
    if config.logging.enabled {
        use std::fs;
        use tracing_appender::rolling;

        // Create log directory if it doesn't exist
        if let Err(e) = fs::create_dir_all(&config.logging.directory) {
            eprintln!(
                "Failed to create log directory {}: {}",
                config.logging.directory, e
            );
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
            .with(
                tracing_subscriber::fmt::layer()
                    .with_writer(non_blocking)
                    .with_ansi(false),
            ) // Disable ANSI colors in file output
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

    if vlogs_enabled_flag.is_some() {
        tracing::info!(
            "VictoriaLogs configured: host={}, batch_size={}, flush_interval={}s, initial_enabled={}",
            config.victoria_logs.host,
            config.victoria_logs.batch_size,
            config.victoria_logs.flush_interval_secs,
            config.victoria_logs.enabled
        );
    }

    // Resolve ports (HTTP and ZeroMQ, dynamic or fixed)
    // runtime.toml is stored in CONFIG_DIR (or current directory)
    let runtime_toml_path = format!("{}/runtime.toml", config_dir);
    let resolved_ports =
        port_resolver::resolve_ports(&config.server, &config.zeromq, &runtime_toml_path)?;

    // Update server address if port was dynamically assigned
    let server_address = if resolved_ports.is_dynamic && config.server.port == 0 {
        format!("{}:{}", config.server.host, resolved_ports.http_port)
    } else {
        config.server_address()
    };

    tracing::info!("Server will listen on: {}", server_address);
    tracing::info!(
        "ZMQ Receiver: {} (port {})",
        resolved_ports.receiver_address(),
        resolved_ports.receiver_port
    );
    tracing::info!(
        "ZMQ Sender (unified): {} (port {})",
        resolved_ports.sender_address(),
        resolved_ports.sender_port
    );
    if resolved_ports.is_dynamic {
        tracing::info!(
            "Ports are dynamically assigned (generated_at: {:?})",
            resolved_ports.generated_at
        );
    }

    // Ensure TLS certificate exists (generate and register if needed)
    let base_path = std::env::current_dir()?;
    cert::ensure_certificate(&config.tls, &base_path)?;
    tracing::info!("TLS certificate ready");

    // Initialize database
    // DATABASE_URL environment variable overrides config.toml setting
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| config.database.url.clone());
    let db = Arc::new(Database::new(&database_url).await?);
    tracing::info!("Database initialized: {}", database_url);

    // Create VLogsController if VictoriaLogs is configured
    // Uses config.toml settings directly (no DB override)
    let vlogs_controller = if let Some(enabled_flag) = vlogs_enabled_flag {
        tracing::info!(
            "VictoriaLogs configured: enabled={} (from config.toml)",
            config.victoria_logs.enabled
        );
        Some(VLogsController::new(
            enabled_flag,
            config.victoria_logs.clone(),
        ))
    } else {
        tracing::info!("VictoriaLogs not configured (host is empty)");
        None
    };

    // Initialize ConnectionManager
    let connection_manager = Arc::new(ConnectionManager::new(config.zeromq.timeout_seconds));
    tracing::info!(
        "Connection manager initialized with {}s timeout",
        config.zeromq.timeout_seconds
    );

    // Create channels
    let (zmq_tx, mut zmq_rx) = mpsc::unbounded_channel::<ZmqMessage>();
    let (broadcast_tx, _) = broadcast::channel::<String>(100);

    // Initialize ZeroMQ server
    let zmq_server = ZmqServer::new(zmq_tx)?;
    zmq_server
        .start_receiver(&resolved_ports.receiver_address())
        .await?;
    tracing::info!(
        "ZeroMQ receiver started on {}",
        resolved_ports.receiver_address()
    );

    // Initialize unified ZeroMQ publisher (PUB socket for all outgoing messages)
    // 2-port architecture: single publisher handles both trade signals and config messages
    // Create a failure channel to persist send failures into the database
    let (failure_tx, mut failure_rx) = mpsc::unbounded_channel::<SendFailure>();

    let zmq_publisher = Arc::new(
        ZmqConfigPublisher::new_with_failure_sender(&resolved_ports.sender_address(), failure_tx)?,
    );
    tracing::info!(
        "ZeroMQ unified publisher started on {}",
        resolved_ports.sender_address()
    );

    // Spawn background task that persists failed send notifications to DB
    {
        let db_clone = db.clone();
        tokio::spawn(async move {
            while let Some(failure) = failure_rx.recv().await {
                match db_clone
                    .record_failed_send(&failure.topic, &failure.payload, &failure.error, failure.attempts)
                    .await
                {
                    Ok(id) => tracing::info!("Persisted failed ZMQ send id={} topic={}", id, failure.topic),
                    Err(e) => tracing::error!("Failed to persist ZMQ send failure: {}", e),
                }
            }
        });
    }

    // Spawn retry worker to re-send persisted failed messages
    tracing::info!("Spawning failed-send retry worker...");
    {
        let db_clone = db.clone();
        let publisher_clone = zmq_publisher.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            loop {
                interval.tick().await;

                // Backoff and dead-letter policy
                const MAX_RETRY_ATTEMPTS: i32 = 5;
                const BASE_BACKOFF_SECS: u64 = 1;
                const MAX_BACKOFF_SECS: u64 = 60;

                match db_clone.fetch_pending_failed_sends(100).await {
                    Ok(items) if !items.is_empty() => {
                        for (id, topic, payload, attempts, updated_at) in items {
                            // If we've reached max attempts, archive to dead-letter storage
                            if attempts >= MAX_RETRY_ATTEMPTS {
                                if let Ok(rows) = db_clone.move_failed_to_dead_letter(id).await {
                                    if rows > 0 {
                                        tracing::warn!("Moved failed send id={} topic={} to dead-letter after attempts={}", id, topic, attempts);
                                    }
                                }
                                continue;
                            }

                            // Parse updated_at (SQLite CURRENT_TIMESTAMP format: "YYYY-MM-DD HH:MM:SS")
                            let ok_to_try = if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(&updated_at, "%Y-%m-%d %H:%M:%S") {
                                let last = chrono::Utc.from_utc_datetime(&naive);
                                let elapsed_secs = chrono::Utc::now().signed_duration_since(last).num_seconds();
                                // exponential backoff: BASE * 2^(attempts)
                                let pow = attempts.max(0) as u32;
                                let factor = 2u64.pow(pow);
                                let mut backoff = BASE_BACKOFF_SECS.saturating_mul(factor);
                                if backoff > MAX_BACKOFF_SECS { backoff = MAX_BACKOFF_SECS; }
                                elapsed_secs >= backoff as i64
                            } else {
                                // If parsing fails, be permissive and try
                                true
                            };

                            if !ok_to_try {
                                continue;
                            }

                            // Try to resend preserved MessagePack payload
                            match publisher_clone.publish_raw(&topic, &payload).await {
                                Ok(_) => {
                                    if let Ok(rows) = db_clone.mark_failed_send_processed(id).await {
                                        if rows > 0 {
                                            tracing::info!("Retried and cleared failed send id={} topic={} attempts={}", id, topic, attempts);
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!("Retrying failed send id={} topic={} failed: {}", id, topic, e);
                                    // Record that we attempted another retry
                                    if let Err(err) = db_clone.increment_failed_send_attempts(id).await {
                                        tracing::error!("Failed to increment retry attempts for id={} : {}", id, err);
                                    }
                                }
                            }
                        }
                    }
                    Ok(_) => {
                        // nothing to do
                    }
                    Err(e) => tracing::error!("Failed to fetch pending failed sends: {}", e),
                }
            }
        });
    }

    // Initialize copy engine
    let copy_engine = Arc::new(CopyEngine::new());

    let runtime_status_metrics = Arc::new(RuntimeStatusMetrics::default());

    // Spawn ZeroMQ message processing task
    tracing::info!("Creating MessageHandler...");
    {
        let handler = MessageHandler::new(
            connection_manager.clone(),
            copy_engine.clone(),
            broadcast_tx.clone(),
            db.clone(),
            zmq_publisher.clone(),
            vlogs_controller.clone(),
            runtime_status_metrics.clone(),
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
        let db_clone = db.clone();
        let publisher_clone = zmq_publisher.clone();
        let broadcast_clone = broadcast_tx.clone();
        let metrics_clone = runtime_status_metrics.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            loop {
                interval.tick().await;
                let timed_out = conn_mgr.check_timeouts().await;

                // Update database statuses for timed-out EAs
                for (account_id, ea_type) in timed_out {
                    match ea_type {
                        EaType::Master => {
                            match db_clone
                                .update_master_statuses_disconnected(&account_id)
                                .await
                            {
                                Ok(count) if count > 0 => {
                                    tracing::info!(
                                        "Master {} timed out: updated {} settings to ENABLED",
                                        account_id,
                                        count
                                    );
                                }
                                Ok(_) => {
                                    // No settings updated
                                }
                                Err(e) => {
                                    tracing::error!(
                                        "Failed to update master statuses for {}: {}",
                                        account_id,
                                        e
                                    );
                                }
                            }

                            notify_slaves_master_offline(
                                &conn_mgr,
                                &db_clone,
                                &publisher_clone,
                                &broadcast_clone,
                                metrics_clone.clone(),
                                &account_id,
                            )
                            .await;
                        }
                        EaType::Slave => {
                            // Slave timed out - update runtime status and notify WebSocket
                            notify_slave_offline(
                                &conn_mgr,
                                &db_clone,
                                &broadcast_clone,
                                metrics_clone.clone(),
                                &account_id,
                            )
                            .await;
                        }
                    }
                }
            }
        });
        tracing::info!("Timeout checker task spawned");
    }

    // Create API state
    tracing::info!("Creating API state...");
    let allowed_origins = config.allowed_origins();
    let cors_disabled = config.cors.disable;
    let app_state = AppState {
        db: db.clone(),
        tx: broadcast_tx,
        connection_manager: connection_manager.clone(),
        config_sender: zmq_publisher.clone(),
        log_buffer: log_buffer.clone(),
        allowed_origins: allowed_origins.clone(),
        cors_disabled,
        config: Arc::new(config.clone()),
        resolved_ports: Arc::new(resolved_ports),
        vlogs_controller,
        runtime_status_metrics,
    };
    if cors_disabled {
        tracing::warn!("CORS is DISABLED in config - all origins will be allowed!");
    } else {
        tracing::info!(
            "API state created with CORS origins (auto-generated from webui port {}): {:?}",
            config.webui.port,
            allowed_origins
        );
    }

    // Build API router
    tracing::info!("Building API router...");
    let app = create_router(app_state);
    tracing::info!("API router built");

    // Start HTTPS server
    tracing::info!("Getting bind address...");
    // Use the resolved address (which handles dynamic port assignment)
    let bind_address = server_address;
    tracing::info!("Bind address: {}, loading TLS certificate...", bind_address);

    // Load TLS certificate and key
    let cert_path = base_path.join(&config.tls.cert_path);
    let key_path = base_path.join(&config.tls.key_path);

    let rustls_config =
        match axum_server::tls_rustls::RustlsConfig::from_pem_file(&cert_path, &key_path).await {
            Ok(config) => {
                tracing::info!("TLS configuration loaded successfully");
                config
            }
            Err(e) => {
                tracing::error!("Failed to load TLS certificate: {}", e);
                return Err(e.into());
            }
        };

    // Parse bind address
    let addr: std::net::SocketAddr = bind_address
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid bind address '{}': {}", bind_address, e))?;

    tracing::info!("HTTPS server listening on https://{}", bind_address);

    axum_server::bind_rustls(addr, rustls_config)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
