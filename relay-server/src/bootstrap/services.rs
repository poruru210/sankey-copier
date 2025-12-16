use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};

use crate::adapters;
use crate::adapters::inbound::zmq::MessageHandler;
use crate::adapters::infrastructure::connection_manager;
use crate::adapters::infrastructure::connection_manager::ConnectionManager;
use crate::adapters::outbound::messaging::{ZmqConfigPublisher, ZmqMessage, ZmqServer};
use crate::adapters::outbound::observability::victoria_logs::VLogsController;
use crate::adapters::outbound::persistence::Database;
use crate::application::runtime_status_updater::{RuntimeStatusMetrics, RuntimeStatusUpdater};
use crate::application::status_service::StatusService;
use crate::domain::services::copy_engine::CopyEngine;
use crate::ports;

pub struct ServiceRegistry {
    pub db: Arc<Database>,
    pub connection_manager: Arc<ConnectionManager>,
    pub config_sender: Arc<ZmqConfigPublisher>,
    pub broadcast_tx: broadcast::Sender<String>,
    pub resolved_ports: adapters::infrastructure::port_resolver::ResolvedPorts,
    pub vlogs_controller: Option<VLogsController>,
    pub runtime_status_metrics: Arc<RuntimeStatusMetrics>,
    // Add other needed fields for AppState
    pub log_buffer: crate::adapters::infrastructure::log_buffer::LogBuffer,
}

pub async fn setup(
    config: &crate::config::Config,
    log_buffer: crate::adapters::infrastructure::log_buffer::LogBuffer,
    vlogs_enabled_flag: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
) -> Result<ServiceRegistry> {
    // Determine config directory for runtime.toml
    let config_dir = std::env::var("CONFIG_DIR").unwrap_or_else(|_| {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_string_lossy().into_owned()))
            .unwrap_or_else(|| ".".to_string())
    });

    // Resolve ports
    let runtime_toml_path = format!("{}/runtime.toml", config_dir);
    // Actually config_dir logic is duplicated here.
    // Maybe better to reuse logic from mod.rs if possible, or just recompute.
    // Recomputing is safer for now.

    let resolved_ports = adapters::infrastructure::port_resolver::resolve_ports(
        &config.server,
        &config.zeromq,
        &runtime_toml_path,
    )?;

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

    // Ensure TLS certificate exists
    let base_path = std::env::current_dir()?;
    adapters::infrastructure::cert::ensure_certificate(&config.tls, &base_path)?;
    tracing::info!("TLS certificate ready");

    // Initialize database
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| config.database.url.clone());
    let db = Arc::new(Database::new(&database_url).await?);
    tracing::info!("Database initialized: {}", database_url);

    // Create VLogsController
    let vlogs_controller = vlogs_enabled_flag
        .map(|enabled_flag| VLogsController::new(enabled_flag, config.victoria_logs.clone()));

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

    // Initialize unified ZeroMQ publisher
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

        let runtime_updater = RuntimeStatusUpdater::with_metrics(
            db.clone(),
            connection_manager.clone(),
            runtime_status_metrics.clone(),
        );

        let status_service = StatusService::new(
            connection_manager.clone() as Arc<dyn ports::ConnectionManager>,
            db.clone() as Arc<dyn ports::TradeGroupRepository>,
            zmq_publisher.clone() as Arc<dyn ports::ConfigPublisher>,
            Arc::new(runtime_updater),
            Some(Arc::new(snapshot_broadcaster) as Arc<dyn ports::UpdateBroadcaster>),
            vlogs_controller
                .clone()
                .map(|c| Arc::new(c) as Arc<dyn crate::ports::outbound::VLogsConfigProvider>),
        );

        // Create WebSocket broadcaster for DisconnectionService
        let ws_broadcaster = Arc::new(
            crate::adapters::outbound::messaging::WebsocketBroadcaster::new(broadcast_tx.clone()),
        );

        // Create DisconnectionService
        let disconnection_service = Arc::new(
            crate::application::disconnection_service::RealDisconnectionService::new(
                connection_manager.clone(),
                db.clone(),
                zmq_publisher.clone(),
                ws_broadcaster.clone(),
                runtime_status_metrics.clone(),
            ),
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
            disconnection_service.clone(),
            Arc::new(config.clone()),
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
        // Re-create broadcaster/service instances for the timeout handler closure if needed?
        // Actually, we can just share the Arc if we lift the definition out of the MessageHandler block.
        // But since we are inside `setup` and moved handler into spawn, we need to create service outside or handle cloning.
        // Wait, the variables above are inside the block { ... }. They are dropped at end of block.
        // We need to move the service creation OUTSIDE the block to share it with timeout monitor.
        // OR re-create it (which is fine since components are Arcs).
        // Let's re-create broadaster/service here for simplicity OR better refactor to share.
        // Sharing is better. But blocks limit scope.
        // Let's remove the block boundaries or duplicate creation?
        // Duplicating creation logic is easy since dependencies are all Arc.

        // Re-create dependencies for TimeoutMonitor
        let ws_broadcaster = Arc::new(
            crate::adapters::outbound::messaging::WebsocketBroadcaster::new(broadcast_tx.clone()),
        );
        let disconnection_service = Arc::new(
            crate::application::disconnection_service::RealDisconnectionService::new(
                connection_manager.clone(),
                db.clone(),
                zmq_publisher.clone(),
                ws_broadcaster.clone(),
                runtime_status_metrics.clone(),
            ),
        );

        let handler = connection_manager::RealTimeoutActionHandler::new(disconnection_service);
        let monitor = connection_manager::TimeoutMonitor::new(
            connection_manager.clone(),
            std::sync::Arc::new(handler),
        );

        tokio::spawn(async move {
            monitor.run().await;
        });
        tracing::info!("Timeout checker task spawned");
    }

    Ok(ServiceRegistry {
        db,
        connection_manager,
        config_sender: zmq_publisher,
        broadcast_tx,
        resolved_ports,
        vlogs_controller,
        runtime_status_metrics,
        log_buffer,
    })
}
