use anyhow::Result;
use e2e_tests::helpers::setup_test_db;
use e2e_tests::TestSandbox;
use futures_util::StreamExt;
use sankey_copier_relay_server::db::Database;
use sankey_copier_relay_server::models::{SlaveSettings, SystemStateSnapshot, WarningCode};
use std::time::Duration;
use tokio_tungstenite::connect_async;

#[tokio::test]
async fn test_system_snapshot_broadcast() -> Result<()> {
    // 1. Start Sandbox
    let sandbox = TestSandbox::new()?;
    let http_port = sandbox.server().http_port;
    let db_url = sandbox.server().db_url();

    // Connect to DB for setup
    let db = Database::new(&db_url).await?;

    // 2. Setup DB with Master (but no Slaves needed for this test)
    let master_account = "Master_1001";
    // Helper requires slave_settings_fn even if empty list
    setup_test_db(&db, master_account, &[], |_| SlaveSettings::default()).await?;
    println!(
        "DB Setup complete: TradeGroup created for {}",
        master_account
    );

    // 3. Connect WebSocket client
    let ws_url = format!("wss://127.0.0.1:{}/ws", http_port);
    println!("Connecting to WebSocket: {}", ws_url);

    // Configure TLS connector to trust self-signed certs (insecure for tests)
    let tls_connector = native_tls::TlsConnector::builder()
        .danger_accept_invalid_certs(true)
        .build()?;
    let connector = tokio_tungstenite::Connector::NativeTls(tls_connector);

    let mut ws_stream = None;
    for _ in 0..10 {
        match tokio_tungstenite::connect_async_tls_with_config(
            &ws_url,
            None,
            false,
            Some(connector.clone()),
        )
        .await
        {
            Ok((stream, _)) => {
                ws_stream = Some(stream);
                break;
            }
            Err(e) => {
                println!("WS connection failed, retrying: {}", e);
                tokio::time::sleep(Duration::from_millis(1000)).await;
            }
        }
    }
    let mut ws_stream = ws_stream.expect("Failed to connect to WebSocket");
    println!("WebSocket connected");

    // 4. Start Master EA Simulator
    let mut master = sandbox.create_master(master_account)?;
    master.set_trade_allowed(true); // AutoTrading ON
    master.start()?;
    println!("Master EA Simulator started");

    // 5. Wait for initial snapshot with Master present
    let mut found_master = false;
    // Note: Master takes a few seconds to register via Heartbeat loop
    let timeout = tokio::time::sleep(Duration::from_secs(15));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            msg = ws_stream.next() => {
                if let Some(Ok(msg)) = msg {
                    if msg.is_text() {
                        let text = msg.to_text()?;
                        if text.starts_with("system_snapshot:") {
                            let json_part = &text["system_snapshot:".len()..];
                            match serde_json::from_str::<SystemStateSnapshot>(json_part) {
                                Ok(snapshot) => {
                                    if let Some(tg) = snapshot.trade_groups.iter().find(|tg| tg.id == master_account) {
                                        println!("Snapshot received with Master: {:?}", tg);
                                        // Verify initial state
                                        if !tg.master_warning_codes.contains(&WarningCode::MasterAutoTradingDisabled) {
                                            found_master = true;
                                            break;
                                        }
                                    }
                                },
                                Err(e) => println!("Failed to parse system_snapshot: {}", e),
                            }
                        }
                    }
                }
            }
            _ = &mut timeout => {
                break;
            }
        }
    }

    if !found_master {
        return Err(anyhow::anyhow!(
            "Timed out waiting for system_snapshot with connected Master"
        ));
    }

    // 6. Toggle Master AutoTrading OFF
    println!("Toggling Master AutoTrading OFF...");
    master.set_trade_allowed(false);
    // Wait for next heartbeat cycle (Master simulator sends HB every 1s) and broadcast

    // 7. Wait for immediate update with Warning
    let mut update_received = false;
    let timeout_update = tokio::time::sleep(Duration::from_secs(10));
    tokio::pin!(timeout_update);

    loop {
        tokio::select! {
            msg = ws_stream.next() => {
                if let Some(Ok(msg)) = msg {
                    if msg.is_text() {
                        let text = msg.to_text()?;
                        if text.starts_with("system_snapshot:") {
                            let json_part = &text["system_snapshot:".len()..];
                            match serde_json::from_str::<SystemStateSnapshot>(json_part) {
                                Ok(snapshot) => {
                                    if let Some(tg) = snapshot.trade_groups.iter().find(|tg| tg.id == master_account) {
                                        // Check for specific warning
                                        if tg.master_warning_codes.contains(&WarningCode::MasterAutoTradingDisabled) {
                                            println!("Verified: Snapshot contains MasterAutoTradingDisabled warning");
                                            update_received = true;
                                            break;
                                        }
                                    }
                                },
                                Err(_) => {}
                            }
                        }
                    }
                }
            }
            _ = &mut timeout_update => {
                break;
            }
        }
    }

    if !update_received {
        return Err(anyhow::anyhow!(
            "Timed out waiting for AutoTrading disabled warning"
        ));
    }

    Ok(())
}
