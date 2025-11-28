// relay-server/src/api/victoria_logs_settings.rs
//
// REST API endpoints for VictoriaLogs global settings management.
// Provides GET/PUT endpoints for Web UI configuration.

use axum::{extract::State, http::StatusCode, Json};

use crate::models::VLogsGlobalSettings;

use super::{AppState, ProblemDetails};

/// Get VictoriaLogs global settings
pub async fn get_vlogs_settings(
    State(state): State<AppState>,
) -> Result<Json<VLogsGlobalSettings>, ProblemDetails> {
    let span = tracing::info_span!("get_vlogs_settings");
    let _enter = span.enter();

    match state.db.get_vlogs_settings().await {
        Ok(settings) => {
            tracing::info!(
                enabled = settings.enabled,
                endpoint = %settings.endpoint,
                batch_size = settings.batch_size,
                flush_interval_secs = settings.flush_interval_secs,
                "Retrieved VictoriaLogs settings"
            );
            Ok(Json(settings))
        }
        Err(e) => {
            tracing::error!(
                error = %e,
                error_type = std::any::type_name_of_val(&e),
                backtrace = ?std::backtrace::Backtrace::capture(),
                "Failed to retrieve VictoriaLogs settings"
            );
            Err(ProblemDetails::internal_error(format!(
                "Failed to retrieve VictoriaLogs settings: {}",
                e
            )))
        }
    }
}

/// Update VictoriaLogs global settings
/// Also broadcasts the new settings to all connected EAs
pub async fn update_vlogs_settings(
    State(state): State<AppState>,
    Json(settings): Json<VLogsGlobalSettings>,
) -> Result<StatusCode, ProblemDetails> {
    let span = tracing::info_span!("update_vlogs_settings");
    let _enter = span.enter();

    // Validate endpoint URL
    if settings.enabled && settings.endpoint.is_empty() {
        tracing::warn!("Attempted to enable VictoriaLogs with empty endpoint");
        return Err(ProblemDetails::validation_error(
            "Endpoint URL is required when VictoriaLogs is enabled",
        ));
    }

    // Validate batch_size
    if settings.batch_size < 1 || settings.batch_size > 10000 {
        tracing::warn!(batch_size = settings.batch_size, "Invalid batch_size value");
        return Err(ProblemDetails::validation_error(
            "batch_size must be between 1 and 10000",
        ));
    }

    // Validate flush_interval_secs
    if settings.flush_interval_secs < 1 || settings.flush_interval_secs > 3600 {
        tracing::warn!(
            flush_interval_secs = settings.flush_interval_secs,
            "Invalid flush_interval_secs value"
        );
        return Err(ProblemDetails::validation_error(
            "flush_interval_secs must be between 1 and 3600",
        ));
    }

    match state.db.save_vlogs_settings(&settings).await {
        Ok(_) => {
            tracing::info!(
                enabled = settings.enabled,
                endpoint = %settings.endpoint,
                batch_size = settings.batch_size,
                flush_interval_secs = settings.flush_interval_secs,
                "Successfully saved VictoriaLogs settings"
            );

            // Broadcast settings to all connected EAs
            broadcast_vlogs_config(&state, &settings).await;

            // Notify via WebSocket
            if let Ok(json) = serde_json::to_string(&settings) {
                let _ = state.tx.send(format!("vlogs_settings_updated:{}", json));
            }

            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            tracing::error!(
                error = %e,
                error_type = std::any::type_name_of_val(&e),
                backtrace = ?std::backtrace::Backtrace::capture(),
                "Failed to save VictoriaLogs settings"
            );
            Err(ProblemDetails::internal_error(format!(
                "Failed to save VictoriaLogs settings: {}",
                e
            )))
        }
    }
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
