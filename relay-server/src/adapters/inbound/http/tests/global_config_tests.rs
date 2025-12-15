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

    use crate::adapters::inbound::http::{
        create_router,
        victoria_logs_settings::{VLogsConfigResponse, VLogsUpdateRequest},
    };

    use super::create_test_app_state_with_vlogs;

    // ... (unchanged parts)

    // Toggle to disabled
    let toggle_request = VLogsUpdateRequest {
        enabled: Some(false),
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

    // ... (unchanged)

    let toggle_request = VLogsUpdateRequest {
        enabled: Some(true),
        log_level: None,
    };

    // ... (test_toggle_vlogs_enabled_when_not_configured)

    let toggle_request = VLogsUpdateRequest {
        enabled: Some(true),
        log_level: None,
    };

    // ... (test_toggle_vlogs_enabled_cycle)

    // Disable
    let toggle_request = VLogsUpdateRequest {
        enabled: Some(false),
        log_level: None,
    };

    // ...

    // Re-enable
    let toggle_request = VLogsUpdateRequest {
        enabled: Some(true),
        log_level: None,
    };
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
