// Tests for VictoriaLogs configuration API endpoints
//
// Tests:
// - GET /api/victoria-logs-config (configured and not configured cases)
// - PUT /api/victoria-logs-settings (toggle enabled state)

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

use crate::api::{
    create_router,
    victoria_logs_settings::{VLogsConfigResponse, VLogsToggleRequest},
};

use super::create_test_app_state_with_vlogs;

/// GET /api/victoria-logs-config should return configured=true when VictoriaLogs is set up
#[tokio::test]
async fn test_get_vlogs_config_when_configured() {
    let state = create_test_app_state_with_vlogs(true).await;
    let app = create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/victoria-logs-config")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let config: VLogsConfigResponse = serde_json::from_slice(&body).unwrap();

    assert!(config.configured);
    assert!(config.enabled);
    assert!(config.config.is_some());

    let config_info = config.config.unwrap();
    assert_eq!(config_info.host, "http://localhost:9428");
    assert_eq!(config_info.batch_size, 100);
    assert_eq!(config_info.flush_interval_secs, 5);
    assert_eq!(config_info.source, "test-relay");
}

/// GET /api/victoria-logs-config should return configured=false when VictoriaLogs is not set up
#[tokio::test]
async fn test_get_vlogs_config_when_not_configured() {
    let state = create_test_app_state_with_vlogs(false).await;
    let app = create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/victoria-logs-config")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let config: VLogsConfigResponse = serde_json::from_slice(&body).unwrap();

    assert!(!config.configured);
    assert!(!config.enabled);
    assert!(config.config.is_none());
}

/// PUT /api/victoria-logs-settings should toggle enabled state when configured
#[tokio::test]
async fn test_toggle_vlogs_enabled_when_configured() {
    let state = create_test_app_state_with_vlogs(true).await;
    let app = create_router(state);

    // First, verify initial state is enabled
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/victoria-logs-config")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let config: VLogsConfigResponse = serde_json::from_slice(&body).unwrap();
    assert!(config.enabled);

    // Toggle to disabled
    let toggle_request = VLogsToggleRequest {
        enabled: false,
        log_level: None,
    };
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/victoria-logs-settings")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&toggle_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // Verify state changed to disabled
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/victoria-logs-config")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let config: VLogsConfigResponse = serde_json::from_slice(&body).unwrap();
    assert!(!config.enabled);
}

/// PUT /api/victoria-logs-settings should return validation error when not configured
#[tokio::test]
async fn test_toggle_vlogs_enabled_when_not_configured() {
    let state = create_test_app_state_with_vlogs(false).await;
    let app = create_router(state);

    let toggle_request = VLogsToggleRequest {
        enabled: true,
        log_level: None,
    };
    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/victoria-logs-settings")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&toggle_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return 400 Bad Request (validation error)
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

/// PUT /api/victoria-logs-settings should work for enable -> disable -> enable cycle
#[tokio::test]
async fn test_toggle_vlogs_enabled_cycle() {
    let state = create_test_app_state_with_vlogs(true).await;
    let app = create_router(state);

    // Disable
    let toggle_request = VLogsToggleRequest {
        enabled: false,
        log_level: None,
    };
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/victoria-logs-settings")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&toggle_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // Re-enable
    let toggle_request = VLogsToggleRequest {
        enabled: true,
        log_level: None,
    };
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/victoria-logs-settings")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&toggle_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // Verify final state is enabled
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/victoria-logs-config")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let config: VLogsConfigResponse = serde_json::from_slice(&body).unwrap();
    assert!(config.enabled);
}
