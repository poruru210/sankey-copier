// relay-server/src/api/trade_groups.rs
//
// REST API endpoints for TradeGroup management.
// Provides Master EA configuration endpoints for Web UI.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sankey_copier_zmq::MasterConfigMessage;

use crate::models::{MasterSettings, TradeGroup};

use super::{AppState, ProblemDetails};

/// List all TradeGroups (Master accounts and their settings)
pub async fn list_trade_groups(
    State(state): State<AppState>,
) -> Result<Json<Vec<TradeGroup>>, ProblemDetails> {
    let span = tracing::info_span!("list_trade_groups");
    let _enter = span.enter();

    match state.db.list_trade_groups().await {
        Ok(trade_groups) => {
            tracing::info!(
                count = trade_groups.len(),
                "Successfully retrieved trade groups"
            );
            Ok(Json(trade_groups))
        }
        Err(e) => {
            tracing::error!(
                error = %e,
                error_type = std::any::type_name_of_val(&e),
                backtrace = ?std::backtrace::Backtrace::capture(),
                "Failed to list trade groups from database"
            );
            Err(ProblemDetails::internal_error(format!(
                "Failed to retrieve trade groups from database: {}",
                e
            )))
        }
    }
}

/// Get a specific TradeGroup by master_account ID
pub async fn get_trade_group(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<TradeGroup>, ProblemDetails> {
    let span = tracing::info_span!("get_trade_group", master_account = %id);
    let _enter = span.enter();

    match state.db.get_trade_group(&id).await {
        Ok(Some(trade_group)) => {
            tracing::info!(
                master_account = %id,
                config_version = trade_group.master_settings.config_version,
                "Successfully retrieved trade group"
            );
            Ok(Json(trade_group))
        }
        Ok(None) => {
            tracing::warn!(master_account = %id, "Trade group not found");
            Err(ProblemDetails::not_found(format!(
                "Trade group with master account '{}' was not found",
                id
            ))
            .with_instance(format!("/api/trade-groups/{}", id)))
        }
        Err(e) => {
            tracing::error!(
                master_account = %id,
                error = %e,
                error_type = std::any::type_name_of_val(&e),
                backtrace = ?std::backtrace::Backtrace::capture(),
                "Failed to retrieve trade group from database"
            );
            Err(ProblemDetails::internal_error(format!(
                "Failed to retrieve trade group from database: {}",
                e
            ))
            .with_instance(format!("/api/trade-groups/{}", id)))
        }
    }
}

/// Update Master settings for a TradeGroup
pub async fn update_trade_group_settings(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(settings): Json<MasterSettings>,
) -> Result<StatusCode, ProblemDetails> {
    let span = tracing::info_span!(
        "update_trade_group_settings",
        master_account = %id
    );
    let _enter = span.enter();

    // Increment config_version for the update
    let mut updated_settings = settings;
    updated_settings.config_version += 1;

    match state.db.update_master_settings(&id, updated_settings.clone()).await {
        Ok(_) => {
            tracing::info!(
                master_account = %id,
                config_version = updated_settings.config_version,
                symbol_prefix = ?updated_settings.symbol_prefix,
                symbol_suffix = ?updated_settings.symbol_suffix,
                "Successfully updated Master settings"
            );

            // Send updated config to Master EA via ZMQ
            send_config_to_master(&state, &id, &updated_settings).await;

            // Notify via WebSocket
            if let Ok(Some(tg)) = state.db.get_trade_group(&id).await {
                if let Ok(json) = serde_json::to_string(&tg) {
                    let _ = state.tx.send(format!("trade_group_updated:{}", json));
                }
            }

            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            tracing::error!(
                master_account = %id,
                error = %e,
                error_type = std::any::type_name_of_val(&e),
                backtrace = ?std::backtrace::Backtrace::capture(),
                "Failed to update Master settings"
            );
            Err(ProblemDetails::internal_error(format!(
                "Failed to update Master settings: {}",
                e
            ))
            .with_instance(format!("/api/trade-groups/{}", id)))
        }
    }
}

/// Send Master config to Master EA via ZMQ
async fn send_config_to_master(state: &AppState, master_account: &str, settings: &MasterSettings) {
    let config = MasterConfigMessage {
        account_id: master_account.to_string(),
        symbol_prefix: settings.symbol_prefix.clone(),
        symbol_suffix: settings.symbol_suffix.clone(),
        config_version: settings.config_version,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    if let Err(e) = state.config_sender.send_master_config(&config).await {
        tracing::error!(
            master_account = %master_account,
            config_version = settings.config_version,
            error = %e,
            "Failed to send MasterConfigMessage via ZMQ"
        );
    } else {
        tracing::info!(
            master_account = %master_account,
            config_version = settings.config_version,
            "Successfully sent MasterConfigMessage via ZMQ"
        );
    }
}
