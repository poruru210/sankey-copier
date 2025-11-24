//! Tests for config distribution operations
//!
//! Tests for Master/Slave status updates during connection lifecycle

use super::*;

#[tokio::test]
async fn test_update_master_statuses_connected() {
    let db = create_test_db().await;

    // Create three settings for the same master: one disabled, two enabled
    let mut settings1 = create_test_settings();
    settings1.master_account = "MASTER_001".to_string();
    settings1.slave_account = "SLAVE_001".to_string();
    settings1.status = 0; // DISABLED
    let id1 = db.save_copy_settings(&settings1).await.unwrap();

    let mut settings2 = create_test_settings();
    settings2.master_account = "MASTER_001".to_string();
    settings2.slave_account = "SLAVE_002".to_string();
    settings2.status = 1; // ENABLED
    let id2 = db.save_copy_settings(&settings2).await.unwrap();

    let mut settings3 = create_test_settings();
    settings3.master_account = "MASTER_001".to_string();
    settings3.slave_account = "SLAVE_003".to_string();
    settings3.status = 1; // ENABLED
    let id3 = db.save_copy_settings(&settings3).await.unwrap();

    // Update master statuses to CONNECTED
    let count = db
        .update_master_statuses_connected("MASTER_001")
        .await
        .unwrap();

    // Should update 2 settings (the enabled ones)
    assert_eq!(count, 2);

    // Verify statuses
    let retrieved1 = db.get_copy_settings(id1).await.unwrap().unwrap();
    assert_eq!(retrieved1.status, 0); // Still DISABLED

    let retrieved2 = db.get_copy_settings(id2).await.unwrap().unwrap();
    assert_eq!(retrieved2.status, 2); // Now CONNECTED

    let retrieved3 = db.get_copy_settings(id3).await.unwrap().unwrap();
    assert_eq!(retrieved3.status, 2); // Now CONNECTED
}

#[tokio::test]
async fn test_update_master_statuses_disconnected() {
    let db = create_test_db().await;

    // Create three settings for the same master with different statuses
    let mut settings1 = create_test_settings();
    settings1.master_account = "MASTER_001".to_string();
    settings1.slave_account = "SLAVE_001".to_string();
    settings1.status = 0; // DISABLED
    let id1 = db.save_copy_settings(&settings1).await.unwrap();

    let mut settings2 = create_test_settings();
    settings2.master_account = "MASTER_001".to_string();
    settings2.slave_account = "SLAVE_002".to_string();
    settings2.status = 1; // ENABLED
    let id2 = db.save_copy_settings(&settings2).await.unwrap();

    let mut settings3 = create_test_settings();
    settings3.master_account = "MASTER_001".to_string();
    settings3.slave_account = "SLAVE_003".to_string();
    settings3.status = 2; // CONNECTED
    let id3 = db.save_copy_settings(&settings3).await.unwrap();

    // Update master statuses to ENABLED (disconnected)
    let count = db
        .update_master_statuses_disconnected("MASTER_001")
        .await
        .unwrap();

    // Should update 1 setting (the connected one)
    assert_eq!(count, 1);

    // Verify statuses
    let retrieved1 = db.get_copy_settings(id1).await.unwrap().unwrap();
    assert_eq!(retrieved1.status, 0); // Still DISABLED

    let retrieved2 = db.get_copy_settings(id2).await.unwrap().unwrap();
    assert_eq!(retrieved2.status, 1); // Still ENABLED

    let retrieved3 = db.get_copy_settings(id3).await.unwrap().unwrap();
    assert_eq!(retrieved3.status, 1); // Now ENABLED (was CONNECTED)
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
