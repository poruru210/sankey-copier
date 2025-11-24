// relay-server/src/api/trade_group_members.rs
//
// REST API endpoints for TradeGroupMember management.
// Provides Slave EA configuration endpoints for Web UI.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sankey_copier_zmq::SlaveConfigMessage;
use serde::{Deserialize, Serialize};

use crate::models::{MasterSettings, SlaveSettings, TradeGroupMember};

use super::{AppState, ProblemDetails};

/// Request body for adding a new member to a TradeGroup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddMemberRequest {
    pub slave_account: String,
    #[serde(default)]
    pub slave_settings: SlaveSettings,
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

    // Verify TradeGroup exists
    match state.db.get_trade_group(&trade_group_id).await {
        Ok(None) => {
            tracing::warn!(
                trade_group_id = %trade_group_id,
                "TradeGroup not found when adding member"
            );
            return Err(ProblemDetails::not_found(format!(
                "TradeGroup '{}' not found",
                trade_group_id
            ))
            .with_instance(format!("/api/trade-groups/{}/members", trade_group_id)));
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

    // Add member to database
    match state
        .db
        .add_member(
            &trade_group_id,
            &request.slave_account,
            request.slave_settings.clone(),
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

/// Send Slave config to Slave EA via ZMQ
async fn send_config_to_slave(state: &AppState, master_account: &str, member: &TradeGroupMember) {
    // Fetch Master settings to include symbol_prefix/suffix
    let master_settings = match state.db.get_trade_group(master_account).await {
        Ok(Some(tg)) => tg.master_settings,
        Ok(None) => {
            tracing::warn!(
                master_account = %master_account,
                "Master settings not found when sending Slave config"
            );
            MasterSettings::default()
        }
        Err(e) => {
            tracing::error!(
                master_account = %master_account,
                error = %e,
                "Failed to fetch Master settings for Slave config"
            );
            MasterSettings::default()
        }
    };

    let config = SlaveConfigMessage {
        account_id: member.slave_account.clone(),
        master_account: master_account.to_string(),
        status: member.status,
        lot_multiplier: member.slave_settings.lot_multiplier,
        reverse_trade: member.slave_settings.reverse_trade,
        symbol_prefix: master_settings.symbol_prefix,
        symbol_suffix: master_settings.symbol_suffix,
        symbol_mappings: member.slave_settings.symbol_mappings.clone(),
        filters: member.slave_settings.filters.clone(),
        config_version: member.slave_settings.config_version,
        timestamp: chrono::Utc::now().to_rfc3339(),
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
