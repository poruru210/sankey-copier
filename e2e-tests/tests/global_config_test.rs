use anyhow::Result;
use e2e_tests::TestSandbox;
use serial_test::serial;
use std::time::Duration;

#[test]
#[serial]
fn test_global_config_broadcast() -> Result<()> {
    let sandbox = TestSandbox::new()?;
    let relay = sandbox.server();

    // 1. Create Master and Slave simulators using factory methods
    let mut master = sandbox.create_master("MASTER_GLOBAL", true)?;
    let mut slave = sandbox.create_slave("SLAVE_GLOBAL", "MASTER_GLOBAL", true)?;

    master.start()?;
    slave.start()?;

    // Wait for connection/initialization
    std::thread::sleep(Duration::from_secs(2));

    // 2. Subscribe to global config using the new API
    master.subscribe_to_global_config()?;
    slave.subscribe_to_global_config()?;

    // Wait for subscription to be processed by OnTimer thread
    std::thread::sleep(Duration::from_secs(1));

    // 3. Trigger Global Config Broadcast via REST API
    // Use https://127.0.0.1:port
    let client = reqwest::blocking::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()?;

    let url = format!("{}/api/victoria-logs-settings", relay.http_base_url());

    let response = client
        .put(&url)
        .json(&serde_json::json!({
            "enabled": true
        }))
        .send()?;

    assert!(response.status().is_success());

    // 4. Verify Master received the config
    // We wait for up to 5 seconds
    let config = master.try_receive_vlogs_config(5000)?;
    assert!(config.is_some(), "Master should receive global config");
    let config = config.unwrap();
    assert!(config.enabled);

    // 5. Verify Slave received the config
    let config = slave.try_receive_vlogs_config(5000)?;
    assert!(config.is_some(), "Slave should receive global config");
    let config = config.unwrap();
    assert!(config.enabled);

    Ok(())
}
