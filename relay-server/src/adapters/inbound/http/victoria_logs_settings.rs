// relay-server/src/api/victoria_logs_settings.rs
//
// REST API endpoints for VictoriaLogs configuration and settings management.
// - GET /api/victoria-logs-config: Returns config.toml settings (read-only) + current enabled state
// - PUT /api/victoria-logs-settings: Toggle enabled state only (updates config.toml)

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};

use crate::config::update_victoria_logs_config;
use crate::domain::models::VLogsGlobalSettings;

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
    pub log_level: String,
}

/// Request for PUT /api/victoria-logs-settings
/// Enabled state and log level can be updated (persisted to config.toml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VLogsUpdateRequest {
    pub enabled: Option<bool>,
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

            let response = VLogsConfigResponse {
                configured: true,
                config: Some(VLogsConfigInfo {
                    host: config.host.clone(),
                    batch_size: config.batch_size,
                    flush_interval_secs: config.flush_interval_secs,
                    source: config.source.clone(),
                    log_level: config.log_level.clone(),
                }),
                enabled: controller.is_enabled(),
            };

            tracing::info!(
                configured = true,
                enabled = response.enabled,
                host = %config.host,
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
/// Updates runtime state, persists to config.toml, and broadcasts to all connected EAs
pub async fn update_vlogs_settings(
    State(state): State<AppState>,
    Json(request): Json<VLogsUpdateRequest>,
) -> Result<StatusCode, ProblemDetails> {
    let span = tracing::info_span!("update_vlogs_settings", ?request);
    let _enter = span.enter();

    // Check if VictoriaLogs is configured
    let controller = state.vlogs_controller.as_ref().ok_or_else(|| {
        tracing::warn!("Attempted to update VictoriaLogs settings but it's not configured");
        ProblemDetails::validation_error(
            "VictoriaLogs is not configured in config.toml. Add [victoria_logs] section with host to enable this feature.",
        )
    })?;

    // Update runtime state if enabled is provided
    if let Some(enabled) = request.enabled {
        controller.set_enabled(enabled);
    }

    // Note: We don't update log_level in controller runtime state immediately
    // because controller usually holds the initial config.
    // However, for correct broadcast, we need to ensure the updated log_level is used.
    // Since config.rs reload is not trivial without restart, we blindly persist
    // and then construct the broadcast message using the requested value or fallback.
    // Ideally, controller should be mutable or reloadable. For now, let's rely on persistence.

    tracing::info!(?request, "Updated VictoriaLogs runtime state");

    // Save state to config file for persistence across restarts
    let config_dir = std::env::var("CONFIG_DIR").unwrap_or_else(|_| ".".to_string());
    let config_base = format!("{}/config", config_dir);

    if let Err(e) =
        update_victoria_logs_config(request.enabled, request.log_level.clone(), &config_base)
    {
        tracing::error!(
            error = %e,
            config_base = %config_base,
            "Failed to persist VictoriaLogs settings to config file"
        );
        return Err(ProblemDetails::internal_error(format!(
            "Failed to update config file: {}",
            e
        )));
    }

    tracing::info!(
        ?request,
        config_base = %config_base,
        "VictoriaLogs settings saved to config file"
    );

    // Build settings for broadcast
    // Logic: Use existing config values, override with request if present
    let config = controller.config();
    let effective_enabled = request.enabled.unwrap_or_else(|| controller.is_enabled());
    let effective_log_level = request
        .log_level
        .clone()
        .unwrap_or_else(|| config.log_level.clone());

    let settings = VLogsGlobalSettings {
        enabled: effective_enabled,
        endpoint: config.endpoint(),
        batch_size: config.batch_size as i32,
        flush_interval_secs: config.flush_interval_secs as i32,
        log_level: effective_log_level,
    };

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
