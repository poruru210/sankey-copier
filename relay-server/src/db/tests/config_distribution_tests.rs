//! Tests for config distribution operations
//!
//! Tests for Master/Slave status updates during connection lifecycle

use super::*;
use crate::models::SlaveSettings;

#[tokio::test]
async fn test_update_master_statuses_connected() {
    let db = create_test_db().await;

    // Create TradeGroup and three members for the same master: one disabled, two enabled
    db.create_trade_group("MASTER_001").await.unwrap();

    let slave_settings = create_test_slave_settings();

    // Member 1: DISABLED
    db.add_member("MASTER_001", "SLAVE_001", slave_settings.clone()).await.unwrap();
    db.update_member_status("MASTER_001", "SLAVE_001", 0).await.unwrap(); // DISABLED

    // Member 2: ENABLED
    db.add_member("MASTER_001", "SLAVE_002", slave_settings.clone()).await.unwrap();
    db.update_member_status("MASTER_001", "SLAVE_002", 1).await.unwrap(); // ENABLED

    // Member 3: ENABLED
    db.add_member("MASTER_001", "SLAVE_003", slave_settings).await.unwrap();
    db.update_member_status("MASTER_001", "SLAVE_003", 1).await.unwrap(); // ENABLED

    // Update master statuses to CONNECTED
    let count = db
        .update_master_statuses_connected("MASTER_001")
        .await
        .unwrap();

    // Should update 2 settings (the enabled ones)
    assert_eq!(count, 2);

    // Verify statuses
    let member1 = db.get_member("MASTER_001", "SLAVE_001").await.unwrap().unwrap();
    assert_eq!(member1.status, 0); // Still DISABLED

    let member2 = db.get_member("MASTER_001", "SLAVE_002").await.unwrap().unwrap();
    assert_eq!(member2.status, 2); // Now CONNECTED

    let member3 = db.get_member("MASTER_001", "SLAVE_003").await.unwrap().unwrap();
    assert_eq!(member3.status, 2); // Now CONNECTED
}

#[tokio::test]
async fn test_update_master_statuses_disconnected() {
    let db = create_test_db().await;

    // Create TradeGroup and three members with different statuses
    db.create_trade_group("MASTER_001").await.unwrap();

    let slave_settings = create_test_slave_settings();

    // Member 1: DISABLED
    db.add_member("MASTER_001", "SLAVE_001", slave_settings.clone()).await.unwrap();
    db.update_member_status("MASTER_001", "SLAVE_001", 0).await.unwrap(); // DISABLED

    // Member 2: ENABLED
    db.add_member("MASTER_001", "SLAVE_002", slave_settings.clone()).await.unwrap();
    db.update_member_status("MASTER_001", "SLAVE_002", 1).await.unwrap(); // ENABLED

    // Member 3: CONNECTED
    db.add_member("MASTER_001", "SLAVE_003", slave_settings).await.unwrap();
    db.update_member_status("MASTER_001", "SLAVE_003", 2).await.unwrap(); // CONNECTED

    // Update master statuses to ENABLED (disconnected)
    let count = db
        .update_master_statuses_disconnected("MASTER_001")
        .await
        .unwrap();

    // Should update 1 setting (the connected one)
    assert_eq!(count, 1);

    // Verify statuses
    let member1 = db.get_member("MASTER_001", "SLAVE_001").await.unwrap().unwrap();
    assert_eq!(member1.status, 0); // Still DISABLED

    let member2 = db.get_member("MASTER_001", "SLAVE_002").await.unwrap().unwrap();
    assert_eq!(member2.status, 1); // Still ENABLED

    let member3 = db.get_member("MASTER_001", "SLAVE_003").await.unwrap().unwrap();
    assert_eq!(member3.status, 1); // Now ENABLED (was CONNECTED)
}

#[tokio::test]
async fn test_update_master_statuses_no_settings() {
    let db = create_test_db().await;

    // Try to update statuses for a master with no settings
    let count = db
        .update_master_statuses_connected("NONEXISTENT_MASTER")
        .await
        .unwrap();

    assert_eq!(count, 0);

    let count = db
        .update_master_statuses_disconnected("NONEXISTENT_MASTER")
        .await
        .unwrap();

    assert_eq!(count, 0);
}
