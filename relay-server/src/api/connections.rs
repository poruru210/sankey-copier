//! Connection endpoint handlers
//!
//! Provides REST API endpoints for retrieving EA connection information.

use axum::{
    extract::{Path, State},
    Json,
};

use crate::{
    api::{AppState, ProblemDetails},
    models::EaConnection,
};

/// List all EA connections
pub async fn list_connections(
    State(state): State<AppState>,
) -> Result<Json<Vec<EaConnection>>, ProblemDetails> {
    let span = tracing::info_span!("list_connections");
    let _enter = span.enter();

    let connections = state.connection_manager.get_all_eas().await;

    tracing::info!(
        count = connections.len(),
        "Successfully retrieved EA connections"
    );

    Ok(Json(connections))
}

/// Get EA connection(s) by account ID
/// Returns all EAs (Master and/or Slave) associated with the account
pub async fn get_connection(
    State(state): State<AppState>,
    Path(account_id): Path<String>,
) -> Result<Json<Vec<EaConnection>>, ProblemDetails> {
    let span = tracing::info_span!("get_connection", account_id = %account_id);
    let _enter = span.enter();

    let connections = state
        .connection_manager
        .get_eas_by_account(&account_id)
        .await;

    if connections.is_empty() {
        tracing::warn!(
            account_id = %account_id,
            "EA connection not found"
        );
        Err(ProblemDetails::not_found("EA connection")
            .with_instance(format!("/api/connections/{}", account_id)))
    } else {
        tracing::info!(
            account_id = %account_id,
            count = connections.len(),
            "Successfully retrieved EA connection(s)"
        );
        Ok(Json(connections))
    }
}
