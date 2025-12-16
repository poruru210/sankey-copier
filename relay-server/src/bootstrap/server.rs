use anyhow::Result;
use axum_server::tls_rustls::RustlsConfig;
use std::net::SocketAddr;
use std::sync::Arc;

use crate::adapters;
use crate::adapters::inbound::http::{create_router, AppState};
use crate::bootstrap::{services::ServiceRegistry, Application};

pub async fn setup(
    config: crate::config::Config,
    registry: ServiceRegistry,
) -> Result<Application> {
    // Update server address
    let server_address = if registry.resolved_ports.is_dynamic && config.server.port == 0 {
        format!(
            "{}:{}",
            config.server.host, registry.resolved_ports.http_port
        )
    } else {
        config.server_address()
    };

    tracing::info!("Server will listen on: {}", server_address);

    // Create API state
    tracing::info!("Creating API state...");
    let allowed_origins = config.allowed_origins();
    let cors_disabled = config.cors.disable;

    // Create on-demand snapshot broadcaster for API state (Separate instance? Yes, usage pattern seems so in original code)
    // Actually the original code created a new SnapshotBroadcaster for AppState.
    let snapshot_broadcaster = adapters::inbound::http::SnapshotBroadcaster::new(
        registry.broadcast_tx.clone(),
        registry.connection_manager.clone(),
        registry.db.clone(),
    );

    let app_state = AppState {
        db: registry.db,
        tx: registry.broadcast_tx,
        connection_manager: registry.connection_manager,
        config_sender: registry.config_sender,
        log_buffer: registry.log_buffer,
        allowed_origins: allowed_origins.clone(),
        cors_disabled,
        config: Arc::new(config.clone()),
        resolved_ports: Arc::new(registry.resolved_ports),
        vlogs_controller: registry.vlogs_controller,
        runtime_status_metrics: registry.runtime_status_metrics,
        snapshot_broadcaster,
    };

    if cors_disabled {
        tracing::warn!("CORS is DISABLED in config - all origins will be allowed!");
    } else {
        tracing::info!("API state created with CORS origins: {:?}", allowed_origins);
    }

    // Build API router
    tracing::info!("Building API router...");
    let app = create_router(app_state);
    tracing::info!("API router built");

    // Load TLS certificate and key
    let base_path = std::env::current_dir()?;
    let cert_path = base_path.join(&config.tls.cert_path);
    let key_path = base_path.join(&config.tls.key_path);

    let tls_config = match RustlsConfig::from_pem_file(&cert_path, &key_path).await {
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
    let socket_addr: SocketAddr = server_address
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid bind address '{}': {}", server_address, e))?;

    Ok(Application {
        router: app,
        tls_config,
        bind_address: server_address,
        socket_addr,
    })
}
