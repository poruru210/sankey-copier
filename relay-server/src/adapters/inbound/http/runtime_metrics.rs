use axum::{extract::State, Json};

use super::AppState;
use crate::application::runtime_status_updater::RuntimeStatusMetricsSnapshot;

/// Returns aggregated runtime status metrics for monitoring/observability.
pub async fn get_runtime_metrics(
    State(state): State<AppState>,
) -> Json<RuntimeStatusMetricsSnapshot> {
    let snapshot = state.runtime_status_metrics.snapshot();
    Json(snapshot)
}
