// relay-server/src/api/victoria_logs_settings.rs
//
// REST API endpoints for VictoriaLogs configuration and settings management.
// - GET /api/victoria-logs-config: Returns config.toml settings (read-only) + current enabled state
// - PUT /api/victoria-logs-settings: Toggle enabled state only

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};

use crate::models::VLogsGlobalSettings;

use super::{AppState, ProblemDetails};

/// Response for GET /api/victoria-logs-config
/// Contains config.toml settings (read-only) and current enabled state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VLogsConfigResponse {
    /// Whether VictoriaLogs is configured in config.toml (has non-empty host)
    pub configured: bool,
    /// Config from config.toml (None if not configured)
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub config: Option<VLogsConfigInfo>,
    /// Current runtime enabled state
    pub enabled: bool,
}

/// Config information from config.toml (read-only)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VLogsConfigInfo {
    /// VictoriaLogs host URL (e.g., "http://localhost:9428")
    pub host: String,
    pub batch_size: usize,
    pub flush_interval_secs: u64,
    pub source: String,
    /// Current log level ("DEBUG", "INFO", "WARN", "ERROR")
    pub log_level: String,
}

/// Request for PUT /api/victoria-logs-settings
/// Enabled state and log level can be toggled
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VLogsToggleRequest {
    pub enabled: bool,
    /// Optional log level update ("DEBUG", "INFO", "WARN", "ERROR")
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub log_level: Option<String>,
}

/// GET /api/victoria-logs-config
/// Returns config.toml settings (read-only) and current enabled state
pub async fn get_vlogs_config(
    State(state): State<AppState>,
) -> Result<Json<VLogsConfigResponse>, ProblemDetails> {
    let span = tracing::info_span!("get_vlogs_config");
    let _enter = span.enter();

    match &state.vlogs_controller {
        Some(controller) => {
            let config = controller.config();

            // Get current log level from DB, default to DEBUG
            let log_level = match state.db.get_vlogs_settings().await {
                Ok(settings) => settings.log_level,
                _ => "DEBUG".to_string(),
            };

            let response = VLogsConfigResponse {
                configured: true,
                config: Some(VLogsConfigInfo {
                    host: config.host.clone(),
                    batch_size: config.batch_size,
                    flush_interval_secs: config.flush_interval_secs,
                    source: config.source.clone(),
                    log_level: log_level.clone(),
                }),
                enabled: controller.is_enabled(),
            };

            tracing::info!(
                configured = true,
                enabled = response.enabled,
                host = %config.host,
                log_level = %log_level,
                "Retrieved VictoriaLogs config"
            );

            Ok(Json(response))
        }
        None => {
            tracing::info!(
                configured = false,
                "VictoriaLogs not configured in config.toml"
            );

            Ok(Json(VLogsConfigResponse {
                configured: false,
                config: None,
                enabled: false,
            }))
        }
    }
}

/// PUT /api/victoria-logs-settings
/// Update VictoriaLogs enabled state and/or log level
/// Updates runtime state and broadcasts to all connected EAs
pub async fn toggle_vlogs_enabled(
    State(state): State<AppState>,
    Json(request): Json<VLogsToggleRequest>,
) -> Result<StatusCode, ProblemDetails> {
    let span = tracing::info_span!("toggle_vlogs_enabled", enabled = request.enabled);
    let _enter = span.enter();

    // Check if VictoriaLogs is configured
    let controller = state.vlogs_controller.as_ref().ok_or_else(|| {
        tracing::warn!("Attempted to toggle VictoriaLogs but it's not configured");
        ProblemDetails::validation_error(
            "VictoriaLogs is not configured in config.toml. Add [victoria_logs] section with endpoint to enable this feature.",
        )
    })?;

    // Update runtime state
    controller.set_enabled(request.enabled);

    // Get current log_level from DB or use provided value
    let current_settings = state.db.get_vlogs_settings().await.ok();
    let log_level = request.log_level.unwrap_or_else(|| {
        current_settings
            .map(|s| s.log_level)
            .unwrap_or_else(|| "DEBUG".to_string())
    });

    // Validate log level
    let valid_levels = ["DEBUG", "INFO", "WARN", "ERROR"];
    if !valid_levels.contains(&log_level.as_str()) {
        return Err(ProblemDetails::validation_error(format!(
            "Invalid log_level '{}'. Must be one of: DEBUG, INFO, WARN, ERROR",
            log_level
        )));
    }

    tracing::info!(
        enabled = request.enabled,
        log_level = %log_level,
        "Updated VictoriaLogs runtime state"
    );

    // Save enabled state to database for persistence across restarts
    let config = controller.config();
    let settings = VLogsGlobalSettings {
        enabled: request.enabled,
        endpoint: config.endpoint(), // Full URL (host + fixed path)
        batch_size: config.batch_size as i32,
        flush_interval_secs: config.flush_interval_secs as i32,
        log_level: log_level.clone(),
    };

    if let Err(e) = state.db.save_vlogs_settings(&settings).await {
        tracing::error!(
            error = %e,
            "Failed to persist VictoriaLogs settings to database"
        );
        // Continue even if DB save fails - runtime state is already updated
    }

    // Broadcast settings to all connected EAs
    broadcast_vlogs_config(&state, &settings).await;

    // Notify via WebSocket
    if let Ok(json) = serde_json::to_string(&settings) {
        let _ = state.tx.send(format!("vlogs_settings_updated:{}", json));
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Broadcast VictoriaLogs config to all connected EAs via ZMQ
async fn broadcast_vlogs_config(state: &AppState, settings: &VLogsGlobalSettings) {
    if let Err(e) = state.config_sender.broadcast_vlogs_config(settings).await {
        tracing::error!(
            enabled = settings.enabled,
            error = %e,
            "Failed to broadcast VictoriaLogs config via ZMQ"
        );
    } else {
        tracing::info!(
            enabled = settings.enabled,
            "Successfully broadcasted VictoriaLogs config to all EAs"
        );
    }
}
