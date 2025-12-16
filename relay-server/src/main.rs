use anyhow::Result;
use sankey_copier_relay_server::adapters;
use sankey_copier_relay_server::adapters::inbound::http::{create_router, AppState};
use sankey_copier_relay_server::adapters::inbound::zmq::MessageHandler;
use sankey_copier_relay_server::adapters::infrastructure::connection_manager;
use sankey_copier_relay_server::adapters::infrastructure::connection_manager::ConnectionManager;
use sankey_copier_relay_server::adapters::infrastructure::log_buffer::create_log_buffer;
use sankey_copier_relay_server::adapters::outbound::messaging::{
    ZmqConfigPublisher, ZmqMessage, ZmqServer,
};
use sankey_copier_relay_server::adapters::outbound::persistence::Database;
use sankey_copier_relay_server::application::status_service::StatusService;
use sankey_copier_relay_server::config::Config;
use sankey_copier_relay_server::domain::services::copy_engine::CopyEngine;
use sankey_copier_relay_server::logging;
use sankey_copier_relay_server::ports;
use sankey_copier_relay_server::ports::adapters::RuntimeStatusEvaluatorAdapter;
use sankey_copier_relay_server::runtime_status_updater::{
    RuntimeStatusMetrics, RuntimeStatusUpdater,
};
use sankey_copier_relay_server::victoria_logs;
use sankey_copier_relay_server::victoria_logs::VLogsController;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};

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

    // Initialize VictoriaLogs
    let (vlogs_layer, _vlogs_handles, vlogs_enabled_flag) =
        victoria_logs::init(&config.victoria_logs);

    // Initialize logging (console + file + log buffer + VictoriaLogs)
    logging::init(&config.logging, log_buffer.clone(), vlogs_layer);

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

    if let Some(enabled) = &vlogs_enabled_flag {
        tracing::info!(
            "VictoriaLogs configured: host={}, batch_size={}, flush_interval={}s, initial_enabled={}",
            config.victoria_logs.host,
            config.victoria_logs.batch_size,
            config.victoria_logs.flush_interval_secs,
            enabled.load(std::sync::atomic::Ordering::Relaxed)
        );
    }

    // Resolve ports (HTTP and ZeroMQ, dynamic or fixed)
    // runtime.toml is stored in CONFIG_DIR (or current directory)
    let runtime_toml_path = format!("{}/runtime.toml", config_dir);
    let resolved_ports = adapters::infrastructure::port_resolver::resolve_ports(
        &config.server,
        &config.zeromq,
        &runtime_toml_path,
    )?;

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
    adapters::infrastructure::cert::ensure_certificate(&config.tls, &base_path)?;
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
    let zmq_publisher = Arc::new(ZmqConfigPublisher::new(&resolved_ports.sender_address())?);
    tracing::info!(
        "ZeroMQ unified publisher started on {}",
        resolved_ports.sender_address()
    );

    // Initialize copy engine
    let copy_engine = Arc::new(CopyEngine::new());

    let runtime_status_metrics = Arc::new(RuntimeStatusMetrics::default());

    // Spawn ZeroMQ message processing task
    tracing::info!("Creating MessageHandler...");
    {
        // Construct StatusService with Hexagonal adapters
        let snapshot_broadcaster = adapters::inbound::http::SnapshotBroadcaster::new(
            broadcast_tx.clone(),
            connection_manager.clone(),
            db.clone(),
        );

        let status_evaluator =
            RuntimeStatusEvaluatorAdapter::new(RuntimeStatusUpdater::with_metrics(
                db.clone(),
                connection_manager.clone(),
                runtime_status_metrics.clone(),
            ));

        let status_service = StatusService::new(
            connection_manager.clone() as Arc<dyn ports::ConnectionManager>,
            db.clone() as Arc<dyn ports::TradeGroupRepository>,
            zmq_publisher.clone() as Arc<dyn ports::ConfigPublisher>,
            Some(Arc::new(status_evaluator) as Arc<dyn ports::StatusEvaluator>),
            Some(Arc::new(snapshot_broadcaster) as Arc<dyn ports::UpdateBroadcaster>),
            vlogs_controller
                .clone()
                .map(|c| Arc::new(c) as Arc<dyn crate::ports::outbound::VLogsConfigProvider>),
        );

        let handler = MessageHandler::new(
            connection_manager.clone(),
            copy_engine.clone(),
            broadcast_tx.clone(),
            db.clone(),
            zmq_publisher.clone(),
            vlogs_controller.clone(),
            runtime_status_metrics.clone(),
            status_service,
        );
        tracing::info!(
            "MessageHandler created with StatusService, spawning message processing task..."
        );

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
        let handler = connection_manager::RealTimeoutActionHandler::new(
            connection_manager.clone(),
            db.clone(),
            zmq_publisher.clone(),
            broadcast_tx.clone(),
            runtime_status_metrics.clone(),
        );
        let monitor = connection_manager::TimeoutMonitor::new(
            connection_manager.clone(),
            std::sync::Arc::new(handler),
        );

        tokio::spawn(async move {
            monitor.run().await;
        });
        tracing::info!("Timeout checker task spawned");
    }

    // Create API state
    tracing::info!("Creating API state...");
    let allowed_origins = config.allowed_origins();
    let cors_disabled = config.cors.disable;

    // Create on-demand snapshot broadcaster for WebSocket clients
    let snapshot_broadcaster = adapters::inbound::http::SnapshotBroadcaster::new(
        broadcast_tx.clone(),
        connection_manager.clone(),
        db.clone(),
    );

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
        snapshot_broadcaster,
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
