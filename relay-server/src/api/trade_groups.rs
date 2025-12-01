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

use crate::models::{
    status::{
        calculate_master_status, calculate_slave_status, MasterStatusInput, SlaveStatusInput,
    },
    MasterSettings, SlaveConfigMessage, TradeGroup,
};

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
                if let Ok(json) = serde_json::to_string(&tg) {
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

            // Notify via WebSocket
            let _ = state.tx.send(format!("trade_group_deleted:{}", id));

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
) -> Result<Json<TradeGroup>, ProblemDetails> {
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

    // Notify via WebSocket
    let _ = state.tx.send(format!("trade_group_updated:{}", id));

    // Fetch and return updated trade group
    match state.db.get_trade_group(&id).await {
        Ok(Some(updated_tg)) => Ok(Json(updated_tg)),
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

/// Send Master config to Master EA via ZMQ
async fn send_config_to_master(state: &AppState, master_account: &str, settings: &MasterSettings) {
    // Get Master connection info
    let master_conn = state.connection_manager.get_ea(master_account).await;
    let is_trade_allowed = master_conn
        .as_ref()
        .map(|c| c.is_trade_allowed)
        .unwrap_or(true);

    // Calculate status using centralized logic (same as heartbeat handler)
    let status = calculate_master_status(&MasterStatusInput {
        web_ui_enabled: settings.enabled,
        connection_status: master_conn.as_ref().map(|c| c.status),
        is_trade_allowed,
    });

    let config = MasterConfigMessage {
        account_id: master_account.to_string(),
        status,
        symbol_prefix: settings.symbol_prefix.clone(),
        symbol_suffix: settings.symbol_suffix.clone(),
        config_version: settings.config_version,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

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
    let master_conn = state.connection_manager.get_ea(master_account).await;
    let master_is_trade_allowed = master_conn
        .as_ref()
        .map(|c| c.is_trade_allowed)
        .unwrap_or(true);

    // Calculate Master status
    let master_status = calculate_master_status(&MasterStatusInput {
        web_ui_enabled: settings.enabled,
        connection_status: master_conn.as_ref().map(|c| c.status),
        is_trade_allowed: master_is_trade_allowed,
    });

    // Fetch Master's equity for margin_ratio mode
    let master_equity = state
        .connection_manager
        .get_ea(master_account)
        .await
        .map(|conn| conn.equity);

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
        let slave_conn = state.connection_manager.get_ea(&member.slave_account).await;
        let slave_is_trade_allowed = slave_conn
            .as_ref()
            .map(|conn| conn.is_trade_allowed)
            .unwrap_or(true);

        // Calculate Slave status
        // Slave is enabled if member.status > 0 (was previously enabled in DB)
        let slave_enabled = member.status > 0;
        let slave_status = calculate_slave_status(&SlaveStatusInput {
            web_ui_enabled: slave_enabled,
            connection_status: slave_conn.as_ref().map(|c| c.status),
            is_trade_allowed: slave_is_trade_allowed,
            master_status,
        });

        let config = SlaveConfigMessage {
            account_id: member.slave_account.clone(),
            master_account: master_account.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            trade_group_id: master_account.to_string(),
            status: slave_status,
            lot_calculation_mode: member.slave_settings.lot_calculation_mode.clone().into(),
            lot_multiplier: member.slave_settings.lot_multiplier,
            reverse_trade: member.slave_settings.reverse_trade,
            symbol_mappings: member.slave_settings.symbol_mappings.clone(),
            filters: member.slave_settings.filters.clone(),
            config_version: member.slave_settings.config_version,
            symbol_prefix: member.slave_settings.symbol_prefix.clone(),
            symbol_suffix: member.slave_settings.symbol_suffix.clone(),
            source_lot_min: member.slave_settings.source_lot_min,
            source_lot_max: member.slave_settings.source_lot_max,
            master_equity,
            sync_mode: member.slave_settings.sync_mode.clone().into(),
            limit_order_expiry_min: member.slave_settings.limit_order_expiry_min,
            market_sync_max_pips: member.slave_settings.market_sync_max_pips,
            max_slippage: member.slave_settings.max_slippage,
            copy_pending_orders: member.slave_settings.copy_pending_orders,
            max_retries: member.slave_settings.max_retries,
            max_signal_delay_ms: member.slave_settings.max_signal_delay_ms,
            use_pending_order_for_delayed: member.slave_settings.use_pending_order_for_delayed,
            allow_new_orders: slave_status > 0,
        };

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
                status = slave_status,
                "Sent SlaveConfigMessage due to Master switch change"
            );
        }
    }
}
