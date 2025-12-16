use axum::{extract::State, Json};

use super::AppState;
use crate::runtime_status_updater::RuntimeStatusMetricsSnapshot;

/// Returns aggregated runtime status metrics for monitoring/observability.
pub async fn get_runtime_metrics(
    State(state): State<AppState>,
) -> Json<RuntimeStatusMetricsSnapshot> {
    let snapshot = state.runtime_status_metrics.snapshot();
    Json(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::inbound::http::test_helpers::create_test_app_state;
    use crate::runtime_status_updater::RuntimeStatusMetricsSnapshot;
    use axum::{body::Body, http::Request, routing::get, Router};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn runtime_metrics_endpoint_returns_snapshot() {
        let state = create_test_app_state().await;

        state.runtime_status_metrics.record_master_eval_success();
        state.runtime_status_metrics.record_master_eval_failure();
        state.runtime_status_metrics.record_slave_eval_success();
        state.runtime_status_metrics.record_slave_eval_failure();
        state.runtime_status_metrics.record_slave_bundle(3);

        let app = Router::new()
            .route("/api/runtime-status-metrics", get(get_runtime_metrics))
            .with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/runtime-status-metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("runtime metrics request failed");

        assert!(response.status().is_success());

        let body = response
            .into_body()
            .collect()
            .await
            .expect("failed to read body");
        let snapshot: RuntimeStatusMetricsSnapshot =
            serde_json::from_slice(&body.to_bytes()).expect("failed to deserialize snapshot");

        assert_eq!(snapshot.master_evaluations_total, 2);
        assert_eq!(snapshot.master_evaluations_failed, 1);
        assert_eq!(snapshot.slave_evaluations_total, 2);
        assert_eq!(snapshot.slave_evaluations_failed, 1);
        assert_eq!(snapshot.slave_bundles_built, 1);
        assert_eq!(snapshot.last_cluster_size, 3);
    }
}
