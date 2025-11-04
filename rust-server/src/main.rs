mod api;
mod config;
mod connection_manager;
mod db;
mod engine;
mod message_handler;
mod models;
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
use message_handler::MessageHandler;
use zeromq::{ZmqMessage, ZmqSender, ZmqServer, ZmqConfigSender};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "forex_copier_server=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Forex Copier Server...");

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

    // Initialize ZeroMQ config sender (PUB socket)
    let zmq_config_sender = Arc::new(ZmqConfigSender::new(&config.zmq_config_sender_address())?);

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
    {
        let handler = MessageHandler::new(
            connection_manager.clone(),
            copy_engine.clone(),
            zmq_sender.clone(),
            settings_cache.clone(),
            broadcast_tx.clone(),
        );

        tokio::spawn(async move {
            while let Some(msg) = zmq_rx.recv().await {
                handler.handle_message(msg).await;
            }
        });
    }

    // Spawn timeout checker task
    {
        let conn_mgr = connection_manager.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            loop {
                interval.tick().await;
                conn_mgr.check_timeouts().await;
            }
        });
    }

    // Create API state
    let app_state = AppState {
        db: db.clone(),
        tx: broadcast_tx,
        settings_cache: settings_cache.clone(),
        connection_manager: connection_manager.clone(),
        config_sender: zmq_config_sender.clone(),
    };

    // Build API router
    let app = create_router(app_state);

    // Start HTTP server
    let bind_address = config.server_address();
    let listener = tokio::net::TcpListener::bind(&bind_address).await?;
    tracing::info!("HTTP server listening on http://{}", bind_address);

    axum::serve(listener, app).await?;

    Ok(())
}
