//! Logs endpoint handler
//!
//! Provides REST API endpoint for retrieving server logs.

use axum::{extract::State, Json};

use crate::adapters::inbound::http::{AppState, ProblemDetails};

/// Get server logs from the log buffer
pub async fn get_logs(
    State(state): State<AppState>,
) -> Result<Json<Vec<crate::log_buffer::LogEntry>>, ProblemDetails> {
    let span = tracing::info_span!("get_logs");
    let _enter = span.enter();

    let buffer = state.log_buffer.read().await;
    let logs: Vec<_> = buffer.iter().cloned().collect();

    tracing::debug!(count = logs.len(), "Successfully retrieved server logs");

    Ok(Json(logs))
}
