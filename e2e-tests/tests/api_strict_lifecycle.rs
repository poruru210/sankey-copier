// e2e-tests/tests/api_strict_lifecycle.rs

use e2e_tests::TestSandbox;
use reqwest::StatusCode;
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_strict_tradegroup_lifecycle() {
    let sandbox = TestSandbox::new().expect("Failed to start sandbox");
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .expect("Failed to create HTTP client");
    // Use sandbox.server().http_base_url() to get the API base URL
    let api_base = sandbox.server().http_base_url();
    let master_account = "MASTER_STRICT_LIFECYCLE";

    // 1. Start Master EA
    // In strict mode, simply connecting should NOT create the TradeGroup.
    // However, currently (Legacy), it DOES. So this test expects Strict behavior and will FAIL.
    let mut master_ea = sandbox
        .create_master(master_account, true)
        .expect("Failed to create Master EA");
    master_ea.set_trade_allowed(true);
    master_ea.start().expect("Failed to start Master EA");

    // Allow some time for heartbeat processing
    sleep(Duration::from_millis(1000)).await;

    // 2. Verify TradeGroup does NOT exist (404)
    // Legacy behavior: Returns 200 (Auto-created).
    // Target behavior: Returns 404.
    let resp = client
        .get(format!("{}/api/trade-groups/{}", api_base, master_account))
        .send()
        .await
        .expect("Failed to execute GET request");

    // TDD Assertion: This will fail until Auto-Creation is removed.
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "TradeGroup should NOT be auto-created by heartbeat. Strict Lifecycle violation."
    );

    // 3. Verify POST /members fails with 404 (Parent missing)
    let resp_add_member = client
        .post(format!(
            "{}/api/trade-groups/{}/members",
            api_base, master_account
        ))
        .json(&json!({
            "slave_account": "SLAVE_ORPHAN",
            "slave_settings": {},
            "enabled": true
        }))
        .send()
        .await
        .expect("Failed to execute POST /members");

    assert_eq!(
        resp_add_member.status(),
        StatusCode::NOT_FOUND,
        "Adding member to non-existent TradeGroup should fail with 404"
    );

    // 4. Create TradeGroup Explicitly via POST /api/trade-groups
    // Note: This endpoint is NOT IMPLEMENTED yet, so it might return 404 or 405.
    let resp_create = client
        .post(format!("{}/api/trade-groups", api_base))
        .json(&json!({
            "id": master_account,
            "master_settings": {
                "enabled": true,
                "symbol_prefix": "STRICT_",
                "config_version": 1
            },
            "members": []
        }))
        .send()
        .await
        .expect("Failed to execute Create TradeGroup");

    assert_eq!(
        resp_create.status(),
        StatusCode::OK,
        "Explicit POST /api/trade-groups should succeed"
    );

    // 5. Verify TradeGroup now exists
    let resp_get_after = client
        .get(format!("{}/api/trade-groups/{}", api_base, master_account))
        .send()
        .await
        .expect("Failed to get TG after creation");

    assert_eq!(
        resp_get_after.status(),
        StatusCode::OK,
        "TradeGroup should exist after explicit creation"
    );

    // 6. Verify Master Received Config (Triggered by POST)
    // The simulator should receive a config with the "STRICT_" prefix we set.
    // Use try_receive_master_config instead of wait_for_config
    let config = master_ea
        .try_receive_master_config(5000)
        .expect("Master should receive config");
    assert!(
        config.is_some(),
        "Master did not receive config after Creation"
    );

    println!("✅ Strict Lifecycle Test Passed");
}
