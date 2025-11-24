//! Settings CRUD endpoint handlers
//!
//! Provides REST API endpoints for managing copy settings,
//! including create, read, update, delete, and toggle operations.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::{
    api::{helpers::*, AppState, ProblemDetails},
    models::{CopySettings, SlaveConfigMessage},
};

// Request/Response structs for API endpoints
// These are kept private as they are only used internally by handlers

#[derive(Debug, Deserialize)]
pub(crate) struct CreateSettingsRequest {
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

#[derive(Debug, Deserialize)]
pub(crate) struct ToggleRequest {
    status: i32, // 0=DISABLED, 1=ENABLED, 2=CONNECTED
}

pub async fn list_settings(
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

pub async fn get_settings(
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

pub async fn create_settings(
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

pub async fn update_settings(
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

pub async fn toggle_settings(
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

pub async fn delete_settings(
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
        let delete_config = SlaveConfigMessage {
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

        if let Err(e) = state.config_sender.send(&delete_config).await {
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
