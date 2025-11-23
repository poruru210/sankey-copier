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
/// Calculate effective status based on settings status and master connection state
fn calculate_effective_status(settings_status: i32, master_conn: Option<&EaConnection>) -> i32 {
    if settings_status == 0 {
        // Slave disabled -> DISABLED
        0
    } else {
        // Slave enabled (status == 1 or 2)
        match master_conn {
            Some(conn) if conn.is_trade_allowed => {
                // Both master trade-allowed and slave enabled -> CONNECTED
                2
            }
            _ => {
                // Master not ready or trade not allowed -> ENABLED (but not connected)
                1
            }
        }
    }
}

/// Send configuration message to slave EA
///
/// CopySettingsから完全な設定情報を含むConfigMessageを生成し、
/// ZeroMQ経由でSlaveEAに送信します。
/// Build ConfigMessage with calculated effective status based on Master connection state
async fn build_config_message(state: &AppState, settings: &CopySettings) -> ConfigMessage {
    // Check if Master is connected and has trading allowed
    let master_conn = state
        .connection_manager
        .get_ea(&settings.master_account)
        .await;

    let effective_status = calculate_effective_status(settings.status, master_conn.as_ref());

    ConfigMessage {
        account_id: settings.slave_account.clone(),
        master_account: settings.master_account.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        status: effective_status,
        lot_multiplier: settings.lot_multiplier,
        reverse_trade: settings.reverse_trade,
        symbol_mappings: settings.symbol_mappings.clone(),
        filters: settings.filters.clone(),
        config_version: 1,
        symbol_prefix: settings.symbol_prefix.clone(),
        symbol_suffix: settings.symbol_suffix.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ConnectionStatus, EaType, Platform, SymbolMapping, TradeFilters};
    use chrono::Utc;
    use tokio::sync::broadcast;

    fn create_test_connection(is_trade_allowed: bool) -> EaConnection {
        EaConnection {
            account_id: "MASTER".to_string(),
            ea_type: EaType::Master,
            platform: Platform::MT5,
            account_number: 12345,
            broker: "Broker".to_string(),
            account_name: "Name".to_string(),
            server: "Server".to_string(),
            balance: 1000.0,
            equity: 1000.0,
            currency: "USD".to_string(),
            leverage: 100,
            last_heartbeat: Utc::now(),
            status: ConnectionStatus::Online,
            connected_at: Utc::now(),
            is_trade_allowed,
        }
    }

    async fn create_test_app_state() -> AppState {
        use std::sync::atomic::{AtomicU16, Ordering};
        static PORT_COUNTER: AtomicU16 = AtomicU16::new(15557);

        let db = Arc::new(Database::new(":memory:").await.unwrap());
        let (tx, _) = broadcast::channel(100);
        let settings_cache = Arc::new(RwLock::new(vec![]));
        let connection_manager = Arc::new(ConnectionManager::new(60)); // 60 second timeout

        // Use unique port for each test to avoid "Address in use" errors
        let port = PORT_COUNTER.fetch_add(1, Ordering::SeqCst);
        let config_sender = Arc::new(
            ZmqConfigPublisher::new(&format!("tcp://127.0.0.1:{}", port))
                .expect("Failed to create test config publisher"),
        );
        let log_buffer = LogBuffer::default();
        let config = Arc::new(Config::default());

        AppState {
            db,
            tx,
            settings_cache,
            connection_manager,
            config_sender,
            log_buffer,
            allowed_origins: vec![],
            cors_disabled: true,
            config,
        }
    }

    fn create_test_heartbeat(
        account_id: &str,
        is_trade_allowed: bool,
    ) -> crate::models::HeartbeatMessage {
        crate::models::HeartbeatMessage {
            message_type: "Heartbeat".to_string(),
            account_id: account_id.to_string(),
            balance: 10000.0,
            equity: 10000.0,
            open_positions: 0,
            timestamp: chrono::Utc::now().to_rfc3339(),
            version: "1.0.0".to_string(),
            ea_type: "Master".to_string(),
            platform: "MT5".to_string(),
            account_number: 12345,
            broker: "Test Broker".to_string(),
            account_name: "Test Account".to_string(),
            server: "Test-Server".to_string(),
            currency: "USD".to_string(),
            leverage: 100,
            is_trade_allowed,
            symbol_prefix: None,
            symbol_suffix: None,
            symbol_map: None,
        }
    }

    fn create_test_copy_settings() -> CopySettings {
        CopySettings {
            id: 1,
            status: 1,
            master_account: "MASTER123".to_string(),
            slave_account: "SLAVE456".to_string(),
            lot_multiplier: Some(2.0),
            reverse_trade: false,
            symbol_prefix: Some("pro.".to_string()),
            symbol_suffix: Some(".m".to_string()),
            symbol_mappings: vec![SymbolMapping {
                source_symbol: "EURUSD".to_string(),
                target_symbol: "EURUSD.e".to_string(),
            }],
            filters: TradeFilters {
                allowed_symbols: Some(vec!["EURUSD".to_string()]),
                blocked_symbols: None,
                allowed_magic_numbers: Some(vec![12345]),
                blocked_magic_numbers: None,
            },
        }
    }

    #[test]
    fn test_calculate_effective_status_slave_disabled() {
        // Case 1: Slave is disabled (status 0)
        // Should be 0 regardless of master state
        assert_eq!(calculate_effective_status(0, None), 0);

        let conn = create_test_connection(true);
        assert_eq!(calculate_effective_status(0, Some(&conn)), 0);
    }

    #[test]
    fn test_calculate_effective_status_slave_enabled_master_offline() {
        // Case 2: Slave enabled (1), Master offline (None)
        // Should be 1 (ENABLED)
        assert_eq!(calculate_effective_status(1, None), 1);
    }

    #[test]
    fn test_calculate_effective_status_slave_enabled_master_trade_allowed() {
        // Case 3: Slave enabled (1), Master online & trade allowed
        // Should be 2 (CONNECTED)
        let conn = create_test_connection(true);
        assert_eq!(calculate_effective_status(1, Some(&conn)), 2);
    }

    #[test]
    fn test_calculate_effective_status_slave_enabled_master_trade_not_allowed() {
        // Case 4: Slave enabled (1), Master online but trade NOT allowed
        // Should be 1 (ENABLED)
        let conn = create_test_connection(false);
        assert_eq!(calculate_effective_status(1, Some(&conn)), 1);
    }

    #[tokio::test]
    async fn test_build_config_message_master_online_trade_allowed() {
        // Test ConfigMessage building when Master is online with trade allowed
        let state = create_test_app_state().await;
        let settings = create_test_copy_settings();

        // Register master connection with trade allowed
        let heartbeat = create_test_heartbeat("MASTER123", true);
        state.connection_manager.update_heartbeat(heartbeat).await;

        let config = build_config_message(&state, &settings).await;

        // Verify config fields
        assert_eq!(config.account_id, "SLAVE456");
        assert_eq!(config.master_account, "MASTER123");
        assert_eq!(config.status, 2); // CONNECTED (slave enabled + master trade allowed)
        assert_eq!(config.lot_multiplier, Some(2.0));
        assert_eq!(config.reverse_trade, false);
        assert_eq!(config.symbol_mappings.len(), 1);
        assert_eq!(config.symbol_prefix, Some("pro.".to_string()));
        assert_eq!(config.symbol_suffix, Some(".m".to_string()));
    }

    #[tokio::test]
    async fn test_build_config_message_master_offline() {
        // Test ConfigMessage building when Master is offline
        let state = create_test_app_state().await;
        let settings = create_test_copy_settings();

        // No master connection registered
        let config = build_config_message(&state, &settings).await;

        // Verify status is ENABLED (1) when master is offline
        assert_eq!(config.status, 1);
        assert_eq!(config.account_id, "SLAVE456");
        assert_eq!(config.master_account, "MASTER123");
    }

    #[tokio::test]
    async fn test_build_config_message_master_trade_not_allowed() {
        // Test ConfigMessage building when Master is online but trade not allowed
        let state = create_test_app_state().await;
        let settings = create_test_copy_settings();

        // Register master connection with trade NOT allowed
        let heartbeat = create_test_heartbeat("MASTER123", false);
        state.connection_manager.update_heartbeat(heartbeat).await;

        let config = build_config_message(&state, &settings).await;

        // Verify status is ENABLED (1) when master trade is not allowed
        assert_eq!(config.status, 1);
    }

    #[tokio::test]
    async fn test_build_config_message_slave_disabled() {
        // Test ConfigMessage building when Slave is disabled
        let state = create_test_app_state().await;
        let mut settings = create_test_copy_settings();
        settings.status = 0; // Disabled

        // Register master connection with trade allowed
        let heartbeat = create_test_heartbeat("MASTER123", true);
        state.connection_manager.update_heartbeat(heartbeat).await;

        let config = build_config_message(&state, &settings).await;

        // Verify status is DISABLED (0) regardless of master state
        assert_eq!(config.status, 0);
    }

    #[tokio::test]
    async fn test_refresh_settings_cache_success() {
        // Test successful cache refresh
        let state = create_test_app_state().await;
        let mut settings = create_test_copy_settings();
        settings.id = 0; // Use 0 for new record creation

        // Save settings to DB
        state.db.save_copy_settings(&settings).await.unwrap();

        // Initial cache should be empty
        assert_eq!(state.settings_cache.read().await.len(), 0);

        // Refresh cache
        refresh_settings_cache(&state).await;

        // Cache should now contain the setting
        let cache = state.settings_cache.read().await;
        assert_eq!(cache.len(), 1);
        assert_eq!(cache[0].master_account, "MASTER123");
        assert_eq!(cache[0].slave_account, "SLAVE456");
    }

    #[tokio::test]
    async fn test_refresh_settings_cache_empty_db() {
        // Test cache refresh with empty database
        let state = create_test_app_state().await;

        // Refresh cache with empty DB
        refresh_settings_cache(&state).await;

        // Cache should remain empty
        assert_eq!(state.settings_cache.read().await.len(), 0);
    }

    #[tokio::test]
    async fn test_build_config_message_with_null_values() {
        // Test ConfigMessage building with null optional fields
        let state = create_test_app_state().await;
        let mut settings = create_test_copy_settings();
        settings.lot_multiplier = None;
        settings.symbol_prefix = None;
        settings.symbol_suffix = None;
        settings.symbol_mappings = vec![];

        let config = build_config_message(&state, &settings).await;

        // Verify null fields are correctly passed through
        assert_eq!(config.lot_multiplier, None);
        assert_eq!(config.symbol_prefix, None);
        assert_eq!(config.symbol_suffix, None);
        assert_eq!(config.symbol_mappings.len(), 0);
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
    #[serde(default)]
    symbol_prefix: Option<String>,
    #[serde(default)]
    symbol_suffix: Option<String>,
    #[serde(default)]
    symbol_mappings: Option<Vec<crate::models::SymbolMapping>>,
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
        symbol_prefix: req.symbol_prefix.clone(),
        symbol_suffix: req.symbol_suffix.clone(),
        symbol_mappings: req.symbol_mappings.unwrap_or_default(),
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
            symbol_prefix: None,
            symbol_suffix: None,
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
