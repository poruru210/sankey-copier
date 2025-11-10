mod mt_installations;

use axum::{
    extract::{Path, State, ws::WebSocket, ws::WebSocketUpgrade},
    response::Response,
    routing::{get, post, delete},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tower_http::cors::CorsLayer;

use crate::{
    connection_manager::ConnectionManager,
    db::Database,
    log_buffer::LogBuffer,
    models::{CopySettings, EaConnection, ConfigMessage},
    zeromq::ZmqConfigPublisher,
};

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub tx: broadcast::Sender<String>,
    pub settings_cache: Arc<RwLock<Vec<CopySettings>>>,
    pub connection_manager: Arc<ConnectionManager>,
    pub config_sender: Arc<ZmqConfigPublisher>,
    pub log_buffer: LogBuffer,
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/api/settings", get(list_settings).post(create_settings))
        .route("/api/settings/:id", get(get_settings).put(update_settings).delete(delete_settings))
        .route("/api/settings/:id/toggle", post(toggle_settings))
        .route("/api/connections", get(list_connections))
        .route("/api/connections/:id", get(get_connection))
        .route("/api/logs", get(get_logs))
        .route("/ws", get(websocket_handler))
        // MT installations API
        .route("/api/mt-installations", get(mt_installations::list_mt_installations))
        .route("/api/mt-installations/manual", post(mt_installations::add_manual_installation))
        .route("/api/mt-installations/:id", delete(mt_installations::remove_manual_installation))
        .layer(CorsLayer::permissive())
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
async fn send_config_to_ea(state: &AppState, settings: &CopySettings) {
    // From<CopySettings>トレイトを使用して変換
    let config: ConfigMessage = settings.clone().into();

    if let Err(e) = state.config_sender.send_config(&config).await {
        tracing::error!("Failed to send config message to {}: {}", settings.slave_account, e);
    } else {
        tracing::info!(
            "Sent full config to EA: {} (master: {}, enabled: {}, lot_mult: {:?})",
            settings.slave_account,
            settings.master_account,
            settings.enabled,
            settings.lot_multiplier
        );
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

impl<T> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}

async fn list_settings(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<CopySettings>>>, Response> {
    match state.db.list_copy_settings().await {
        Ok(settings) => {
            refresh_settings_cache(&state).await;
            Ok(Json(ApiResponse::success(settings)))
        }
        Err(e) => {
            tracing::error!("Failed to list settings: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

async fn get_settings(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<CopySettings>>, Response> {
    match state.db.get_copy_settings(id).await {
        Ok(Some(settings)) => Ok(Json(ApiResponse::success(settings))),
        Ok(None) => Ok(Json(ApiResponse::error("Settings not found".to_string()))),
        Err(e) => {
            tracing::error!("Failed to get settings: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

#[derive(Debug, Deserialize)]
struct CreateSettingsRequest {
    master_account: String,
    slave_account: String,
    lot_multiplier: Option<f64>,
    reverse_trade: bool,
}

async fn create_settings(
    State(state): State<AppState>,
    Json(req): Json<CreateSettingsRequest>,
) -> Result<Json<ApiResponse<i32>>, Response> {
    let settings = CopySettings {
        id: 0,
        enabled: true,
        master_account: req.master_account,
        slave_account: req.slave_account,
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
            refresh_settings_cache(&state).await;
            send_config_to_ea(&state, &settings).await;

            // Notify via WebSocket
            let _ = state.tx.send(format!("settings_updated:{}", id));

            Ok(Json(ApiResponse::success(id)))
        }
        Err(e) => {
            let error_msg = e.to_string();
            tracing::error!("Failed to create settings: {}", error_msg);

            // Check for duplicate entry error
            let user_friendly_msg = if error_msg.contains("UNIQUE constraint failed") {
                "この組み合わせの接続設定は既に存在します。同じマスターとスレーブのペアは1つのみ登録できます。".to_string()
            } else {
                error_msg
            };

            Ok(Json(ApiResponse::error(user_friendly_msg)))
        }
    }
}

async fn update_settings(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(settings): Json<CopySettings>,
) -> Result<Json<ApiResponse<()>>, Response> {
    let mut updated = settings;
    updated.id = id;

    match state.db.save_copy_settings(&updated).await {
        Ok(_) => {
            refresh_settings_cache(&state).await;
            send_config_to_ea(&state, &updated).await;

            // Notify via WebSocket
            let _ = state.tx.send(format!("settings_updated:{}", id));

            Ok(Json(ApiResponse::success(())))
        }
        Err(e) => {
            let error_msg = e.to_string();
            tracing::error!("Failed to update settings: {}", error_msg);

            // Check for duplicate entry error
            let user_friendly_msg = if error_msg.contains("UNIQUE constraint failed") {
                "この組み合わせの接続設定は既に存在します。同じマスターとスレーブのペアは1つのみ登録できます。".to_string()
            } else {
                error_msg
            };

            Ok(Json(ApiResponse::error(user_friendly_msg)))
        }
    }
}

#[derive(Debug, Deserialize)]
struct ToggleRequest {
    enabled: bool,
}

async fn toggle_settings(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<ToggleRequest>,
) -> Result<Json<ApiResponse<()>>, Response> {
    match state.db.update_enabled_status(id, req.enabled).await {
        Ok(_) => {
            refresh_settings_cache(&state).await;

            // Notify via WebSocket
            let _ = state.tx.send(format!("settings_toggled:{}:{}", id, req.enabled));

            Ok(Json(ApiResponse::success(())))
        }
        Err(e) => {
            tracing::error!("Failed to toggle settings: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

async fn delete_settings(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, Response> {
    match state.db.delete_copy_settings(id).await {
        Ok(_) => {
            refresh_settings_cache(&state).await;

            // Notify via WebSocket
            let _ = state.tx.send(format!("settings_deleted:{}", id));

            Ok(Json(ApiResponse::success(())))
        }
        Err(e) => {
            tracing::error!("Failed to delete settings: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(|socket| handle_websocket(socket, state))
}

async fn handle_websocket(mut socket: WebSocket, state: AppState) {
    let mut rx = state.tx.subscribe();

    while let Ok(msg) = rx.recv().await {
        if socket.send(axum::extract::ws::Message::Text(msg)).await.is_err() {
            break;
        }
    }
}

// EA接続一覧取得
async fn list_connections(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<EaConnection>>>, Response> {
    let connections = state.connection_manager.get_all_eas().await;
    Ok(Json(ApiResponse::success(connections)))
}

// 特定のEA接続情報取得
async fn get_connection(
    State(state): State<AppState>,
    Path(account_id): Path<String>,
) -> Result<Json<ApiResponse<EaConnection>>, Response> {
    match state.connection_manager.get_ea(&account_id).await {
        Some(connection) => Ok(Json(ApiResponse::success(connection))),
        None => Ok(Json(ApiResponse::error(format!(
            "Connection not found: {}",
            account_id
        )))),
    }
}

// サーバーログ取得
async fn get_logs(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<crate::log_buffer::LogEntry>>>, Response> {
    let buffer = state.log_buffer.read().await;
    let logs: Vec<_> = buffer.iter().cloned().collect();
    Ok(Json(ApiResponse::success(logs)))
}
