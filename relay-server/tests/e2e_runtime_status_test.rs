// relay-server/tests/e2e_runtime_status_test.rs
//
// E2E regression tests for RuntimeStatusUpdater.
// Verifies that Slave runtime_status transitions Standby→Connected when
// only ZeroMQ Heartbeat traffic is involved (no manual DB tweaks after setup).

mod test_server;

use std::sync::Arc;

use anyhow::Result;
use sankey_copier_relay_server::db::Database;
use sankey_copier_relay_server::models::{
    MasterSettings, SlaveSettings, STATUS_CONNECTED, STATUS_DISABLED, STATUS_ENABLED,
};
use sankey_copier_zmq::ffi::{
    zmq_context_create, zmq_context_destroy, zmq_socket_connect, zmq_socket_create,
    zmq_socket_destroy, zmq_socket_send_binary, ZMQ_PUSH,
};
use sankey_copier_zmq::HeartbeatMessage;
use test_server::TestServer;
use tokio::time::{sleep, Duration};

const SETTLE_WAIT_MS: u64 = 250;

#[tokio::test]
async fn slave_runtime_status_tracks_master_cluster_events() {
    let server = TestServer::start()
        .await
        .expect("failed to start relay-server test instance");

    let master_account = "RUNTIME_MASTER_001";
    let slave_account = "RUNTIME_SLAVE_001";

    seed_trade_group(server.db.clone(), master_account, slave_account)
        .await
        .expect("failed to seed trade group");

    assert_runtime_status(&server.db, master_account, slave_account, STATUS_DISABLED).await;

    send_heartbeat(
        &server.zmq_pull_address(),
        slave_account,
        "Slave",
        /* is_trade_allowed */ true,
    )
    .expect("slave heartbeat should be sent");
    sleep(Duration::from_millis(SETTLE_WAIT_MS)).await;

    assert_runtime_status(&server.db, master_account, slave_account, STATUS_ENABLED).await;

    send_heartbeat(&server.zmq_pull_address(), master_account, "Master", true)
        .expect("master heartbeat should be sent");
    sleep(Duration::from_millis(SETTLE_WAIT_MS)).await;

    assert_runtime_status(&server.db, master_account, slave_account, STATUS_CONNECTED).await;

    server.shutdown().await;
}

async fn seed_trade_group(
    db: Arc<Database>,
    master_account: &str,
    slave_account: &str,
) -> Result<()> {
    db.create_trade_group(master_account).await?;
    db.update_master_settings(
        master_account,
        MasterSettings {
            enabled: true,
            config_version: 1,
            ..MasterSettings::default()
        },
    )
    .await?;

    db.add_member(
        master_account,
        slave_account,
        SlaveSettings::default(),
        STATUS_DISABLED,
    )
    .await?;

    db.update_member_enabled_flag(master_account, slave_account, true)
        .await?;
    db.update_member_runtime_status(master_account, slave_account, STATUS_DISABLED)
        .await?;
    Ok(())
}

async fn assert_runtime_status(
    db: &Arc<Database>,
    master_account: &str,
    slave_account: &str,
    expected: i32,
) {
    let member = db
        .get_member(master_account, slave_account)
        .await
        .expect("DB query should succeed")
        .expect("member should exist");
    assert_eq!(member.runtime_status, expected);
}

fn send_heartbeat(
    push_address: &str,
    account_id: &str,
    ea_type: &str,
    is_trade_allowed: bool,
) -> Result<()> {
    let context = zmq_context_create();
    if context < 0 {
        anyhow::bail!("failed to create ZMQ context");
    }

    let socket = zmq_socket_create(context, ZMQ_PUSH);
    if socket < 0 {
        zmq_context_destroy(context);
        anyhow::bail!("failed to create ZMQ PUSH socket");
    }

    let address: Vec<u16> = push_address.encode_utf16().chain(Some(0)).collect();
    let connect_result = unsafe { zmq_socket_connect(socket, address.as_ptr()) };
    if connect_result != 1 {
        zmq_socket_destroy(socket);
        zmq_context_destroy(context);
        anyhow::bail!("failed to connect ZMQ PUSH socket");
    }

    let heartbeat = HeartbeatMessage {
        message_type: "Heartbeat".to_string(),
        account_id: account_id.to_string(),
        balance: 50_000.0,
        equity: 50_000.0,
        open_positions: 0,
        timestamp: chrono::Utc::now().to_rfc3339(),
        version: "runtime-status-test".to_string(),
        ea_type: ea_type.to_string(),
        platform: "MT5".to_string(),
        account_number: 123456,
        broker: "TestBroker".to_string(),
        account_name: format!("{} Account", account_id),
        server: "TestServer".to_string(),
        currency: "USD".to_string(),
        leverage: 500,
        is_trade_allowed,
        symbol_prefix: None,
        symbol_suffix: None,
        symbol_map: None,
    };

    let payload = rmp_serde::to_vec_named(&heartbeat)?;
    let send_result =
        unsafe { zmq_socket_send_binary(socket, payload.as_ptr(), payload.len() as i32) };
    zmq_socket_destroy(socket);
    zmq_context_destroy(context);

    if send_result != 1 {
        anyhow::bail!("failed to send heartbeat frame");
    }

    Ok(())
}
