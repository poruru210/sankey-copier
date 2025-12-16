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
// use serde::{Deserialize, Serialize};

use crate::domain::models::{SlaveSettings, TradeGroupMember, STATUS_NO_CONFIG};
use crate::domain::services::status_calculator::SlaveRuntimeTarget;
use crate::runtime_status_updater::RuntimeStatusUpdater;

use super::{AppState, ProblemDetails};
use crate::adapters::inbound::http::dtos::{AddMemberRequest, ToggleStatusRequest};

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

            let runtime_updater = runtime_status_updater_for(&state);
            let mut hydrated = Vec::with_capacity(members.len());
            for member in members {
                hydrated.push(hydrate_member_runtime(&runtime_updater, member).await);
            }

            Ok(Json(hydrated))
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

                    // Send initial config to Master EA so it can display the TradeGroup
                    send_initial_config_to_master(&state, &trade_group_id).await;
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
                    let runtime_updater = runtime_status_updater_for(&state);
                    let hydrated_member = hydrate_member_runtime(&runtime_updater, member).await;

                    // Send config to Slave EA via ZMQ
                    send_config_to_slave(&state, &trade_group_id, &hydrated_member).await;

                    // Notify via WebSocket
                    if let Ok(json) = serde_json::to_string(&hydrated_member) {
                        let _ = state.tx.send(format!("member_added:{}", json));
                    }

                    Ok((StatusCode::CREATED, Json(hydrated_member)))
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
            let runtime_updater = runtime_status_updater_for(&state);
            let hydrated_member = hydrate_member_runtime(&runtime_updater, member).await;
            Ok(Json(hydrated_member))
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
                let runtime_updater = runtime_status_updater_for(&state);
                let hydrated_member = hydrate_member_runtime(&runtime_updater, member).await;

                // Send updated config to Slave EA via ZMQ
                send_config_to_slave(&state, &trade_group_id, &hydrated_member).await;

                // Notify via WebSocket
                if let Ok(json) = serde_json::to_string(&hydrated_member) {
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
        .update_member_enabled_flag(&trade_group_id, &slave_account, request.enabled)
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
                let runtime_updater = runtime_status_updater_for(&state);
                let hydrated_member = hydrate_member_runtime(&runtime_updater, member).await;

                // Send updated config to Slave EA via ZMQ (with updated status)
                send_config_to_slave(&state, &trade_group_id, &hydrated_member).await;

                // Notify via WebSocket
                if let Ok(json) = serde_json::to_string(&hydrated_member) {
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

fn runtime_status_updater_for(state: &AppState) -> RuntimeStatusUpdater {
    RuntimeStatusUpdater::with_metrics(
        state.db.clone(),
        state.connection_manager.clone(),
        state.runtime_status_metrics.clone(),
    )
}

async fn hydrate_member_runtime(
    runtime_updater: &RuntimeStatusUpdater,
    member: TradeGroupMember,
) -> TradeGroupMember {
    let mut member = member;
    let status_result = runtime_updater
        .evaluate_member_runtime_status(SlaveRuntimeTarget {
            master_account: member.trade_group_id.as_str(),
            trade_group_id: member.trade_group_id.as_str(),
            slave_account: member.slave_account.as_str(),
            enabled_flag: member.enabled_flag,
            slave_settings: &member.slave_settings,
        })
        .await;

    member.status = status_result.status;
    member.warning_codes = status_result.warning_codes;
    member
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
        filters: crate::domain::models::TradeFilters::default(),
        config_version: 0,
        source_lot_min: None,
        source_lot_max: None,
        master_equity: None,
        timestamp: chrono::Utc::now().timestamp_millis(),
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
        warning_codes: Vec::new(),
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
        timestamp: chrono::Utc::now().timestamp_millis(),
        warning_codes: Vec::new(),
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
    let runtime_updater = runtime_status_updater_for(state);
    let slave_bundle = runtime_updater
        .build_slave_bundle(SlaveRuntimeTarget {
            master_account,
            trade_group_id: master_account,
            slave_account: &member.slave_account,
            enabled_flag: member.enabled_flag,
            slave_settings: &member.slave_settings,
        })
        .await;
    let slave_status = slave_bundle.status_result.status;
    let config = slave_bundle.config;

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

    if let Err(e) = state
        .db
        .update_member_runtime_status(master_account, &member.slave_account, slave_status)
        .await
    {
        tracing::error!(
            slave_account = %member.slave_account,
            master_account = %master_account,
            status = slave_status,
            error = %e,
            "Failed to persist Slave runtime status"
        );
    }
}

/// Send initial config to Master EA via ZMQ when TradeGroup is newly created
/// This allows the Master EA to display the TradeGroup even with default settings
async fn send_initial_config_to_master(state: &AppState, master_account: &str) {
    use crate::domain::models::MasterSettings;

    // Get Master connection info (may not exist yet if Master EA is not connected)
    let _master_conn = state.connection_manager.get_master(master_account).await;

    // Use default settings (enabled=false) for newly created TradeGroup
    let settings = MasterSettings::default();

    let config = MasterConfigMessage {
        account_id: master_account.to_string(),
        status: 0, // DISABLED - Master is created but not yet enabled
        symbol_prefix: settings.symbol_prefix.clone(),
        symbol_suffix: settings.symbol_suffix.clone(),
        config_version: settings.config_version,
        timestamp: chrono::Utc::now().timestamp_millis(),
        warning_codes: Vec::new(),
    };

    if let Err(e) = state.config_sender.send(&config).await {
        tracing::error!(
            master_account = %master_account,
            error = %e,
            "Failed to send initial MasterConfigMessage via ZMQ"
        );
    } else {
        tracing::info!(
            master_account = %master_account,
            status = 0,
            "Successfully sent initial MasterConfigMessage via ZMQ (TradeGroup created)"
        );
    }
}

#[cfg(test)]
mod tests {
    // use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    use crate::adapters::inbound::http::{
        create_router,
        dtos::{AddMemberRequest, ToggleStatusRequest},
    };
    use crate::domain::models::{
        LotCalculationMode, MasterSettings, SlaveSettings, SymbolMapping, TradeFilters,
    };

    use crate::adapters::inbound::http::test_helpers::create_test_app_state;

    /// Helper function to create a test TradeGroup (Master)
    async fn setup_test_trade_group(
        state: &crate::adapters::inbound::http::AppState,
        master_account: &str,
    ) {
        // First create the TradeGroup
        state.db.create_trade_group(master_account).await.unwrap();

        // Then update its settings
        let settings = MasterSettings {
            enabled: true,
            symbol_prefix: Some("pro.".to_string()),
            symbol_suffix: Some(".m".to_string()),
            config_version: 1,
        };

        state
            .db
            .update_master_settings(master_account, settings)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_list_members_empty() {
        let state = create_test_app_state().await;
        setup_test_trade_group(&state, "MASTER_001").await;

        let app = create_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/trade-groups/MASTER_001/members")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let members: Vec<crate::domain::models::TradeGroupMember> =
            serde_json::from_slice(&body).unwrap();

        assert_eq!(members.len(), 0);
    }

    #[tokio::test]
    async fn test_add_member_success() {
        let state = create_test_app_state().await;
        setup_test_trade_group(&state, "MASTER_001").await;

        let app = create_router(state);

        let request_body = AddMemberRequest {
            slave_account: "SLAVE_001".to_string(),
            slave_settings: SlaveSettings {
                lot_calculation_mode: LotCalculationMode::default(),
                lot_multiplier: Some(1.5),
                reverse_trade: false,
                symbol_prefix: None,
                symbol_suffix: None,
                symbol_mappings: vec![SymbolMapping {
                    source_symbol: "EURUSD".to_string(),
                    target_symbol: "EURUSDm".to_string(),
                }],
                filters: TradeFilters {
                    allowed_symbols: Some(vec!["EURUSD".to_string()]),
                    blocked_symbols: None,
                    allowed_magic_numbers: None,
                    blocked_magic_numbers: None,
                },
                config_version: 0,
                source_lot_min: None,
                source_lot_max: None,
                sync_mode: crate::domain::models::SyncMode::Skip,
                limit_order_expiry_min: None,
                market_sync_max_pips: None,
                max_slippage: None,
                copy_pending_orders: false,
                max_retries: 3,
                max_signal_delay_ms: 5000,
                use_pending_order_for_delayed: false,
            },
            status: 0,
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/trade-groups/MASTER_001/members")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let member: crate::domain::models::TradeGroupMember =
            serde_json::from_slice(&body).unwrap();

        assert_eq!(member.slave_account, "SLAVE_001");
        assert_eq!(member.trade_group_id, "MASTER_001");
        assert_eq!(member.status, 0); // DISABLED - initial status is OFF
        assert_eq!(member.slave_settings.lot_multiplier, Some(1.5));
        assert!(!member.slave_settings.reverse_trade);
    }

    #[tokio::test]
    async fn test_add_member_auto_creates_trade_group() {
        let state = create_test_app_state().await;
        let app = create_router(state.clone());

        let request_body = AddMemberRequest {
            slave_account: "SLAVE_001".to_string(),
            slave_settings: SlaveSettings::default(),
            status: 0,
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/trade-groups/NONEXISTENT/members")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should auto-create TradeGroup and return CREATED
        assert_eq!(response.status(), StatusCode::CREATED);

        // Verify TradeGroup was created
        let trade_group = state.db.get_trade_group("NONEXISTENT").await.unwrap();
        assert!(trade_group.is_some());

        // Verify member was added
        let member = state
            .db
            .get_member("NONEXISTENT", "SLAVE_001")
            .await
            .unwrap();
        assert!(member.is_some());
    }

    #[tokio::test]
    async fn test_get_member_success() {
        let state = create_test_app_state().await;
        setup_test_trade_group(&state, "MASTER_001").await;

        // Add a member first
        let settings = SlaveSettings {
            lot_multiplier: Some(2.0),
            reverse_trade: true,
            ..Default::default()
        };
        state
            .db
            .add_member("MASTER_001", "SLAVE_001", settings, 0)
            .await
            .unwrap();

        let app = create_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/trade-groups/MASTER_001/members/SLAVE_001")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let member: crate::domain::models::TradeGroupMember =
            serde_json::from_slice(&body).unwrap();

        assert_eq!(member.slave_account, "SLAVE_001");
        assert_eq!(member.slave_settings.lot_multiplier, Some(2.0));
        assert!(member.slave_settings.reverse_trade);
    }

    #[tokio::test]
    async fn test_get_member_not_found() {
        let state = create_test_app_state().await;
        setup_test_trade_group(&state, "MASTER_001").await;

        let app = create_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/trade-groups/MASTER_001/members/NONEXISTENT")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_update_member_success() {
        let state = create_test_app_state().await;
        setup_test_trade_group(&state, "MASTER_001").await;

        // Add a member first
        let settings = SlaveSettings {
            lot_multiplier: Some(1.0),
            reverse_trade: false,
            config_version: 0,
            ..Default::default()
        };
        state
            .db
            .add_member("MASTER_001", "SLAVE_001", settings, 0)
            .await
            .unwrap();

        let app = create_router(state.clone());

        // Update the member
        let updated_settings = SlaveSettings {
            lot_multiplier: Some(3.0),
            reverse_trade: true,
            config_version: 0, // Will be incremented by handler
            ..Default::default()
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/trade-groups/MASTER_001/members/SLAVE_001")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&updated_settings).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        // Verify the update
        let member = state
            .db
            .get_member("MASTER_001", "SLAVE_001")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(member.slave_settings.lot_multiplier, Some(3.0));
        assert!(member.slave_settings.reverse_trade);
        assert_eq!(member.slave_settings.config_version, 1); // Incremented
    }

    #[tokio::test]
    async fn test_toggle_member_status_disable() {
        let state = create_test_app_state().await;
        setup_test_trade_group(&state, "MASTER_001").await;

        // Add a member (initial status = DISABLED)
        state
            .db
            .add_member("MASTER_001", "SLAVE_001", SlaveSettings::default(), 0)
            .await
            .unwrap();

        // First enable the member
        state
            .db
            .update_member_enabled_flag("MASTER_001", "SLAVE_001", true)
            .await
            .unwrap();
        state
            .db
            .update_member_runtime_status("MASTER_001", "SLAVE_001", 1)
            .await
            .unwrap();

        let app = create_router(state.clone());

        // Toggle to DISABLED
        let toggle_request = ToggleStatusRequest { enabled: false };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/trade-groups/MASTER_001/members/SLAVE_001/toggle")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&toggle_request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        // Verify status changed
        let member = state
            .db
            .get_member("MASTER_001", "SLAVE_001")
            .await
            .unwrap()
            .unwrap();

        assert!(!member.enabled_flag);
        assert_eq!(member.status, 0); // DISABLED
    }

    #[tokio::test]
    async fn test_toggle_member_status_enable() {
        let state = create_test_app_state().await;
        setup_test_trade_group(&state, "MASTER_001").await;

        // Add a member (initial status = DISABLED)
        state
            .db
            .add_member("MASTER_001", "SLAVE_001", SlaveSettings::default(), 0)
            .await
            .unwrap();

        // Verify initial status is DISABLED
        let member = state
            .db
            .get_member("MASTER_001", "SLAVE_001")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(member.status, 0); // Initial status is DISABLED

        let app = create_router(state.clone());

        // Toggle to ENABLED
        let toggle_request = ToggleStatusRequest { enabled: true };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/trade-groups/MASTER_001/members/SLAVE_001/toggle")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&toggle_request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        // Verify status changed
        let member = state
            .db
            .get_member("MASTER_001", "SLAVE_001")
            .await
            .unwrap()
            .unwrap();

        assert!(member.enabled_flag);
        assert_eq!(member.status, 0); // runtime stays DISABLED until heartbeat evaluates
    }

    #[tokio::test]
    async fn test_delete_member_success() {
        let state = create_test_app_state().await;
        setup_test_trade_group(&state, "MASTER_001").await;

        // Add a member
        state
            .db
            .add_member("MASTER_001", "SLAVE_001", SlaveSettings::default(), 0)
            .await
            .unwrap();

        let app = create_router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/trade-groups/MASTER_001/members/SLAVE_001")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        // Verify member was deleted
        let member = state
            .db
            .get_member("MASTER_001", "SLAVE_001")
            .await
            .unwrap();

        assert!(member.is_none());
    }

    #[tokio::test]
    async fn test_list_members_with_multiple_members() {
        let state = create_test_app_state().await;
        setup_test_trade_group(&state, "MASTER_001").await;

        // Add multiple members
        for i in 1..=3 {
            let settings = SlaveSettings {
                lot_multiplier: Some(i as f64),
                ..Default::default()
            };
            state
                .db
                .add_member("MASTER_001", &format!("SLAVE_00{}", i), settings, 0)
                .await
                .unwrap();
        }

        let app = create_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/trade-groups/MASTER_001/members")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let members: Vec<crate::domain::models::TradeGroupMember> =
            serde_json::from_slice(&body).unwrap();

        assert_eq!(members.len(), 3);
        assert_eq!(members[0].slave_account, "SLAVE_001");
        assert_eq!(members[1].slave_account, "SLAVE_002");
        assert_eq!(members[2].slave_account, "SLAVE_003");
    }

    #[tokio::test]
    async fn test_member_with_complex_settings() {
        let state = create_test_app_state().await;
        setup_test_trade_group(&state, "MASTER_001").await;

        let app = create_router(state);

        let request_body = AddMemberRequest {
            slave_account: "SLAVE_COMPLEX".to_string(),
            slave_settings: SlaveSettings {
                lot_calculation_mode: LotCalculationMode::default(),
                lot_multiplier: Some(2.5),
                reverse_trade: true,
                symbol_prefix: Some("FX_".to_string()),
                symbol_suffix: Some(".m".to_string()),
                symbol_mappings: vec![
                    SymbolMapping {
                        source_symbol: "EURUSD".to_string(),
                        target_symbol: "EUR.USD".to_string(),
                    },
                    SymbolMapping {
                        source_symbol: "GBPUSD".to_string(),
                        target_symbol: "GBP.USD".to_string(),
                    },
                ],
                filters: TradeFilters {
                    allowed_symbols: Some(vec!["EURUSD".to_string(), "GBPUSD".to_string()]),
                    blocked_symbols: Some(vec!["USDJPY".to_string()]),
                    allowed_magic_numbers: Some(vec![12345, 67890]),
                    blocked_magic_numbers: Some(vec![99999]),
                },
                config_version: 0,
                source_lot_min: None,
                source_lot_max: None,
                sync_mode: crate::domain::models::SyncMode::Skip,
                limit_order_expiry_min: None,
                market_sync_max_pips: None,
                max_slippage: None,
                copy_pending_orders: false,
                max_retries: 3,
                max_signal_delay_ms: 5000,
                use_pending_order_for_delayed: false,
            },
            status: 0,
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/trade-groups/MASTER_001/members")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let member: crate::domain::models::TradeGroupMember =
            serde_json::from_slice(&body).unwrap();

        assert_eq!(member.slave_account, "SLAVE_COMPLEX");
        assert_eq!(member.slave_settings.lot_multiplier, Some(2.5));
        assert!(member.slave_settings.reverse_trade);
        assert_eq!(member.slave_settings.symbol_prefix, Some("FX_".to_string()));
        assert_eq!(member.slave_settings.symbol_mappings.len(), 2);
        assert_eq!(
            member
                .slave_settings
                .filters
                .allowed_symbols
                .as_ref()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            member
                .slave_settings
                .filters
                .blocked_symbols
                .as_ref()
                .unwrap()
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn test_add_member_duplicate_conflict() {
        let state = create_test_app_state().await;
        setup_test_trade_group(&state, "MASTER_001").await;

        let slave_settings = SlaveSettings {
            lot_multiplier: Some(1.0),
            ..Default::default()
        };

        // Add the first member
        state
            .db
            .add_member("MASTER_001", "SLAVE_DUP", slave_settings.clone(), 0)
            .await
            .unwrap();

        let app = create_router(state);

        // Try to add the same member again
        let request_body = AddMemberRequest {
            slave_account: "SLAVE_DUP".to_string(),
            slave_settings,
            status: 0,
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/trade-groups/MASTER_001/members")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should return 409 Conflict or similar error
        assert!(response.status().is_client_error());
    }

    #[tokio::test]
    async fn test_update_member_not_found() {
        let state = create_test_app_state().await;
        setup_test_trade_group(&state, "MASTER_001").await;

        let app = create_router(state);

        let update_body = serde_json::json!({
            "slave_settings": {
                "lot_multiplier": 2.0,
                "reverse_trade": false,
                "symbol_mappings": [],
                "filters": {
                    "allowed_symbols": null,
                    "blocked_symbols": null,
                    "allowed_magic_numbers": null,
                    "blocked_magic_numbers": null
                },
                "config_version": 0
            },
            "status": 1
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/trade-groups/MASTER_001/members/NONEXISTENT_SLAVE")
                    .header("content-type", "application/json")
                    .body(Body::from(update_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should return 404 Not Found
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_delete_member_not_found() {
        let state = create_test_app_state().await;
        setup_test_trade_group(&state, "MASTER_001").await;

        let app = create_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/trade-groups/MASTER_001/members/NONEXISTENT_SLAVE")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should handle gracefully (either 404 or 204)
        assert!(
            response.status() == StatusCode::NOT_FOUND
                || response.status() == StatusCode::NO_CONTENT
        );
    }

    #[tokio::test]
    async fn test_toggle_member_status_not_found() {
        let state = create_test_app_state().await;
        setup_test_trade_group(&state, "MASTER_001").await;

        let app = create_router(state);

        let toggle_body = ToggleStatusRequest { enabled: true };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/trade-groups/MASTER_001/members/NONEXISTENT_SLAVE/toggle")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&toggle_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should return 404 Not Found
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
