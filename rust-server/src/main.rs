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
use config::Config;
use connection_manager::ConnectionManager;
use db::Database;
use engine::CopyEngine;
use log_buffer::{create_log_buffer, LogBufferLayer};
use message_handler::MessageHandler;
use zeromq::{ZmqMessage, ZmqSender, ZmqServer, ZmqConfigPublisher};

#[tokio::main]
async fn main() -> Result<()> {
    // Create log buffer
    let log_buffer = create_log_buffer();

    // Initialize logging with log buffer layer
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sankey_copier_server=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .with(LogBufferLayer::new(log_buffer.clone()))
        .init();

    tracing::info!("Starting SANKEY Copier Server...");
    tracing::info!("Server Version: {}", env!("BUILD_INFO"));

    // Load configuration
    let config = match Config::from_file("config.toml") {
        Ok(cfg) => {
            tracing::info!("Loaded configuration from config.toml");
            cfg
        }
        Err(e) => {
            tracing::warn!("Failed to load config.toml: {}, using defaults", e);
            Config::default()
        }
    };

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
    let app_state = AppState {
        db: db.clone(),
        tx: broadcast_tx,
        settings_cache: settings_cache.clone(),
        connection_manager: connection_manager.clone(),
        config_sender: zmq_config_sender.clone(),
        log_buffer: log_buffer.clone(),
    };
    tracing::info!("API state created");

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
