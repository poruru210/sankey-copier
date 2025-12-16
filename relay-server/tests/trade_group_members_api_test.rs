// relay-server/tests/trade_group_members_api_test.rs

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

use sankey_copier_relay_server::adapters::inbound::http::{
    create_router,
    dtos::{AddMemberRequest, ToggleStatusRequest},
    AppState, SnapshotBroadcaster,
};
use sankey_copier_relay_server::adapters::infrastructure::connection_manager::ConnectionManager;
use sankey_copier_relay_server::adapters::infrastructure::log_buffer::create_log_buffer;
use sankey_copier_relay_server::adapters::infrastructure::port_resolver::ResolvedPorts;
use sankey_copier_relay_server::adapters::outbound::messaging::ZmqConfigPublisher;
use sankey_copier_relay_server::adapters::outbound::persistence::Database;
use sankey_copier_relay_server::application::runtime_status_updater::RuntimeStatusMetrics;
use sankey_copier_relay_server::domain::models::{
    LotCalculationMode, MasterSettings, SlaveSettings, SymbolMapping, TradeFilters,
};

use std::sync::Arc;
use tokio::sync::broadcast;

/// Helper function to create a test app with in-memory database
async fn create_test_app() -> (axum::Router, Arc<Database>) {
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

    let app_state = AppState {
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
    };

    (create_router(app_state), db)
}

/// Helper function to create a test TradeGroup (Master)
async fn setup_test_trade_group(db: &Arc<Database>, master_account: &str) {
    // First create the TradeGroup
    db.create_trade_group(master_account).await.unwrap();

    // Then update its settings
    let settings = MasterSettings {
        enabled: true,
        symbol_prefix: Some("pro.".to_string()),
        symbol_suffix: Some(".m".to_string()),
        config_version: 1,
    };

    db.update_master_settings(master_account, settings)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_list_members_empty() {
    let (app, db) = create_test_app().await;
    setup_test_trade_group(&db, "MASTER_001").await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/trade-groups/MASTER_001/members")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let members: Vec<sankey_copier_relay_server::domain::models::TradeGroupMember> =
        serde_json::from_slice(&body).unwrap();

    assert_eq!(members.len(), 0);
}

#[tokio::test]
async fn test_add_member_success() {
    let (app, db) = create_test_app().await;
    setup_test_trade_group(&db, "MASTER_001").await;

    let request_body = AddMemberRequest {
        slave_account: "SLAVE_001".to_string(),
        slave_settings: SlaveSettings {
            lot_calculation_mode: LotCalculationMode::default(),
            lot_multiplier: Some(1.5),
            reverse_trade: false,
            symbol_prefix: None,
            symbol_suffix: None,
            symbol_mappings: vec![SymbolMapping {
                source_symbol: "EURUSD".to_string(),
                target_symbol: "EURUSDm".to_string(),
            }],
            filters: TradeFilters {
                allowed_symbols: Some(vec!["EURUSD".to_string()]),
                blocked_symbols: None,
                allowed_magic_numbers: None,
                blocked_magic_numbers: None,
            },
            config_version: 0,
            source_lot_min: None,
            source_lot_max: None,
            sync_mode: sankey_copier_relay_server::domain::models::SyncMode::Skip,
            limit_order_expiry_min: None,
            market_sync_max_pips: None,
            max_slippage: None,
            copy_pending_orders: false,
            max_retries: 3,
            max_signal_delay_ms: 5000,
            use_pending_order_for_delayed: false,
        },
        status: 0,
    };

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/trade-groups/MASTER_001/members")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let member: sankey_copier_relay_server::domain::models::TradeGroupMember =
        serde_json::from_slice(&body).unwrap();

    assert_eq!(member.slave_account, "SLAVE_001");
    assert_eq!(member.trade_group_id, "MASTER_001");
    assert_eq!(member.status, 0); // DISABLED - initial status is OFF
    assert_eq!(member.slave_settings.lot_multiplier, Some(1.5));
    assert!(!member.slave_settings.reverse_trade);
}

#[tokio::test]
async fn test_add_member_auto_creates_trade_group() {
    let (app, db) = create_test_app().await;

    let request_body = AddMemberRequest {
        slave_account: "SLAVE_001".to_string(),
        slave_settings: SlaveSettings::default(),
        status: 0,
    };

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/trade-groups/NONEXISTENT/members")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should auto-create TradeGroup and return CREATED
    assert_eq!(response.status(), StatusCode::CREATED);

    // Verify TradeGroup was created
    let trade_group = db.get_trade_group("NONEXISTENT").await.unwrap();
    assert!(trade_group.is_some());

    // Verify member was added
    let member = db.get_member("NONEXISTENT", "SLAVE_001").await.unwrap();
    assert!(member.is_some());
}

#[tokio::test]
async fn test_get_member_success() {
    let (app, db) = create_test_app().await;
    setup_test_trade_group(&db, "MASTER_001").await;

    // Add a member first
    let settings = SlaveSettings {
        lot_multiplier: Some(2.0),
        reverse_trade: true,
        ..Default::default()
    };
    db.add_member("MASTER_001", "SLAVE_001", settings, 0)
        .await
        .unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/trade-groups/MASTER_001/members/SLAVE_001")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let member: sankey_copier_relay_server::domain::models::TradeGroupMember =
        serde_json::from_slice(&body).unwrap();

    assert_eq!(member.slave_account, "SLAVE_001");
    assert_eq!(member.slave_settings.lot_multiplier, Some(2.0));
    assert!(member.slave_settings.reverse_trade);
}

#[tokio::test]
async fn test_get_member_not_found() {
    let (app, db) = create_test_app().await;
    setup_test_trade_group(&db, "MASTER_001").await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/trade-groups/MASTER_001/members/NONEXISTENT")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_update_member_success() {
    let (app, db) = create_test_app().await;
    setup_test_trade_group(&db, "MASTER_001").await;

    // Add a member first
    let settings = SlaveSettings {
        lot_multiplier: Some(1.0),
        reverse_trade: false,
        config_version: 0,
        ..Default::default()
    };
    db.add_member("MASTER_001", "SLAVE_001", settings, 0)
        .await
        .unwrap();

    // Update the member
    let updated_settings = SlaveSettings {
        lot_multiplier: Some(3.0),
        reverse_trade: true,
        config_version: 0, // Will be incremented by handler
        ..Default::default()
    };

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/trade-groups/MASTER_001/members/SLAVE_001")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&updated_settings).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // Verify the update
    let member = db
        .get_member("MASTER_001", "SLAVE_001")
        .await
        .unwrap()
        .unwrap();

    assert_eq!(member.slave_settings.lot_multiplier, Some(3.0));
    assert!(member.slave_settings.reverse_trade);
    assert_eq!(member.slave_settings.config_version, 1); // Incremented
}

#[tokio::test]
async fn test_toggle_member_status_disable() {
    let (app, db) = create_test_app().await;
    setup_test_trade_group(&db, "MASTER_001").await;

    // Add a member (initial status = DISABLED)
    db.add_member("MASTER_001", "SLAVE_001", SlaveSettings::default(), 0)
        .await
        .unwrap();

    // First enable the member
    db.update_member_enabled_flag("MASTER_001", "SLAVE_001", true)
        .await
        .unwrap();
    db.update_member_runtime_status("MASTER_001", "SLAVE_001", 1)
        .await
        .unwrap();

    // Toggle to DISABLED
    let toggle_request = ToggleStatusRequest { enabled: false };

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/trade-groups/MASTER_001/members/SLAVE_001/toggle")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&toggle_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // Verify status changed
    let member = db
        .get_member("MASTER_001", "SLAVE_001")
        .await
        .unwrap()
        .unwrap();

    assert!(!member.enabled_flag);
    assert_eq!(member.status, 0); // DISABLED
}

#[tokio::test]
async fn test_toggle_member_status_enable() {
    let (app, db) = create_test_app().await;
    setup_test_trade_group(&db, "MASTER_001").await;

    // Add a member (initial status = DISABLED)
    db.add_member("MASTER_001", "SLAVE_001", SlaveSettings::default(), 0)
        .await
        .unwrap();

    // Verify initial status is DISABLED
    let member = db
        .get_member("MASTER_001", "SLAVE_001")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(member.status, 0); // Initial status is DISABLED

    // Toggle to ENABLED
    let toggle_request = ToggleStatusRequest { enabled: true };

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/trade-groups/MASTER_001/members/SLAVE_001/toggle")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&toggle_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // Verify status changed
    let member = db
        .get_member("MASTER_001", "SLAVE_001")
        .await
        .unwrap()
        .unwrap();

    assert!(member.enabled_flag);
    assert_eq!(member.status, 0); // runtime stays DISABLED until heartbeat evaluates
}

#[tokio::test]
async fn test_delete_member_success() {
    let (app, db) = create_test_app().await;
    setup_test_trade_group(&db, "MASTER_001").await;

    // Add a member
    db.add_member("MASTER_001", "SLAVE_001", SlaveSettings::default(), 0)
        .await
        .unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/trade-groups/MASTER_001/members/SLAVE_001")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // Verify member was deleted
    let member = db.get_member("MASTER_001", "SLAVE_001").await.unwrap();

    assert!(member.is_none());
}

#[tokio::test]
async fn test_list_members_with_multiple_members() {
    let (app, db) = create_test_app().await;
    setup_test_trade_group(&db, "MASTER_001").await;

    // Add multiple members
    for i in 1..=3 {
        let settings = SlaveSettings {
            lot_multiplier: Some(i as f64),
            ..Default::default()
        };
        db.add_member("MASTER_001", &format!("SLAVE_00{}", i), settings, 0)
            .await
            .unwrap();
    }

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/trade-groups/MASTER_001/members")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let members: Vec<sankey_copier_relay_server::domain::models::TradeGroupMember> =
        serde_json::from_slice(&body).unwrap();

    assert_eq!(members.len(), 3);
    assert_eq!(members[0].slave_account, "SLAVE_001");
    assert_eq!(members[1].slave_account, "SLAVE_002");
    assert_eq!(members[2].slave_account, "SLAVE_003");
}

#[tokio::test]
async fn test_member_with_complex_settings() {
    let (app, db) = create_test_app().await;
    setup_test_trade_group(&db, "MASTER_001").await;

    let request_body = AddMemberRequest {
        slave_account: "SLAVE_COMPLEX".to_string(),
        slave_settings: SlaveSettings {
            lot_calculation_mode: LotCalculationMode::default(),
            lot_multiplier: Some(2.5),
            reverse_trade: true,
            symbol_prefix: Some("FX_".to_string()),
            symbol_suffix: Some(".m".to_string()),
            symbol_mappings: vec![
                SymbolMapping {
                    source_symbol: "EURUSD".to_string(),
                    target_symbol: "EUR.USD".to_string(),
                },
                SymbolMapping {
                    source_symbol: "GBPUSD".to_string(),
                    target_symbol: "GBP.USD".to_string(),
                },
            ],
            filters: TradeFilters {
                allowed_symbols: Some(vec!["EURUSD".to_string(), "GBPUSD".to_string()]),
                blocked_symbols: Some(vec!["USDJPY".to_string()]),
                allowed_magic_numbers: Some(vec![12345, 67890]),
                blocked_magic_numbers: Some(vec![99999]),
            },
            config_version: 0,
            source_lot_min: None,
            source_lot_max: None,
            sync_mode: sankey_copier_relay_server::domain::models::SyncMode::Skip,
            limit_order_expiry_min: None,
            market_sync_max_pips: None,
            max_slippage: None,
            copy_pending_orders: false,
            max_retries: 3,
            max_signal_delay_ms: 5000,
            use_pending_order_for_delayed: false,
        },
        status: 0,
    };

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/trade-groups/MASTER_001/members")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let member: sankey_copier_relay_server::domain::models::TradeGroupMember =
        serde_json::from_slice(&body).unwrap();

    assert_eq!(member.slave_account, "SLAVE_COMPLEX");
    assert_eq!(member.slave_settings.lot_multiplier, Some(2.5));
    assert!(member.slave_settings.reverse_trade);
    assert_eq!(member.slave_settings.symbol_prefix, Some("FX_".to_string()));
    assert_eq!(member.slave_settings.symbol_mappings.len(), 2);
    assert_eq!(
        member
            .slave_settings
            .filters
            .allowed_symbols
            .as_ref()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        member
            .slave_settings
            .filters
            .blocked_symbols
            .as_ref()
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn test_add_member_duplicate_conflict() {
    let (app, db) = create_test_app().await;
    setup_test_trade_group(&db, "MASTER_001").await;

    let slave_settings = SlaveSettings {
        lot_multiplier: Some(1.0),
        ..Default::default()
    };

    // Add the first member
    db.add_member("MASTER_001", "SLAVE_DUP", slave_settings.clone(), 0)
        .await
        .unwrap();

    // Try to add the same member again
    let request_body = AddMemberRequest {
        slave_account: "SLAVE_DUP".to_string(),
        slave_settings,
        status: 0,
    };

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/trade-groups/MASTER_001/members")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return 409 Conflict or similar error
    assert!(response.status().is_client_error());
}

#[tokio::test]
async fn test_update_member_not_found() {
    let (app, db) = create_test_app().await;
    setup_test_trade_group(&db, "MASTER_001").await;

    let update_body = serde_json::json!({
        "slave_settings": {
            "lot_multiplier": 2.0,
            "reverse_trade": false,
            "symbol_mappings": [],
            "filters": {
                "allowed_symbols": null,
                "blocked_symbols": null,
                "allowed_magic_numbers": null,
                "blocked_magic_numbers": null
            },
            "config_version": 0
        },
        "status": 1
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/trade-groups/MASTER_001/members/NONEXISTENT_SLAVE")
                .header("content-type", "application/json")
                .body(Body::from(update_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return 404 Not Found
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_member_not_found() {
    let (app, db) = create_test_app().await;
    setup_test_trade_group(&db, "MASTER_001").await;

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/trade-groups/MASTER_001/members/NONEXISTENT_SLAVE")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should handle gracefully (either 404 or 204)
    assert!(
        response.status() == StatusCode::NOT_FOUND || response.status() == StatusCode::NO_CONTENT
    );
}

#[tokio::test]
async fn test_toggle_member_status_not_found() {
    let (app, db) = create_test_app().await;
    setup_test_trade_group(&db, "MASTER_001").await;

    let toggle_body = ToggleStatusRequest { enabled: true };

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/trade-groups/MASTER_001/members/NONEXISTENT_SLAVE/toggle")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&toggle_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return 404 Not Found
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
