mod error;
mod mt_installations;

pub use error::ProblemDetails;

use axum::{
    body::Body,
    extract::{ws::WebSocket, ws::WebSocketUpgrade, Path, State},
    http::{header::HeaderValue, Request, StatusCode},
    middleware,
    response::Response,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
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
    models::{ConfigMessage, CopySettings, EaConnection},
    zeromq::ZmqConfigPublisher,
};

/// Middleware to add PNA (Private Network Access) headers
///
/// Adds Access-Control-Allow-Private-Network header to responses
/// for browsers to allow HTTPS pages to access local network resources.
async fn add_pna_headers(request: Request<Body>, next: middleware::Next) -> Response {
    let mut response = next.run(request).await;

    // Add PNA header to allow private network access
    // This is required for Chrome's Private Network Access feature
    response.headers_mut().insert(
        "Access-Control-Allow-Private-Network",
        HeaderValue::from_static("true"),
    );

    response
}

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
        .layer(trace_layer)
        .layer(cors)
        // PNA headers must be added after CORS layer (outermost) so they are included
        // in CORS preflight responses. Axum layers are applied outside-in for requests
        // and inside-out for responses, so this position ensures all responses
        // (including OPTIONS preflight) get the PNA header.
        .layer(middleware::from_fn(add_pna_headers))
        .with_state(state)
}

/// Refresh the settings cache from the database
async fn refresh_settings_cache(state: &AppState) {
    if let Ok(all_settings) = state.db.list_copy_settings().await {
        *state.settings_cache.write().await = all_settings;
    }
}

/// Send configuration message to slave EA
///
/// CopySettingsから完全な設定情報を含むConfigMessageを生成し、
/// ZeroMQ経由でSlaveEAに送信します。
/// Build ConfigMessage with calculated effective status based on Master connection state
async fn build_config_message(state: &AppState, settings: &CopySettings) -> ConfigMessage {
    // Calculate effective status (0/1/2) based on Master's is_trade_allowed
    let effective_status = if settings.status == 0 {
        // User disabled -> DISABLED
        0
    } else {
        // User enabled (status == 1)
        // Check if Master is connected and has trading allowed
        let master_conn = state
            .connection_manager
            .get_ea(&settings.master_account)
            .await;

        if let Some(conn) = master_conn {
            if conn.is_trade_allowed {
                // Master online && trading allowed -> CONNECTED
                2
            } else {
                // Master online but trading NOT allowed -> ENABLED (but not connected)
                1
            }
        } else {
            // Master offline -> ENABLED (but not connected)
            1
        }
    };

    ConfigMessage {
        account_id: settings.slave_account.clone(),
        master_account: settings.master_account.clone(),
        trade_group_id: settings.master_account.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        status: effective_status,
        lot_multiplier: settings.lot_multiplier,
        reverse_trade: settings.reverse_trade,
        symbol_mappings: settings.symbol_mappings.clone(),
        filters: settings.filters.clone(),
        config_version: 1,
    }
}

async fn send_config_to_ea(state: &AppState, settings: &CopySettings) {
    // Build ConfigMessage with calculated effective status
    let config = build_config_message(state, settings).await;

    if let Err(e) = state.config_sender.send_config(&config).await {
        tracing::error!(
            "Failed to send config message to {}: {}",
            settings.slave_account,
            e
        );
    } else {
        tracing::info!(
            "Sent full config to EA: {} (master: {}, db_status: {}, effective_status: {}, lot_mult: {:?})",
            settings.slave_account,
            settings.master_account,
            settings.status,
            config.status,
            settings.lot_multiplier
        );
    }
}

async fn list_settings(
    State(state): State<AppState>,
) -> Result<Json<Vec<CopySettings>>, ProblemDetails> {
    let span = tracing::info_span!("list_settings");
    let _enter = span.enter();

    match state.db.list_copy_settings().await {
        Ok(settings) => {
            tracing::info!(
                count = settings.len(),
                "Successfully retrieved copy settings"
            );
            refresh_settings_cache(&state).await;
            Ok(Json(settings))
        }
        Err(e) => {
            tracing::error!(
                error = %e,
                error_type = std::any::type_name_of_val(&e),
                backtrace = ?std::backtrace::Backtrace::capture(),
                "Failed to list settings from database"
            );
            Err(ProblemDetails::internal_error(format!(
                "Failed to retrieve settings from database: {}",
                e
            )))
        }
    }
}

async fn get_settings(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<CopySettings>, ProblemDetails> {
    let span = tracing::info_span!("get_settings", settings_id = id);
    let _enter = span.enter();

    match state.db.get_copy_settings(id).await {
        Ok(Some(settings)) => {
            tracing::info!(
                settings_id = id,
                master_account = %settings.master_account,
                slave_account = %settings.slave_account,
                status = settings.status,
                "Successfully retrieved copy settings"
            );
            Ok(Json(settings))
        }
        Ok(None) => {
            tracing::warn!(settings_id = id, "Settings not found");
            Err(ProblemDetails::not_found("settings")
                .with_instance(format!("/api/settings/{}", id)))
        }
        Err(e) => {
            tracing::error!(
                settings_id = id,
                error = %e,
                error_type = std::any::type_name_of_val(&e),
                backtrace = ?std::backtrace::Backtrace::capture(),
                "Failed to get settings from database"
            );
            Err(ProblemDetails::internal_error(format!(
                "Failed to retrieve settings from database: {}",
                e
            ))
            .with_instance(format!("/api/settings/{}", id)))
        }
    }
}

#[derive(Debug, Deserialize)]
struct CreateSettingsRequest {
    master_account: String,
    slave_account: String,
    lot_multiplier: Option<f64>,
    reverse_trade: bool,
    status: Option<i32>, // Allow frontend to control initial status (0=DISABLED, 1=ENABLED, 2=CONNECTED)
}

async fn create_settings(
    State(state): State<AppState>,
    Json(req): Json<CreateSettingsRequest>,
) -> Result<(StatusCode, Json<i32>), ProblemDetails> {
    let span = tracing::info_span!(
        "create_settings",
        master_account = %req.master_account,
        slave_account = %req.slave_account
    );
    let _enter = span.enter();

    let settings = CopySettings {
        id: 0,
        status: req.status.unwrap_or(0), // Respect frontend's status value, default to DISABLED (0)
        master_account: req.master_account.clone(),
        slave_account: req.slave_account.clone(),
        lot_multiplier: req.lot_multiplier,
        reverse_trade: req.reverse_trade,
        symbol_mappings: vec![],
        filters: crate::models::TradeFilters {
            allowed_symbols: None,
            blocked_symbols: None,
            allowed_magic_numbers: None,
            blocked_magic_numbers: None,
        },
    };

    match state.db.save_copy_settings(&settings).await {
        Ok(id) => {
            tracing::info!(
                settings_id = id,
                master_account = %req.master_account,
                slave_account = %req.slave_account,
                lot_multiplier = ?req.lot_multiplier,
                reverse_trade = req.reverse_trade,
                "Successfully created copy settings"
            );

            refresh_settings_cache(&state).await;
            send_config_to_ea(&state, &settings).await;

            // Update settings object with the generated ID
            let mut created_settings = settings.clone();
            created_settings.id = id;

            // Notify via WebSocket
            if let Ok(json) = serde_json::to_string(&created_settings) {
                let _ = state.tx.send(format!("settings_created:{}", json));
            }

            Ok((StatusCode::CREATED, Json(id)))
        }
        Err(e) => {
            let error_msg = e.to_string();
            let is_duplicate = error_msg.contains("UNIQUE constraint failed");

            tracing::error!(
                master_account = %req.master_account,
                slave_account = %req.slave_account,
                error = %e,
                error_type = std::any::type_name_of_val(&e),
                is_duplicate_error = is_duplicate,
                backtrace = ?std::backtrace::Backtrace::capture(),
                "Failed to create copy settings"
            );

            // Check for duplicate entry error
            if is_duplicate {
                Err(ProblemDetails::conflict(
                    "A connection setting with this combination already exists. Only one master-slave pair can be registered."
                ).with_instance("/api/settings"))
            } else {
                Err(ProblemDetails::internal_error(format!(
                    "Failed to create settings: {}",
                    error_msg
                ))
                .with_instance("/api/settings"))
            }
        }
    }
}

async fn update_settings(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(settings): Json<CopySettings>,
) -> Result<StatusCode, ProblemDetails> {
    let span = tracing::info_span!(
        "update_settings",
        settings_id = id,
        master_account = %settings.master_account,
        slave_account = %settings.slave_account
    );
    let _enter = span.enter();

    let mut updated = settings;
    updated.id = id;

    match state.db.save_copy_settings(&updated).await {
        Ok(_) => {
            tracing::info!(
                settings_id = id,
                master_account = %updated.master_account,
                slave_account = %updated.slave_account,
                status = updated.status,
                lot_multiplier = ?updated.lot_multiplier,
                reverse_trade = updated.reverse_trade,
                "Successfully updated copy settings"
            );

            refresh_settings_cache(&state).await;
            send_config_to_ea(&state, &updated).await;

            // Notify via WebSocket
            if let Ok(json) = serde_json::to_string(&updated) {
                let _ = state.tx.send(format!("settings_updated:{}", json));
            }

            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            let error_msg = e.to_string();
            let is_duplicate = error_msg.contains("UNIQUE constraint failed");

            tracing::error!(
                settings_id = id,
                master_account = %updated.master_account,
                slave_account = %updated.slave_account,
                error = %e,
                error_type = std::any::type_name_of_val(&e),
                is_duplicate_error = is_duplicate,
                backtrace = ?std::backtrace::Backtrace::capture(),
                "Failed to update copy settings"
            );

            // Check for duplicate entry error
            if is_duplicate {
                Err(ProblemDetails::conflict(
                    "A connection setting with this combination already exists. Only one master-slave pair can be registered."
                ).with_instance(format!("/api/settings/{}", id)))
            } else {
                Err(ProblemDetails::internal_error(format!(
                    "Failed to update settings: {}",
                    error_msg
                ))
                .with_instance(format!("/api/settings/{}", id)))
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct ToggleRequest {
    status: i32, // 0=DISABLED, 1=ENABLED, 2=CONNECTED
}

async fn toggle_settings(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<ToggleRequest>,
) -> Result<StatusCode, ProblemDetails> {
    let span = tracing::info_span!("toggle_settings", settings_id = id, status = req.status);
    let _enter = span.enter();

    // Simplified: Store user's switch state (0=OFF, 1=ON) as-is
    // Active state will be calculated at runtime based on:
    // - Master: is_trade_allowed && status == 1
    // - Slave: is_trade_allowed && status == 1 && all_masters_active
    match state.db.update_status(id, req.status).await {
        Ok(_) => {
            tracing::info!(
                settings_id = id,
                status = req.status,
                "Successfully toggled copy settings"
            );

            refresh_settings_cache(&state).await;

            // Send updated config to Slave EA for real-time reflection
            if let Ok(Some(settings)) = state.db.get_copy_settings(id).await {
                send_config_to_ea(&state, &settings).await;
            }

            // Notify via WebSocket
            if let Ok(Some(updated_settings)) = state.db.get_copy_settings(id).await {
                if let Ok(json) = serde_json::to_string(&updated_settings) {
                    let _ = state.tx.send(format!("settings_updated:{}", json));
                }
            }

            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            tracing::error!(
                settings_id = id,
                status = req.status,
                error = %e,
                error_type = std::any::type_name_of_val(&e),
                backtrace = ?std::backtrace::Backtrace::capture(),
                "Failed to toggle copy settings"
            );
            Err(
                ProblemDetails::internal_error(format!("Failed to toggle settings: {}", e))
                    .with_instance(format!("/api/settings/{}/toggle", id)),
            )
        }
    }
}

async fn delete_settings(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<StatusCode, ProblemDetails> {
    let span = tracing::info_span!("delete_settings", settings_id = id);
    let _enter = span.enter();

    // Retrieve settings before deletion to send notification to Slave EA
    let settings_opt = match state.db.get_copy_settings(id).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(
                settings_id = id,
                error = %e,
                "Failed to retrieve settings before deletion"
            );
            return Err(ProblemDetails::internal_error(format!(
                "Failed to retrieve settings: {}",
                e
            ))
            .with_instance(format!("/api/settings/{}", id)));
        }
    };

    // If settings exist, send notification to Slave EA
    if let Some(settings) = settings_opt {
        tracing::info!(
            settings_id = id,
            slave_account = %settings.slave_account,
            "Sending delete notification to Slave EA"
        );

        // Send config with status=0 to indicate deletion
        let delete_config = ConfigMessage {
            account_id: settings.slave_account.clone(),
            master_account: settings.master_account.clone(),
            trade_group_id: settings.master_account.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            status: 0, // DISABLED - indicates config was deleted
            lot_multiplier: None,
            reverse_trade: false,
            symbol_mappings: vec![],
            filters: crate::models::TradeFilters {
                allowed_symbols: None,
                blocked_symbols: None,
                allowed_magic_numbers: None,
                blocked_magic_numbers: None,
            },
            config_version: 1,
        };

        if let Err(e) = state.config_sender.send_config(&delete_config).await {
            tracing::warn!(
                settings_id = id,
                slave_account = %settings.slave_account,
                error = %e,
                "Failed to send delete notification to Slave EA (continuing with deletion)"
            );
        } else {
            tracing::info!(
                settings_id = id,
                slave_account = %settings.slave_account,
                "Delete notification sent successfully"
            );
        }
    }

    // Delete from database
    match state.db.delete_copy_settings(id).await {
        Ok(_) => {
            tracing::info!(settings_id = id, "Successfully deleted copy settings");

            refresh_settings_cache(&state).await;

            // Notify via WebSocket
            let _ = state.tx.send(format!("settings_deleted:{}", id));

            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            tracing::error!(
                settings_id = id,
                error = %e,
                error_type = std::any::type_name_of_val(&e),
                backtrace = ?std::backtrace::Backtrace::capture(),
                "Failed to delete copy settings"
            );
            Err(
                ProblemDetails::internal_error(format!("Failed to delete settings: {}", e))
                    .with_instance(format!("/api/settings/{}", id)),
            )
        }
    }
}

async fn websocket_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(|socket| handle_websocket(socket, state))
}

async fn handle_websocket(mut socket: WebSocket, state: AppState) {
    let mut rx = state.tx.subscribe();

    while let Ok(msg) = rx.recv().await {
        if socket
            .send(axum::extract::ws::Message::Text(msg))
            .await
            .is_err()
        {
            break;
        }
    }
}

// EA接続一覧取得
async fn list_connections(
    State(state): State<AppState>,
) -> Result<Json<Vec<EaConnection>>, ProblemDetails> {
    let span = tracing::info_span!("list_connections");
    let _enter = span.enter();

    let connections = state.connection_manager.get_all_eas().await;

    tracing::info!(
        count = connections.len(),
        "Successfully retrieved EA connections"
    );

    Ok(Json(connections))
}

// 特定のEA接続情報取得
async fn get_connection(
    State(state): State<AppState>,
    Path(account_id): Path<String>,
) -> Result<Json<EaConnection>, ProblemDetails> {
    let span = tracing::info_span!("get_connection", account_id = %account_id);
    let _enter = span.enter();

    match state.connection_manager.get_ea(&account_id).await {
        Some(connection) => {
            tracing::info!(
                account_id = %account_id,
                ea_type = ?connection.ea_type,
                status = ?connection.status,
                "Successfully retrieved EA connection"
            );
            Ok(Json(connection))
        }
        None => {
            tracing::warn!(
                account_id = %account_id,
                "EA connection not found"
            );
            Err(ProblemDetails::not_found("EA connection")
                .with_instance(format!("/api/connections/{}", account_id)))
        }
    }
}

// サーバーログ取得
async fn get_logs(
    State(state): State<AppState>,
) -> Result<Json<Vec<crate::log_buffer::LogEntry>>, ProblemDetails> {
    let span = tracing::info_span!("get_logs");
    let _enter = span.enter();

    let buffer = state.log_buffer.read().await;
    let logs: Vec<_> = buffer.iter().cloned().collect();

    tracing::debug!(count = logs.len(), "Successfully retrieved server logs");

    Ok(Json(logs))
}
