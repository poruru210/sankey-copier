// relay-server/src/api/trade_groups.rs
//
// REST API endpoints for TradeGroup management.
// Provides Master EA configuration endpoints for Web UI.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Serialize;

use crate::config_builder::{ConfigBuilder, MasterConfigContext, SlaveConfigContext};
use crate::models::{
    status_engine::{
        evaluate_master_status, ConnectionSnapshot, MasterIntent, MasterStatusResult, SlaveIntent,
    },
    MasterSettings, SlaveConfigWithMaster, TradeGroup, WarningCode,
};
use crate::runtime_status_updater::{RuntimeStatusUpdater, SlaveRuntimeTarget};

use super::{AppState, ProblemDetails};

/// API response view that augments TradeGroup with runtime status evaluated by the status engine.
#[derive(Debug, Clone, Serialize)]
pub struct TradeGroupRuntimeView {
    pub id: String,
    pub master_settings: MasterSettings,
    pub master_runtime_status: i32,
    pub master_warning_codes: Vec<WarningCode>,
    pub created_at: String,
    pub updated_at: String,
}

impl TradeGroupRuntimeView {
    fn new(trade_group: TradeGroup, master_runtime: MasterStatusResult) -> Self {
        Self {
            id: trade_group.id,
            master_settings: trade_group.master_settings,
            master_runtime_status: master_runtime.status,
            master_warning_codes: master_runtime.warning_codes,
            created_at: trade_group.created_at,
            updated_at: trade_group.updated_at,
        }
    }
}

/// List all TradeGroups (Master accounts and their settings)
pub async fn list_trade_groups(
    State(state): State<AppState>,
) -> Result<Json<Vec<TradeGroupRuntimeView>>, ProblemDetails> {
    let span = tracing::info_span!("list_trade_groups");
    let _enter = span.enter();

    match state.db.list_trade_groups().await {
        Ok(trade_groups) => {
            tracing::info!(
                count = trade_groups.len(),
                "Successfully retrieved trade groups"
            );

            let mut response = Vec::with_capacity(trade_groups.len());
            for trade_group in trade_groups {
                response.push(build_trade_group_response(&state, trade_group).await);
            }

            Ok(Json(response))
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
) -> Result<Json<TradeGroupRuntimeView>, ProblemDetails> {
    let span = tracing::info_span!("get_trade_group", master_account = %id);
    let _enter = span.enter();

    match state.db.get_trade_group(&id).await {
        Ok(Some(trade_group)) => {
            tracing::info!(
                master_account = %id,
                config_version = trade_group.master_settings.config_version,
                "Successfully retrieved trade group"
            );
            let response = build_trade_group_response(&state, trade_group).await;
            Ok(Json(response))
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

    match state
        .db
        .update_master_settings(&id, updated_settings.clone())
        .await
    {
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
                let response = build_trade_group_response(&state, tg).await;
                if let Ok(json) = serde_json::to_string(&response) {
                    let _ = state.tx.send(format!("trade_group_updated:{}", json));
                }
            }

            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            let error_msg = e.to_string();

            // Check if this is a "not found" error from the database layer
            if error_msg.contains("TradeGroup not found") {
                tracing::warn!(
                    master_account = %id,
                    "TradeGroup not found for update"
                );
                return Err(
                    ProblemDetails::not_found(format!("TradeGroup not found: {}", id))
                        .with_instance(format!("/api/trade-groups/{}", id)),
                );
            }

            tracing::error!(
                master_account = %id,
                error = %e,
                error_type = std::any::type_name_of_val(&e),
                backtrace = ?std::backtrace::Backtrace::capture(),
                "Failed to update Master settings"
            );
            Err(
                ProblemDetails::internal_error(format!("Failed to update Master settings: {}", e))
                    .with_instance(format!("/api/trade-groups/{}", id)),
            )
        }
    }
}

/// Delete a TradeGroup (CASCADE deletes all members)
pub async fn delete_trade_group(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ProblemDetails> {
    let span = tracing::info_span!(
        "delete_trade_group",
        master_account = %id
    );
    let _enter = span.enter();

    match state.db.delete_trade_group(&id).await {
        Ok(_) => {
            tracing::info!(
                master_account = %id,
                "Successfully deleted TradeGroup and all its members (CASCADE)"
            );

            // Notify via WebSocket with structured payload
            if let Ok(json) = serde_json::to_string(&serde_json::json!({"id": id})) {
                let _ = state.tx.send(format!("trade_group_deleted:{}", json));
            }

            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            tracing::error!(
                master_account = %id,
                error = %e,
                error_type = std::any::type_name_of_val(&e),
                backtrace = ?std::backtrace::Backtrace::capture(),
                "Failed to delete TradeGroup"
            );
            Err(
                ProblemDetails::internal_error(format!("Failed to delete TradeGroup: {}", e))
                    .with_instance(format!("/api/trade-groups/{}", id)),
            )
        }
    }
}

/// Request body for toggling Master enabled state
#[derive(Debug, serde::Deserialize)]
pub struct ToggleMasterRequest {
    pub enabled: bool,
}

/// Toggle Master enabled state
/// POST /api/trade-groups/{id}/toggle
pub async fn toggle_master(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<ToggleMasterRequest>,
) -> Result<Json<TradeGroupRuntimeView>, ProblemDetails> {
    let span = tracing::info_span!(
        "toggle_master",
        master_account = %id,
        enabled = body.enabled
    );
    let _enter = span.enter();

    // Get current trade group
    let trade_group = match state.db.get_trade_group(&id).await {
        Ok(Some(tg)) => tg,
        Ok(None) => {
            return Err(
                ProblemDetails::not_found(format!("TradeGroup '{}' not found", id))
                    .with_instance(format!("/api/trade-groups/{}/toggle", id)),
            );
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to get TradeGroup");
            return Err(
                ProblemDetails::internal_error(format!("Failed to get TradeGroup: {}", e))
                    .with_instance(format!("/api/trade-groups/{}/toggle", id)),
            );
        }
    };

    // Update settings with new enabled state
    let mut new_settings = trade_group.master_settings.clone();
    new_settings.enabled = body.enabled;
    new_settings.config_version += 1;

    // Save to database
    if let Err(e) = state
        .db
        .update_master_settings(&id, new_settings.clone())
        .await
    {
        tracing::error!(error = %e, "Failed to update Master settings");
        return Err(
            ProblemDetails::internal_error(format!("Failed to toggle Master: {}", e))
                .with_instance(format!("/api/trade-groups/{}/toggle", id)),
        );
    }

    tracing::info!(
        master_account = %id,
        enabled = body.enabled,
        config_version = new_settings.config_version,
        "Successfully toggled Master enabled state"
    );

    // Send config to Master EA via ZMQ
    send_config_to_master(&state, &id, &new_settings).await;

    // Send config to all connected Slave EAs via ZMQ
    // When Master switch changes, Slaves need to know the new Master status
    send_config_to_slaves(&state, &id, &new_settings).await;

    // Re-evaluate and broadcast all affected Slaves' warning_codes
    // Master toggle changes Slave status (e.g., adds/removes master_web_ui_disabled warning)
    reevaluate_and_broadcast_slaves(&state, &id).await;

    // Fetch updated trade group, broadcast runtime-aware payload, and return response
    match state.db.get_trade_group(&id).await {
        Ok(Some(updated_tg)) => {
            let response = build_trade_group_response(&state, updated_tg).await;
            if let Ok(json) = serde_json::to_string(&response) {
                let _ = state.tx.send(format!("trade_group_updated:{}", json));
            }
            Ok(Json(response))
        }
        Ok(None) => Err(
            ProblemDetails::internal_error("TradeGroup disappeared after update")
                .with_instance(format!("/api/trade-groups/{}/toggle", id)),
        ),
        Err(e) => Err(ProblemDetails::internal_error(format!(
            "Failed to fetch updated TradeGroup: {}",
            e
        ))
        .with_instance(format!("/api/trade-groups/{}/toggle", id))),
    }
}

async fn build_trade_group_response(
    state: &AppState,
    trade_group: TradeGroup,
) -> TradeGroupRuntimeView {
    let master_runtime = evaluate_master_runtime_status(state, &trade_group).await;
    TradeGroupRuntimeView::new(trade_group, master_runtime)
}

async fn evaluate_master_runtime_status(
    state: &AppState,
    trade_group: &TradeGroup,
) -> MasterStatusResult {
    let master_conn = state.connection_manager.get_master(&trade_group.id).await;
    let master_snapshot = ConnectionSnapshot {
        connection_status: master_conn.as_ref().map(|c| c.status),
        is_trade_allowed: master_conn
            .as_ref()
            .map(|c| c.is_trade_allowed)
            .unwrap_or(true),
    };

    evaluate_master_status(
        MasterIntent {
            web_ui_enabled: trade_group.master_settings.enabled,
        },
        master_snapshot,
    )
}

/// Send Master config to Master EA via ZMQ
async fn send_config_to_master(state: &AppState, master_account: &str, settings: &MasterSettings) {
    // Get Master connection info
    let master_conn = state.connection_manager.get_master(master_account).await;
    let master_snapshot = ConnectionSnapshot {
        connection_status: master_conn.as_ref().map(|c| c.status),
        is_trade_allowed: master_conn
            .as_ref()
            .map(|c| c.is_trade_allowed)
            .unwrap_or(true),
    };
    let is_trade_allowed = master_snapshot.is_trade_allowed;

    let bundle = ConfigBuilder::build_master_config(MasterConfigContext {
        account_id: master_account.to_string(),
        intent: MasterIntent {
            web_ui_enabled: settings.enabled,
        },
        connection_snapshot: master_snapshot,
        settings,
        timestamp: chrono::Utc::now(),
    });
    let status = bundle.status_result.status;
    let config = bundle.config;

    if let Err(e) = state.config_sender.send(&config).await {
        tracing::error!(
            master_account = %master_account,
            config_version = settings.config_version,
            status = status,
            enabled = settings.enabled,
            is_trade_allowed = is_trade_allowed,
            error = %e,
            "Failed to send MasterConfigMessage via ZMQ"
        );
    } else {
        tracing::info!(
            master_account = %master_account,
            config_version = settings.config_version,
            status = status,
            enabled = settings.enabled,
            is_trade_allowed = is_trade_allowed,
            "Successfully sent MasterConfigMessage via ZMQ"
        );
    }
}

/// Send config to all Slave EAs connected to this Master via ZMQ
/// Called when Master switch changes to notify Slaves of the new Master status
async fn send_config_to_slaves(state: &AppState, master_account: &str, settings: &MasterSettings) {
    // Get is_trade_allowed for Master (to calculate master_status)
    let master_conn = state.connection_manager.get_master(master_account).await;
    let master_snapshot = ConnectionSnapshot {
        connection_status: master_conn.as_ref().map(|c| c.status),
        is_trade_allowed: master_conn
            .as_ref()
            .map(|c| c.is_trade_allowed)
            .unwrap_or(true),
    };

    // Calculate Master status once and reuse for every Slave notification
    let master_status = evaluate_master_status(
        MasterIntent {
            web_ui_enabled: settings.enabled,
        },
        master_snapshot,
    );

    // Fetch Master's equity for margin_ratio mode
    let master_equity = master_conn.as_ref().map(|conn| conn.equity);

    // Get all members (Slaves) for this Master
    let members = match state.db.get_members(master_account).await {
        Ok(m) => m,
        Err(e) => {
            tracing::error!(
                master_account = %master_account,
                error = %e,
                "Failed to get members for Slave notification"
            );
            return;
        }
    };

    // Send config to each Slave
    for member in members {
        // Get Slave's is_trade_allowed from connection manager (if connected)
        let slave_conn = state.connection_manager.get_slave(&member.slave_account).await;
        let slave_snapshot = ConnectionSnapshot {
            connection_status: slave_conn.as_ref().map(|c| c.status),
            is_trade_allowed: slave_conn
                .as_ref()
                .map(|conn| conn.is_trade_allowed)
                .unwrap_or(true),
        };

        let bundle = ConfigBuilder::build_slave_config(SlaveConfigContext {
            slave_account: member.slave_account.clone(),
            master_account: master_account.to_string(),
            trade_group_id: master_account.to_string(),
            intent: SlaveIntent {
                web_ui_enabled: member.enabled_flag,
            },
            slave_connection_snapshot: slave_snapshot,
            master_status_result: master_status.clone(),
            slave_settings: &member.slave_settings,
            master_equity,
            timestamp: chrono::Utc::now(),
        });
        let config = bundle.config;
        let new_status = bundle.status_result.status;

        if let Err(e) = state.config_sender.send(&config).await {
            tracing::error!(
                slave_account = %member.slave_account,
                master_account = %master_account,
                error = %e,
                "Failed to send SlaveConfigMessage via ZMQ"
            );
        } else {
            tracing::info!(
                slave_account = %member.slave_account,
                master_account = %master_account,
                status = new_status,
                "Sent SlaveConfigMessage due to Master switch change"
            );
        }

        if let Err(e) = state
            .db
            .update_member_runtime_status(master_account, &member.slave_account, new_status)
            .await
        {
            tracing::error!(
                slave_account = %member.slave_account,
                master_account = %master_account,
                status = new_status,
                error = %e,
                "Failed to update member status after Master switch change"
            );
        }
    }
}

/// Re-evaluate all Slaves for a Master and broadcast warning_codes changes
///
/// Called after Master toggle to ensure Slave warning_codes reflect the new Master state.
/// Uses BroadcastCoordinator for change detection and WebSocket notifications.
async fn reevaluate_and_broadcast_slaves(state: &AppState, master_account: &str) {
    let runtime_updater = RuntimeStatusUpdater::with_metrics(
        state.db.clone(),
        state.connection_manager.clone(),
        state.runtime_status_metrics.clone(),
    );

    let members = match state.db.get_members(master_account).await {
        Ok(members) => members,
        Err(e) => {
            tracing::error!(
                master = %master_account,
                error = %e,
                "Failed to get members for Master toggle broadcast"
            );
            return;
        }
    };

    for member in members {
        let slave_bundle = runtime_updater
            .build_slave_bundle(SlaveRuntimeTarget {
                master_account,
                trade_group_id: master_account,
                slave_account: &member.slave_account,
                enabled_flag: member.enabled_flag,
                slave_settings: &member.slave_settings,
            })
            .await;

        let payload = SlaveConfigWithMaster {
            master_account: master_account.to_string(),
            slave_account: member.slave_account.clone(),
            status: slave_bundle.status_result.status,
            enabled_flag: member.enabled_flag,
            warning_codes: slave_bundle.status_result.warning_codes.clone(),
            slave_settings: member.slave_settings.clone(),
        };

        // Broadcast settings update to WebSocket clients
        if let Ok(json) = serde_json::to_string(&payload) {
            let _ = state.tx.send(format!("settings_updated:{}", json));
            tracing::info!(
                slave = %member.slave_account,
                master = %master_account,
                "Broadcasted Slave warning_codes after Master toggle"
            );
        }
    }
}
