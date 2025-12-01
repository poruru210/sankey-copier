use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use tower::util::ServiceExt;

use sankey_copier_relay_server::api::create_router;
use sankey_copier_relay_server::api::AppState;
use sankey_copier_relay_server::connection_manager::ConnectionManager;
use sankey_copier_relay_server::db::Database;
use sankey_copier_relay_server::log_buffer::create_log_buffer;
use sankey_copier_relay_server::port_resolver::ResolvedPorts;
use sankey_copier_relay_server::zeromq::ZmqConfigPublisher;

use std::sync::Arc;
use tokio::sync::broadcast;

/// Helper function to create a test app
async fn create_test_app() -> axum::Router {
    let db = Arc::new(Database::new("sqlite::memory:").await.unwrap());
    let connection_manager = Arc::new(ConnectionManager::new(30));
    let (broadcast_tx, _) = broadcast::channel::<String>(100);
    let log_buffer = create_log_buffer();

    // Create a dummy ZMQ config sender
    let config_sender = Arc::new(ZmqConfigPublisher::new("tcp://127.0.0.1:0").unwrap());

    // 2-port architecture: receiver and unified publisher
    let resolved_ports = Arc::new(ResolvedPorts {
        receiver_port: 5555,
        sender_port: 5556,
        is_dynamic: false,
        generated_at: None,
    });

    let app_state = AppState {
        db,
        tx: broadcast_tx,
        connection_manager,
        config_sender,
        log_buffer,
        allowed_origins: vec!["http://localhost:8080".to_string()],
        cors_disabled: false, // Use strict CORS for tests
        config: Arc::new(sankey_copier_relay_server::config::Config::default()),
        resolved_ports,
        vlogs_controller: None,
    };

    create_router(app_state)
}

#[tokio::test]
async fn test_list_mt_installations_endpoint() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/mt-installations")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Check response structure (no longer wrapped in ApiResponse)
    assert!(json["success"].is_boolean());
    assert!(json["data"].is_array());
    assert!(json["detection_summary"].is_object());

    let summary = &json["detection_summary"];
    assert!(summary["total_found"].is_number());
}

#[tokio::test]
async fn test_install_to_mt_endpoint_not_found() {
    let app = create_test_app().await;

    // Try to install to a non-existent MT installation
    let request = Request::builder()
        .method("POST")
        .uri("/api/mt-installations/non-existent-id/install")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // RFC 9457: Should return 404 Not Found with ProblemDetails
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Check RFC 9457 Problem Details structure
    assert!(json["type"].is_string());
    assert!(json["title"].is_string());
    assert_eq!(json["status"], 404);
    assert!(json["detail"].is_string());
    assert!(json["instance"].is_string());

    // Error message should indicate MT4/MT5 not found (English detail)
    let detail = json["detail"].as_str().unwrap();
    assert!(detail.contains("was not found"));
}

#[tokio::test]
async fn test_mt_installations_response_structure() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/mt-installations")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // No longer wrapped in ApiResponse
    let installations = json["data"].as_array().unwrap();

    // Check structure of each installation if any are found
    for installation in installations {
        assert!(installation["id"].is_string());
        assert!(installation["name"].is_string());
        assert!(installation["type"].is_string());
        assert!(installation["platform"].is_string());
        assert!(installation["path"].is_string());
        assert!(installation["executable"].is_string());
        assert!(installation["version"].is_null() || installation["version"].is_string());
        // Check components structure
        let components = &installation["components"];
        assert!(components["dll"].is_boolean());
        assert!(components["master_ea"].is_boolean());
        assert!(components["slave_ea"].is_boolean());

        // Check type values
        let mt_type = installation["type"].as_str().unwrap();
        assert!(mt_type == "MT4" || mt_type == "MT5");

        let platform = installation["platform"].as_str().unwrap();
        assert!(platform == "32-bit" || platform == "64-bit");
    }
}
