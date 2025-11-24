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

/// Get specific EA connection by account ID
pub async fn get_connection(
    State(state): State<AppState>,
    Path(account_id): Path<String>,
) -> Result<Json<EaConnection>, ProblemDetails> {
    let span = tracing::info_span!("get_connection", account_id = %account_id);
    let _enter = span.enter();

    match state.connection_manager.get_ea(&account_id).await {
        Some(connection) => {
            tracing::info!(
                account_id = %account_id,
                ea_type = ?connection.ea_type,
                status = ?connection.status,
                "Successfully retrieved EA connection"
            );
            Ok(Json(connection))
        }
        None => {
            tracing::warn!(
                account_id = %account_id,
                "EA connection not found"
            );
            Err(ProblemDetails::not_found("EA connection")
                .with_instance(format!("/api/connections/{}", account_id)))
        }
    }
}
