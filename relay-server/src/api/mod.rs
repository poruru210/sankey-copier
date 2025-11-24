//! Relay Server REST API module
//!
//! Provides REST API endpoints for managing copy settings, EA connections,
//! logs, and WebSocket real-time updates. Includes CORS configuration,
//! request tracing, and Private Network Access (PNA) headers.

// Existing submodules (not modified)
mod error;
mod mt_installations;
mod trade_groups;
mod trade_group_members;

// New submodules for modular structure
mod middleware;
mod helpers;
mod settings;
mod connections;
mod logs;
mod websocket;

#[cfg(test)]
mod tests;

// Public re-exports
pub use error::ProblemDetails;
pub use helpers::*;
pub use middleware::*;

use axum::{
    middleware as axum_middleware,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tower_http::cors::CorsLayer;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tower_http::LatencyUnit;

use crate::{
    config::Config,
    connection_manager::ConnectionManager,
    db::Database,
    log_buffer::LogBuffer,
    models::CopySettings,
    zeromq::ZmqConfigPublisher,
};

// Import handlers from submodules
use connections::{get_connection, list_connections};
use logs::get_logs;
use settings::{create_settings, delete_settings, get_settings, list_settings, toggle_settings, update_settings};
use websocket::websocket_handler;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub tx: broadcast::Sender<String>,
    pub settings_cache: Arc<RwLock<Vec<CopySettings>>>,
    pub connection_manager: Arc<ConnectionManager>,
    pub config_sender: Arc<ZmqConfigPublisher>,
    pub log_buffer: LogBuffer,
    pub allowed_origins: Vec<String>,
    pub cors_disabled: bool,
    pub config: Arc<Config>,
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
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(
            DefaultMakeSpan::new()
                .level(tracing::Level::INFO)
                .include_headers(true),
        )
        .on_request(|request: &axum::http::Request<_>, _span: &tracing::Span| {
            tracing::info!(
                method = %request.method(),
                uri = %request.uri(),
                version = ?request.version(),
                "HTTP request started"
            );
        })
        .on_response(
            DefaultOnResponse::new()
                .level(tracing::Level::INFO)
                .latency_unit(LatencyUnit::Millis)
                .include_headers(true),
        );

    Router::new()
        .route("/api/settings", get(list_settings).post(create_settings))
        .route(
            "/api/settings/:id",
            get(get_settings)
                .put(update_settings)
                .delete(delete_settings),
        )
        .route("/api/settings/:id/toggle", post(toggle_settings))
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
            get(trade_groups::get_trade_group).put(trade_groups::update_trade_group_settings),
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
        .layer(trace_layer)
        .layer(cors)
        // PNA headers must be added after CORS layer (outermost) so they are included
        // in CORS preflight responses. Axum layers are applied outside-in for requests
        // and inside-out for responses, so this position ensures all responses
        // (including OPTIONS preflight) get the PNA header.
        .layer(axum_middleware::from_fn(add_pna_headers))
        .with_state(state)
}
