use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use tower::util::ServiceExt;

use sankey_copier_server::api::create_router;
use sankey_copier_server::api::AppState;
use sankey_copier_server::connection_manager::ConnectionManager;
use sankey_copier_server::db::Database;
use sankey_copier_server::log_buffer::create_log_buffer;
use sankey_copier_server::zeromq::ZmqConfigPublisher;

use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

/// Helper function to create a test app
async fn create_test_app() -> axum::Router {
    let db = Arc::new(Database::new("sqlite::memory:").await.unwrap());
    let connection_manager = Arc::new(ConnectionManager::new(30));
    let settings_cache = Arc::new(RwLock::new(Vec::new()));
    let (broadcast_tx, _) = broadcast::channel::<String>(100);
    let log_buffer = create_log_buffer();

    // Create a dummy ZMQ config sender
    let config_sender = Arc::new(ZmqConfigPublisher::new("tcp://127.0.0.1:0").unwrap());

    let app_state = AppState {
        db,
        tx: broadcast_tx,
        settings_cache,
        connection_manager,
        config_sender,
        log_buffer,
        allowed_origins: vec!["http://localhost:8080".to_string()],
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

    // Check response structure
    assert!(json["success"].is_boolean());
    assert!(json["data"].is_object());

    let data = &json["data"];
    assert!(data["success"].is_boolean());
    assert!(data["data"].is_array());
    assert!(data["detection_summary"].is_object());

    let summary = &data["detection_summary"];
    assert!(summary["total_found"].is_number());
    assert!(summary["by_method"].is_object());
    assert!(summary["running"].is_number());
    assert!(summary["stopped"].is_number());
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

    // Should return OK with error in JSON (not HTTP error)
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Check that success is false
    assert_eq!(json["success"], false);
    assert!(json["error"].is_string());

    // Error message should indicate MT4/MT5 not found
    let error = json["error"].as_str().unwrap();
    assert!(error.contains("見つかりません") || error.contains("not found"));
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

    let data = &json["data"];
    let installations = data["data"].as_array().unwrap();

    // Check structure of each installation if any are found
    for installation in installations {
        assert!(installation["id"].is_string());
        assert!(installation["name"].is_string());
        assert!(installation["type"].is_string());
        assert!(installation["platform"].is_string());
        assert!(installation["path"].is_string());
        assert!(installation["executable"].is_string());
        assert!(installation["is_running"].is_boolean());
        assert!(installation["detection_method"].is_string());
        assert!(installation["is_installed"].is_boolean());
        assert!(installation["available_version"].is_string());

        // Check components structure
        let components = &installation["components"];
        assert!(components["dll"].is_boolean());
        assert!(components["master_ea"].is_boolean());
        assert!(components["slave_ea"].is_boolean());
        assert!(components["includes"].is_boolean());

        // Check type values
        let mt_type = installation["type"].as_str().unwrap();
        assert!(mt_type == "MT4" || mt_type == "MT5");

        let platform = installation["platform"].as_str().unwrap();
        assert!(platform == "32-bit" || platform == "64-bit");

        let detection_method = installation["detection_method"].as_str().unwrap();
        assert_eq!(detection_method, "process");
    }
}
