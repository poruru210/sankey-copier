// relay-server/src/api/trade_group_members.rs
//
// REST API endpoints for TradeGroupMember management.
// Provides Slave EA configuration endpoints for Web UI.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sankey_copier_zmq::{MasterConfigMessage, SlaveConfigMessage};
use serde::{Deserialize, Serialize};

use crate::models::{SlaveSettings, TradeGroupMember, STATUS_NO_CONFIG};

use super::{AppState, ProblemDetails};

/// Request body for adding a new member to a TradeGroup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddMemberRequest {
    pub slave_account: String,
    #[serde(default)]
    pub slave_settings: SlaveSettings,
    /// Initial status for the member (0 = DISABLED, 2 = CONNECTED/enabled)
    /// Defaults to 0 (disabled) if not specified
    #[serde(default)]
    pub status: i32,
}

/// Request body for toggling member status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToggleStatusRequest {
    pub enabled: bool,
}

/// List all members (Slaves) for a TradeGroup
pub async fn list_members(
    State(state): State<AppState>,
    Path(trade_group_id): Path<String>,
) -> Result<Json<Vec<TradeGroupMember>>, ProblemDetails> {
    let span = tracing::info_span!("list_members", trade_group_id = %trade_group_id);
    let _enter = span.enter();

    match state.db.get_members(&trade_group_id).await {
        Ok(members) => {
            tracing::info!(
                trade_group_id = %trade_group_id,
                count = members.len(),
                "Successfully retrieved members"
            );
            Ok(Json(members))
        }
        Err(e) => {
            tracing::error!(
                trade_group_id = %trade_group_id,
                error = %e,
                error_type = std::any::type_name_of_val(&e),
                backtrace = ?std::backtrace::Backtrace::capture(),
                "Failed to list members from database"
            );
            Err(ProblemDetails::internal_error(format!(
                "Failed to retrieve members from database: {}",
                e
            ))
            .with_instance(format!("/api/trade-groups/{}/members", trade_group_id)))
        }
    }
}

/// Add a new member (Slave) to a TradeGroup
pub async fn add_member(
    State(state): State<AppState>,
    Path(trade_group_id): Path<String>,
    Json(request): Json<AddMemberRequest>,
) -> Result<(StatusCode, Json<TradeGroupMember>), ProblemDetails> {
    let span = tracing::info_span!(
        "add_member",
        trade_group_id = %trade_group_id,
        slave_account = %request.slave_account
    );
    let _enter = span.enter();

    // Verify TradeGroup exists, create if it doesn't
    match state.db.get_trade_group(&trade_group_id).await {
        Ok(None) => {
            // TradeGroup doesn't exist - create it with default Master settings
            tracing::info!(
                trade_group_id = %trade_group_id,
                "TradeGroup not found, creating with default settings"
            );

            match state.db.create_trade_group(&trade_group_id).await {
                Ok(_) => {
                    tracing::info!(
                        trade_group_id = %trade_group_id,
                        "Successfully created TradeGroup with default settings"
                    );
                }
                Err(e) => {
                    tracing::error!(
                        trade_group_id = %trade_group_id,
                        error = %e,
                        "Failed to create TradeGroup automatically"
                    );
                    return Err(ProblemDetails::internal_error(format!(
                        "Failed to create TradeGroup automatically: {}",
                        e
                    ))
                    .with_instance(format!("/api/trade-groups/{}/members", trade_group_id)));
                }
            }
        }
        Err(e) => {
            tracing::error!(
                trade_group_id = %trade_group_id,
                error = %e,
                "Failed to verify TradeGroup existence"
            );
            return Err(ProblemDetails::internal_error(format!(
                "Failed to verify TradeGroup existence: {}",
                e
            ))
            .with_instance(format!("/api/trade-groups/{}/members", trade_group_id)));
        }
        Ok(Some(_)) => {
            // TradeGroup exists, continue
        }
    }

    // Add member to database with the requested status
    match state
        .db
        .add_member(
            &trade_group_id,
            &request.slave_account,
            request.slave_settings.clone(),
            request.status,
        )
        .await
    {
        Ok(_) => {
            tracing::info!(
                trade_group_id = %trade_group_id,
                slave_account = %request.slave_account,
                "Successfully added member"
            );

            // Retrieve the newly created member
            match state
                .db
                .get_member(&trade_group_id, &request.slave_account)
                .await
            {
                Ok(Some(member)) => {
                    // Send config to Slave EA via ZMQ
                    send_config_to_slave(&state, &trade_group_id, &member).await;

                    // Notify via WebSocket
                    if let Ok(json) = serde_json::to_string(&member) {
                        let _ = state.tx.send(format!("member_added:{}", json));
                    }

                    Ok((StatusCode::CREATED, Json(member)))
                }
                Ok(None) => {
                    tracing::error!(
                        trade_group_id = %trade_group_id,
                        slave_account = %request.slave_account,
                        "Member not found after creation"
                    );
                    Err(ProblemDetails::internal_error(
                        "Member not found after creation".to_string(),
                    )
                    .with_instance(format!("/api/trade-groups/{}/members", trade_group_id)))
                }
                Err(e) => {
                    tracing::error!(
                        trade_group_id = %trade_group_id,
                        slave_account = %request.slave_account,
                        error = %e,
                        "Failed to retrieve member after creation"
                    );
                    Err(ProblemDetails::internal_error(format!(
                        "Failed to retrieve member after creation: {}",
                        e
                    ))
                    .with_instance(format!("/api/trade-groups/{}/members", trade_group_id)))
                }
            }
        }
        Err(e) => {
            let error_msg = e.to_string();

            // Check if this is a unique constraint violation (duplicate member)
            if error_msg.contains("UNIQUE constraint failed")
                || error_msg.contains("unique constraint")
            {
                tracing::warn!(
                    trade_group_id = %trade_group_id,
                    slave_account = %request.slave_account,
                    "Duplicate member - already exists"
                );
                return Err(ProblemDetails::validation_error(format!(
                    "Member already exists: {}",
                    request.slave_account
                ))
                .with_instance(format!("/api/trade-groups/{}/members", trade_group_id)));
            }

            tracing::error!(
                trade_group_id = %trade_group_id,
                slave_account = %request.slave_account,
                error = %e,
                error_type = std::any::type_name_of_val(&e),
                backtrace = ?std::backtrace::Backtrace::capture(),
                "Failed to add member to database"
            );
            Err(
                ProblemDetails::internal_error(format!("Failed to add member to database: {}", e))
                    .with_instance(format!("/api/trade-groups/{}/members", trade_group_id)),
            )
        }
    }
}

/// Get a specific member
pub async fn get_member(
    State(state): State<AppState>,
    Path((trade_group_id, slave_account)): Path<(String, String)>,
) -> Result<Json<TradeGroupMember>, ProblemDetails> {
    let span = tracing::info_span!(
        "get_member",
        trade_group_id = %trade_group_id,
        slave_account = %slave_account
    );
    let _enter = span.enter();

    match state.db.get_member(&trade_group_id, &slave_account).await {
        Ok(Some(member)) => {
            tracing::info!(
                trade_group_id = %trade_group_id,
                slave_account = %slave_account,
                config_version = member.slave_settings.config_version,
                "Successfully retrieved member"
            );
            Ok(Json(member))
        }
        Ok(None) => {
            tracing::warn!(
                trade_group_id = %trade_group_id,
                slave_account = %slave_account,
                "Member not found"
            );
            Err(ProblemDetails::not_found(format!(
                "Member '{}' not found in TradeGroup '{}'",
                slave_account, trade_group_id
            ))
            .with_instance(format!(
                "/api/trade-groups/{}/members/{}",
                trade_group_id, slave_account
            )))
        }
        Err(e) => {
            tracing::error!(
                trade_group_id = %trade_group_id,
                slave_account = %slave_account,
                error = %e,
                error_type = std::any::type_name_of_val(&e),
                backtrace = ?std::backtrace::Backtrace::capture(),
                "Failed to retrieve member from database"
            );
            Err(ProblemDetails::internal_error(format!(
                "Failed to retrieve member from database: {}",
                e
            ))
            .with_instance(format!(
                "/api/trade-groups/{}/members/{}",
                trade_group_id, slave_account
            )))
        }
    }
}

/// Update member settings
pub async fn update_member(
    State(state): State<AppState>,
    Path((trade_group_id, slave_account)): Path<(String, String)>,
    Json(settings): Json<SlaveSettings>,
) -> Result<StatusCode, ProblemDetails> {
    let span = tracing::info_span!(
        "update_member",
        trade_group_id = %trade_group_id,
        slave_account = %slave_account
    );
    let _enter = span.enter();

    // Increment config_version for the update
    let mut updated_settings = settings;
    updated_settings.config_version += 1;

    match state
        .db
        .update_member_settings(&trade_group_id, &slave_account, updated_settings.clone())
        .await
    {
        Ok(_) => {
            tracing::info!(
                trade_group_id = %trade_group_id,
                slave_account = %slave_account,
                config_version = updated_settings.config_version,
                lot_multiplier = ?updated_settings.lot_multiplier,
                reverse_trade = updated_settings.reverse_trade,
                symbol_prefix = ?updated_settings.symbol_prefix,
                symbol_suffix = ?updated_settings.symbol_suffix,
                "Successfully updated member settings"
            );

            // Retrieve updated member for ZMQ notification
            if let Ok(Some(member)) = state.db.get_member(&trade_group_id, &slave_account).await {
                // Send updated config to Slave EA via ZMQ
                send_config_to_slave(&state, &trade_group_id, &member).await;

                // Notify via WebSocket
                if let Ok(json) = serde_json::to_string(&member) {
                    let _ = state.tx.send(format!("member_updated:{}", json));
                }
            }

            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            let error_msg = e.to_string();

            // Check if this is a "not found" error from the database layer
            if error_msg.contains("Member not found") {
                tracing::warn!(
                    trade_group_id = %trade_group_id,
                    slave_account = %slave_account,
                    "Member not found for update"
                );
                return Err(ProblemDetails::not_found(format!(
                    "Member not found: {}",
                    slave_account
                ))
                .with_instance(format!(
                    "/api/trade-groups/{}/members/{}",
                    trade_group_id, slave_account
                )));
            }

            tracing::error!(
                trade_group_id = %trade_group_id,
                slave_account = %slave_account,
                error = %e,
                error_type = std::any::type_name_of_val(&e),
                backtrace = ?std::backtrace::Backtrace::capture(),
                "Failed to update member settings"
            );
            Err(
                ProblemDetails::internal_error(format!("Failed to update member settings: {}", e))
                    .with_instance(format!(
                        "/api/trade-groups/{}/members/{}",
                        trade_group_id, slave_account
                    )),
            )
        }
    }
}

/// Toggle member status (ENABLED ↔ DISABLED)
pub async fn toggle_member_status(
    State(state): State<AppState>,
    Path((trade_group_id, slave_account)): Path<(String, String)>,
    Json(request): Json<ToggleStatusRequest>,
) -> Result<StatusCode, ProblemDetails> {
    let span = tracing::info_span!(
        "toggle_member_status",
        trade_group_id = %trade_group_id,
        slave_account = %slave_account,
        enabled = request.enabled
    );
    let _enter = span.enter();

    // Map boolean to status code (0=DISABLED, 1=ENABLED)
    let new_status = if request.enabled { 1 } else { 0 };

    match state
        .db
        .update_member_status(&trade_group_id, &slave_account, new_status)
        .await
    {
        Ok(_) => {
            tracing::info!(
                trade_group_id = %trade_group_id,
                slave_account = %slave_account,
                status = new_status,
                "Successfully toggled member status"
            );

            // Retrieve updated member for ZMQ notification
            if let Ok(Some(member)) = state.db.get_member(&trade_group_id, &slave_account).await {
                // Send updated config to Slave EA via ZMQ (with updated status)
                send_config_to_slave(&state, &trade_group_id, &member).await;

                // Notify via WebSocket
                if let Ok(json) = serde_json::to_string(&member) {
                    let _ = state.tx.send(format!("member_status_changed:{}", json));
                }
            }

            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            let error_msg = e.to_string();

            // Check if this is a "not found" error from the database layer
            if error_msg.contains("Member not found") {
                tracing::warn!(
                    trade_group_id = %trade_group_id,
                    slave_account = %slave_account,
                    "Member not found for status toggle"
                );
                return Err(ProblemDetails::not_found(format!(
                    "Member not found: {}",
                    slave_account
                ))
                .with_instance(format!(
                    "/api/trade-groups/{}/members/{}/toggle",
                    trade_group_id, slave_account
                )));
            }

            tracing::error!(
                trade_group_id = %trade_group_id,
                slave_account = %slave_account,
                error = %e,
                error_type = std::any::type_name_of_val(&e),
                backtrace = ?std::backtrace::Backtrace::capture(),
                "Failed to toggle member status"
            );
            Err(
                ProblemDetails::internal_error(format!("Failed to toggle member status: {}", e))
                    .with_instance(format!(
                        "/api/trade-groups/{}/members/{}/toggle",
                        trade_group_id, slave_account
                    )),
            )
        }
    }
}

/// Delete a member
pub async fn delete_member(
    State(state): State<AppState>,
    Path((trade_group_id, slave_account)): Path<(String, String)>,
) -> Result<StatusCode, ProblemDetails> {
    let span = tracing::info_span!(
        "delete_member",
        trade_group_id = %trade_group_id,
        slave_account = %slave_account
    );
    let _enter = span.enter();

    // Before deleting, send status=REMOVED config to Slave EA
    send_disabled_config_to_slave(&state, &trade_group_id, &slave_account).await;

    match state
        .db
        .delete_member(&trade_group_id, &slave_account)
        .await
    {
        Ok(_) => {
            tracing::info!(
                trade_group_id = %trade_group_id,
                slave_account = %slave_account,
                "Successfully deleted member"
            );

            // Check if TradeGroup has any remaining members
            let remaining_members = state
                .db
                .get_members(&trade_group_id)
                .await
                .unwrap_or_default();
            if remaining_members.is_empty() {
                tracing::info!(
                    trade_group_id = %trade_group_id,
                    "No remaining members, deleting TradeGroup and notifying Master EA"
                );

                // Send REMOVED config to Master EA
                send_removed_config_to_master(&state, &trade_group_id).await;

                // Delete the TradeGroup from DB
                if let Err(e) = state.db.delete_trade_group(&trade_group_id).await {
                    tracing::error!(
                        trade_group_id = %trade_group_id,
                        error = %e,
                        "Failed to delete empty TradeGroup"
                    );
                } else {
                    tracing::info!(
                        trade_group_id = %trade_group_id,
                        "Successfully deleted empty TradeGroup"
                    );
                }
            }

            // Notify via WebSocket
            let event = serde_json::json!({
                "trade_group_id": trade_group_id,
                "slave_account": slave_account,
            });
            if let Ok(json) = serde_json::to_string(&event) {
                let _ = state.tx.send(format!("member_deleted:{}", json));
            }

            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            tracing::error!(
                trade_group_id = %trade_group_id,
                slave_account = %slave_account,
                error = %e,
                error_type = std::any::type_name_of_val(&e),
                backtrace = ?std::backtrace::Backtrace::capture(),
                "Failed to delete member"
            );
            Err(
                ProblemDetails::internal_error(format!("Failed to delete member: {}", e))
                    .with_instance(format!(
                        "/api/trade-groups/{}/members/{}",
                        trade_group_id, slave_account
                    )),
            )
        }
    }
}

/// Send disabled config (status=0) to Slave EA via ZMQ to remove config
async fn send_disabled_config_to_slave(
    state: &AppState,
    master_account: &str,
    slave_account: &str,
) {
    let config = SlaveConfigMessage {
        account_id: slave_account.to_string(),
        master_account: master_account.to_string(),
        status: STATUS_NO_CONFIG,
        lot_calculation_mode: sankey_copier_zmq::LotCalculationMode::default(),
        lot_multiplier: None,
        reverse_trade: false,
        symbol_prefix: None,
        symbol_suffix: None,
        symbol_mappings: vec![],
        filters: crate::models::TradeFilters::default(),
        config_version: 0,
        source_lot_min: None,
        source_lot_max: None,
        master_equity: None,
        timestamp: chrono::Utc::now().to_rfc3339(),
        trade_group_id: master_account.to_string(),
        // Open Sync Policy defaults
        sync_mode: sankey_copier_zmq::SyncMode::default(),
        limit_order_expiry_min: None,
        market_sync_max_pips: None,
        max_slippage: None,
        copy_pending_orders: false,
        // Trade Execution defaults
        max_retries: 3,
        max_signal_delay_ms: 5000,
        use_pending_order_for_delayed: false,
        // Disabled = no new orders allowed
        allow_new_orders: false,
    };

    if let Err(e) = state.config_sender.send(&config).await {
        tracing::error!(
            slave_account = %slave_account,
            master_account = %master_account,
            error = %e,
            "Failed to send disabled SlaveConfigMessage via ZMQ"
        );
    } else {
        tracing::info!(
            slave_account = %slave_account,
            master_account = %master_account,
            "Successfully sent disabled SlaveConfigMessage via ZMQ"
        );
    }
}

/// Send REMOVED config to Master EA via ZMQ (when TradeGroup is deleted)
async fn send_removed_config_to_master(state: &AppState, master_account: &str) {
    let config = MasterConfigMessage {
        account_id: master_account.to_string(),
        status: STATUS_NO_CONFIG,
        symbol_prefix: None,
        symbol_suffix: None,
        config_version: 0,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    if let Err(e) = state.config_sender.send(&config).await {
        tracing::error!(
            master_account = %master_account,
            error = %e,
            "Failed to send REMOVED MasterConfigMessage via ZMQ"
        );
    } else {
        tracing::info!(
            master_account = %master_account,
            "Successfully sent REMOVED MasterConfigMessage via ZMQ"
        );
    }
}

/// Send Slave config to Slave EA via ZMQ
async fn send_config_to_slave(state: &AppState, master_account: &str, member: &TradeGroupMember) {
    // Fetch Master's connection info to calculate master status
    let master_conn = state.connection_manager.get_ea(master_account).await;

    let master_equity = master_conn.as_ref().map(|conn| conn.equity);

    // Calculate Master status first (needed for Slave status)
    // Master is CONNECTED if enabled in DB AND is_trade_allowed is true AND connection is Online
    // Check both is_trade_allowed and connection status (Online/Timeout/Offline)
    let master_is_trade_allowed = master_conn
        .as_ref()
        .map(|c| c.is_effective_trade_allowed())
        .unwrap_or(false); // Default to false if Master not connected
                           // Note: We assume Master Web UI is enabled here because if it was disabled,
                           // the TradeGroup wouldn't be active or we'd be handling it differently.
                           // However, strictly speaking, we should check the TradeGroup's enabled state.
                           // For now, we'll assume if we are sending config, we want to calculate status based on connection.
                           // But wait, send_config_to_slave is called when adding/updating member.
                           // We need to know if Master is enabled.
                           // Let's fetch TradeGroup to be sure, or pass it in.
                           // For simplicity and performance, we'll assume Master is enabled if not passed,
                           // but correct way is to check TradeGroup.
                           // Actually, calculate_slave_status needs master_status.

    // Let's get Master's effective status.
    // We need to know if Master is enabled in Web UI.
    // Since we don't have TradeGroup here, we'll fetch it or assume enabled if not found (fallback).
    let master_web_ui_enabled = match state.db.get_trade_group(master_account).await {
        Ok(Some(tg)) => tg.master_settings.enabled,
        _ => true, // Fallback
    };

    let master_status =
        crate::models::status::calculate_master_status(&crate::models::status::MasterStatusInput {
            web_ui_enabled: master_web_ui_enabled,
            is_trade_allowed: master_is_trade_allowed,
        });

    // Fetch Slave's connection info for is_trade_allowed
    // Check both is_trade_allowed and connection status via helper method
    let slave_is_trade_allowed = state
        .connection_manager
        .get_ea(&member.slave_account)
        .await
        .map(|conn| conn.is_effective_trade_allowed())
        .unwrap_or(false); // Default to false if Slave not connected

    // Calculate Slave's effective status
    let effective_status =
        crate::models::status::calculate_slave_status(&crate::models::status::SlaveStatusInput {
            web_ui_enabled: member.status > 0, // member.status from DB is 0 (DISABLED) or 1 (ENABLED)
            is_trade_allowed: slave_is_trade_allowed,
            master_status,
        });

    let config = SlaveConfigMessage {
        account_id: member.slave_account.clone(),
        master_account: master_account.to_string(),
        trade_group_id: master_account.to_string(),
        status: effective_status,
        lot_calculation_mode: member.slave_settings.lot_calculation_mode.clone().into(),
        lot_multiplier: member.slave_settings.lot_multiplier,
        reverse_trade: member.slave_settings.reverse_trade,
        symbol_prefix: member.slave_settings.symbol_prefix.clone(),
        symbol_suffix: member.slave_settings.symbol_suffix.clone(),
        symbol_mappings: member.slave_settings.symbol_mappings.clone(),
        filters: member.slave_settings.filters.clone(),
        config_version: member.slave_settings.config_version,
        source_lot_min: member.slave_settings.source_lot_min,
        source_lot_max: member.slave_settings.source_lot_max,
        master_equity,
        timestamp: chrono::Utc::now().to_rfc3339(),
        // Open Sync Policy settings
        sync_mode: member.slave_settings.sync_mode.clone().into(),
        limit_order_expiry_min: member.slave_settings.limit_order_expiry_min,
        market_sync_max_pips: member.slave_settings.market_sync_max_pips,
        max_slippage: member.slave_settings.max_slippage,
        copy_pending_orders: member.slave_settings.copy_pending_orders,
        // Trade Execution settings
        max_retries: member.slave_settings.max_retries,
        max_signal_delay_ms: member.slave_settings.max_signal_delay_ms,
        use_pending_order_for_delayed: member.slave_settings.use_pending_order_for_delayed,
        // Derived from status: allow new orders when enabled
        allow_new_orders: effective_status == crate::models::STATUS_CONNECTED,
    };

    if let Err(e) = state.config_sender.send(&config).await {
        tracing::error!(
            slave_account = %member.slave_account,
            master_account = %master_account,
            config_version = member.slave_settings.config_version,
            error = %e,
            "Failed to send SlaveConfigMessage via ZMQ"
        );
    } else {
        tracing::info!(
            slave_account = %member.slave_account,
            master_account = %master_account,
            config_version = member.slave_settings.config_version,
            "Successfully sent SlaveConfigMessage via ZMQ"
        );
    }
}
