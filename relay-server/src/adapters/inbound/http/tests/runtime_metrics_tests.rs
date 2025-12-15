use axum::{body::Body, http::Request, routing::get, Router};
use http_body_util::BodyExt;
use tower::ServiceExt;

use super::create_test_app_state;
use crate::adapters::inbound::http::runtime_metrics::get_runtime_metrics;
use crate::runtime_status_updater::RuntimeStatusMetricsSnapshot;

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
