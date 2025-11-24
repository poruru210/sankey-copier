// relay-server/tests/trade_groups_api_test.rs
//
// Unit tests for TradeGroups REST API endpoints.
// Tests list, get, and update operations for Master EA settings.

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
use sankey_copier_relay_server::models::MasterSettings;
use sankey_copier_relay_server::zeromq::ZmqConfigPublisher;

use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

/// Helper function to create a test app with in-memory database
async fn create_test_app() -> (axum::Router, Arc<Database>) {
    let db = Arc::new(Database::new("sqlite::memory:").await.unwrap());
    let connection_manager = Arc::new(ConnectionManager::new(30));
    let settings_cache = Arc::new(RwLock::new(Vec::new()));
    let (broadcast_tx, _) = broadcast::channel::<String>(100);
    let log_buffer = create_log_buffer();

    // Create a dummy ZMQ config sender
    let config_sender = Arc::new(ZmqConfigPublisher::new("tcp://127.0.0.1:0").unwrap());

    let app_state = AppState {
        db: db.clone(),
        tx: broadcast_tx,
        settings_cache,
        connection_manager,
        config_sender,
        log_buffer,
        allowed_origins: vec!["http://localhost:8080".to_string()],
        cors_disabled: false,
        config: Arc::new(sankey_copier_relay_server::config::Config::default()),
    };

    (create_router(app_state), db)
}

#[tokio::test]
async fn test_list_trade_groups_empty() {
    let (app, _db) = create_test_app().await;

    let request = Request::builder()
        .uri("/api/trade-groups")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Should return empty array
    assert!(json.is_array());
    assert_eq!(json.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_list_trade_groups_with_data() {
    let (app, db) = create_test_app().await;

    // Create test trade groups
    db.create_trade_group("MASTER_001").await.unwrap();
    db.create_trade_group("MASTER_002").await.unwrap();

    let request = Request::builder()
        .uri("/api/trade-groups")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Should return 2 trade groups
    assert!(json.is_array());
    let trade_groups = json.as_array().unwrap();
    assert_eq!(trade_groups.len(), 2);

    // Check structure
    for tg in trade_groups {
        assert!(tg["id"].is_string());
        assert!(tg["master_settings"].is_object());
        assert!(tg["created_at"].is_string());
        assert!(tg["updated_at"].is_string());
    }
}

#[tokio::test]
async fn test_get_trade_group_success() {
    let (app, db) = create_test_app().await;

    // Create a test trade group
    db.create_trade_group("MASTER_123").await.unwrap();

    let request = Request::builder()
        .uri("/api/trade-groups/MASTER_123")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Check response structure
    assert_eq!(json["id"], "MASTER_123");
    assert!(json["master_settings"].is_object());
    assert!(json["created_at"].is_string());
    assert!(json["updated_at"].is_string());

    // Check default settings
    let settings = &json["master_settings"];
    assert_eq!(settings["config_version"], 0);
}

#[tokio::test]
async fn test_get_trade_group_not_found() {
    let (app, _db) = create_test_app().await;

    let request = Request::builder()
        .uri("/api/trade-groups/NONEXISTENT")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return 404 Not Found with ProblemDetails
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

    let detail = json["detail"].as_str().unwrap();
    assert!(detail.contains("was not found"));
}

#[tokio::test]
async fn test_update_trade_group_settings_success() {
    let (app, db) = create_test_app().await;

    // Create a test trade group
    db.create_trade_group("MASTER_456").await.unwrap();

    // Update settings
    let updated_settings = MasterSettings {
        symbol_prefix: Some("pro.".to_string()),
        symbol_suffix: Some(".m".to_string()),
        config_version: 0, // Will be incremented by the API
    };

    let request = Request::builder()
        .method("PUT")
        .uri("/api/trade-groups/MASTER_456")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&updated_settings).unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return 204 No Content
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // Verify the settings were updated in the database
    let trade_group = db.get_trade_group("MASTER_456").await.unwrap().unwrap();
    assert_eq!(
        trade_group.master_settings.symbol_prefix,
        Some("pro.".to_string())
    );
    assert_eq!(
        trade_group.master_settings.symbol_suffix,
        Some(".m".to_string())
    );
    // Config version should have been incremented
    assert_eq!(trade_group.master_settings.config_version, 1);
}

#[tokio::test]
async fn test_update_trade_group_settings_increments_version() {
    let (app, db) = create_test_app().await;

    // Create a test trade group
    db.create_trade_group("MASTER_789").await.unwrap();

    // First update
    let settings_v1 = MasterSettings {
        symbol_prefix: Some("v1.".to_string()),
        symbol_suffix: None,
        config_version: 0,
    };

    let request1 = Request::builder()
        .method("PUT")
        .uri("/api/trade-groups/MASTER_789")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&settings_v1).unwrap()))
        .unwrap();

    app.clone().oneshot(request1).await.unwrap();

    // Verify first version
    let tg1 = db.get_trade_group("MASTER_789").await.unwrap().unwrap();
    assert_eq!(tg1.master_settings.config_version, 1);

    // Second update
    let settings_v2 = MasterSettings {
        symbol_prefix: Some("v2.".to_string()),
        symbol_suffix: Some(".v2".to_string()),
        config_version: 1, // API will increment this
    };

    let request2 = Request::builder()
        .method("PUT")
        .uri("/api/trade-groups/MASTER_789")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&settings_v2).unwrap()))
        .unwrap();

    app.oneshot(request2).await.unwrap();

    // Verify second version
    let tg2 = db.get_trade_group("MASTER_789").await.unwrap().unwrap();
    assert_eq!(tg2.master_settings.config_version, 2);
    assert_eq!(tg2.master_settings.symbol_prefix, Some("v2.".to_string()));
    assert_eq!(tg2.master_settings.symbol_suffix, Some(".v2".to_string()));
}

#[tokio::test]
async fn test_trade_group_response_structure() {
    let (app, db) = create_test_app().await;

    // Create test data
    db.create_trade_group("MASTER_STRUCT_TEST").await.unwrap();

    let request = Request::builder()
        .uri("/api/trade-groups")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    let trade_groups = json.as_array().unwrap();
    assert_eq!(trade_groups.len(), 1);

    let tg = &trade_groups[0];

    // Verify all required fields exist
    assert_eq!(tg["id"], "MASTER_STRUCT_TEST");

    // Verify master_settings structure
    let settings = &tg["master_settings"];
    assert!(settings.is_object());
    assert!(settings["config_version"].is_number());
    assert_eq!(settings["config_version"], 0);

    // Optional fields should not be present if None
    // (because of #[serde(skip_serializing_if = "Option::is_none")])
    let settings_str = serde_json::to_string(settings).unwrap();
    assert!(!settings_str.contains("symbol_prefix"));
    assert!(!settings_str.contains("symbol_suffix"));

    // Timestamps should be present
    assert!(tg["created_at"].is_string());
    assert!(tg["updated_at"].is_string());
}
