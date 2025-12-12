//! Relay Server REST API module
//!
//! Provides REST API endpoints for managing copy settings, EA connections,
//! logs, and WebSocket real-time updates. Includes CORS configuration,
//! request tracing, and Private Network Access (PNA) headers.

// Existing submodules (not modified)
mod error;
mod mt_installations;
mod trade_group_members;
pub mod trade_groups;

// New submodules for modular structure
mod connections;
mod logs;
mod middleware;
mod runtime_metrics;
mod victoria_logs_settings;
mod websocket;
mod zeromq_settings;

#[cfg(test)]
mod tests;

// Public re-exports
pub use error::ProblemDetails;
pub use middleware::*;

use axum::{
    middleware as axum_middleware,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tower_http::LatencyUnit;

pub use websocket::SnapshotBroadcaster;

use crate::{
    config::Config, connection_manager::ConnectionManager, db::Database, log_buffer::LogBuffer,
    port_resolver::ResolvedPorts, runtime_status_updater::RuntimeStatusMetrics,
    victoria_logs::VLogsController, zeromq::ZmqConfigPublisher,
};

// Import handlers from submodules
use connections::{get_connection, list_connections};
use logs::get_logs;
use websocket::websocket_handler;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub tx: broadcast::Sender<String>,
    pub connection_manager: Arc<ConnectionManager>,
    pub config_sender: Arc<ZmqConfigPublisher>,
    pub log_buffer: LogBuffer,
    pub allowed_origins: Vec<String>,
    pub cors_disabled: bool,
    pub config: Arc<Config>,
    /// Resolved ZeroMQ ports (may be dynamically assigned)
    pub resolved_ports: Arc<ResolvedPorts>,
    /// Controller for runtime VictoriaLogs toggle (None if not configured in config.toml)
    pub vlogs_controller: Option<VLogsController>,
    /// Shared runtime status metrics (Heartbeat, API, ZMQ handlers)
    pub runtime_status_metrics: Arc<RuntimeStatusMetrics>,
    /// On-demand snapshot broadcaster for WebSocket clients
    pub snapshot_broadcaster: SnapshotBroadcaster,
}

pub fn create_router(state: AppState) -> Router {
    // Create CORS layer - either permissive (all origins) or restricted based on config
    let cors = if state.cors_disabled {
        // CORS disabled: allow all origins (development mode)
        tracing::warn!(
            "CORS is DISABLED - allowing all origins. This should only be used in development!"
        );
        CorsLayer::permissive()
    } else {
        // CORS enabled: restrict to configured origins
        CorsLayer::new()
            .allow_origin(
                state
                    .allowed_origins
                    .iter()
                    .filter_map(|origin| origin.parse().ok())
                    .collect::<Vec<_>>(),
            )
            .allow_methods([
                axum::http::Method::GET,
                axum::http::Method::POST,
                axum::http::Method::PUT,
                axum::http::Method::DELETE,
                axum::http::Method::OPTIONS,
            ])
            .allow_headers([
                axum::http::header::CONTENT_TYPE,
                axum::http::header::AUTHORIZATION,
            ])
            .allow_credentials(true)
    };

    // Create HTTP tracing layer for request/response logging
    // Use DEBUG level to reduce log volume (API requests are frequent)
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(
            DefaultMakeSpan::new()
                .level(tracing::Level::DEBUG)
                .include_headers(true),
        )
        .on_request(|request: &axum::http::Request<_>, _span: &tracing::Span| {
            tracing::debug!(
                method = %request.method(),
                uri = %request.uri(),
                version = ?request.version(),
                "HTTP request started"
            );
        })
        .on_response(
            DefaultOnResponse::new()
                .level(tracing::Level::DEBUG)
                .latency_unit(LatencyUnit::Millis)
                .include_headers(true),
        );

    Router::new()
        .route("/api/connections", get(list_connections))
        .route("/api/connections/:id", get(get_connection))
        .route("/api/logs", get(get_logs))
        .route("/ws", get(websocket_handler))
        // MT installations API
        .route(
            "/api/mt-installations",
            get(mt_installations::list_mt_installations),
        )
        .route(
            "/api/mt-installations/:id/install",
            post(mt_installations::install_to_mt),
        )
        // TradeGroups API (Master settings)
        .route("/api/trade-groups", get(trade_groups::list_trade_groups))
        .route(
            "/api/trade-groups/:id",
            get(trade_groups::get_trade_group)
                .put(trade_groups::update_trade_group_settings)
                .delete(trade_groups::delete_trade_group),
        )
        .route(
            "/api/trade-groups/:id/toggle",
            post(trade_groups::toggle_master),
        )
        // TradeGroupMembers API (Slave settings)
        .route(
            "/api/trade-groups/:id/members",
            get(trade_group_members::list_members).post(trade_group_members::add_member),
        )
        .route(
            "/api/trade-groups/:id/members/:slave_id",
            get(trade_group_members::get_member)
                .put(trade_group_members::update_member)
                .delete(trade_group_members::delete_member),
        )
        .route(
            "/api/trade-groups/:id/members/:slave_id/toggle",
            post(trade_group_members::toggle_member_status),
        )
        // VictoriaLogs API
        // GET /api/victoria-logs-config: Returns config.toml settings (read-only) + current enabled state
        .route(
            "/api/victoria-logs-config",
            get(victoria_logs_settings::get_vlogs_config),
        )
        // PUT /api/victoria-logs-settings: Toggle enabled state only
        .route(
            "/api/victoria-logs-settings",
            axum::routing::put(victoria_logs_settings::toggle_vlogs_enabled),
        )
        // ZeroMQ API
        // GET /api/zeromq-config: Returns current ZeroMQ port configuration (read-only)
        .route(
            "/api/zeromq-config",
            get(zeromq_settings::get_zeromq_config),
        )
        .route(
            "/api/runtime-status-metrics",
            get(runtime_metrics::get_runtime_metrics),
        )
        .layer(trace_layer)
        .layer(cors)
        // PNA headers must be added after CORS layer (outermost) so they are included
        // in CORS preflight responses. Axum layers are applied outside-in for requests
        // and inside-out for responses, so this position ensures all responses
        // (including OPTIONS preflight) get the PNA header.
        .layer(axum_middleware::from_fn(add_pna_headers))
        .with_state(state)
}
