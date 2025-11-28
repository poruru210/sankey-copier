// relay-server/tests/reproduce_switch_issue.rs
//
// Reproduction test for Web UI switch persistence issue

mod test_server;

use sankey_copier_relay_server::models::SlaveSettings;
use test_server::TestServer;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_switch_persistence() {
    // Start test server
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let master_account = "MASTER_SWITCH_TEST";
    let slave_account = "SLAVE_SWITCH_TEST";

    // 1. Create TradeGroup and Member (initially DISABLED)
    server
        .db
        .create_trade_group(master_account)
        .await
        .expect("Failed to create trade group");

    server
        .db
        .add_member(master_account, slave_account, SlaveSettings::default(), 0)
        .await
        .expect("Failed to add member");

    // Verify initial status
    let member = server
        .db
        .get_member(master_account, slave_account)
        .await
        .expect("Failed to get member")
        .expect("Member not found");
    assert_eq!(member.status, 0, "Initial status should be 0");

    // 2. Toggle ON
    server
        .db
        .update_member_status(master_account, slave_account, 1)
        .await
        .expect("Failed to toggle ON");

    // Verify status persisted as 1
    let member = server
        .db
        .get_member(master_account, slave_account)
        .await
        .expect("Failed to get member")
        .expect("Member not found");
    assert_eq!(member.status, 1, "Status after toggle ON should be 1");

    // 3. Toggle OFF
    server
        .db
        .update_member_status(master_account, slave_account, 0)
        .await
        .expect("Failed to toggle OFF");

    // Verify status persisted as 0
    let member = server
        .db
        .get_member(master_account, slave_account)
        .await
        .expect("Failed to get member")
        .expect("Member not found");
    assert_eq!(member.status, 0, "Status after toggle OFF should be 0");

    println!("✅ Switch persistence test passed");

    server.shutdown().await;
}
