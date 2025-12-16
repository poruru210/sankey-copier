// relay-server/tests/runtime_metrics_api_test.rs

use axum::{body::Body, http::Request, routing::get, Router};
use http_body_util::BodyExt;
use tower::ServiceExt;

use sankey_copier_relay_server::adapters::inbound::http::{
    get_runtime_metrics, AppState, SnapshotBroadcaster,
};
use sankey_copier_relay_server::adapters::infrastructure::connection_manager::ConnectionManager;
use sankey_copier_relay_server::adapters::infrastructure::log_buffer::create_log_buffer;
use sankey_copier_relay_server::adapters::infrastructure::port_resolver::ResolvedPorts;
use sankey_copier_relay_server::adapters::outbound::messaging::ZmqConfigPublisher;
use sankey_copier_relay_server::adapters::outbound::persistence::Database;
use sankey_copier_relay_server::application::runtime_status_updater::{
    RuntimeStatusMetrics, RuntimeStatusMetricsSnapshot,
};

use std::sync::Arc;
use tokio::sync::broadcast;

/// Helper function to create a test app state
async fn create_test_app_state() -> AppState {
    let db = Arc::new(Database::new("sqlite::memory:").await.unwrap());
    let connection_manager = Arc::new(ConnectionManager::new(30));
    let (broadcast_tx, _) = broadcast::channel::<String>(100);
    let log_buffer = create_log_buffer();

    // Create a dummy ZMQ config sender with ephemeral port
    let config_sender = Arc::new(ZmqConfigPublisher::new("tcp://127.0.0.1:0").unwrap());

    // 2-port architecture
    let resolved_ports = Arc::new(ResolvedPorts {
        http_port: 3000,
        receiver_port: 5555,
        sender_port: 5556,
        is_dynamic: false,
        generated_at: None,
    });

    // Create snapshot broadcaster
    let snapshot_broadcaster =
        SnapshotBroadcaster::new(broadcast_tx.clone(), connection_manager.clone(), db.clone());

    AppState {
        db: db.clone(),
        tx: broadcast_tx,
        connection_manager,
        config_sender,
        log_buffer,
        allowed_origins: vec![],
        cors_disabled: true,
        config: Arc::new(sankey_copier_relay_server::config::Config::default()),
        resolved_ports,
        vlogs_controller: None,
        runtime_status_metrics: Arc::new(RuntimeStatusMetrics::default()),
        snapshot_broadcaster,
    }
}

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
