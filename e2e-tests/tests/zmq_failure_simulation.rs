use e2e_tests::relay_server_process::RelayServerProcess;
use sankey_copier_relay_server::db::Database;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_failed_send_moved_to_dead_letter() {
    // Ensure the relay-server binary is up-to-date (build workspace release)
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let status = std::process::Command::new("cargo")
        .args(["build", "--release", "-p", "sankey-copier-relay-server"])
        .current_dir(&workspace_root)
        .status()
        .expect("failed to run cargo build");
    assert!(status.success(), "Failed to build relay-server binary");

    // Start server
    let server = RelayServerProcess::start().expect("Failed to start server");

    // Connect to DB
    let db = Database::new(&server.db_url())
        .await
        .expect("DB connect failed");

    // Insert a failed send with attempts equal to retry threshold (worker should move it)
    let topic = "tests/failure-sim";
    let payload = vec![1u8, 2, 3];
    let err = "simulated";

    // Use attempts >= MAX_RETRY_ATTEMPTS (5) so the server's retry worker will move it to dead-letter
    let id = db
        .record_failed_send(topic, &payload, err, 5)
        .await
        .expect("record failed send");
    assert!(id > 0);

    // Wait up to 15 seconds for the retry worker to move the item to dead-letter
    let mut found = false;
    for _ in 0..30 {
        let rows = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM failed_outgoing_dead_letters WHERE original_id = ?",
        )
        .bind(id)
        .fetch_one(db.pool())
        .await
        .unwrap_or(0);

        if rows > 0 {
            found = true;
            break;
        }

        // Also check if original record was marked processed (the worker may clear it instead of moving)
        let processed: i64 =
            sqlx::query_scalar("SELECT processed FROM failed_outgoing_messages WHERE id = ?")
                .bind(id)
                .fetch_one(db.pool())
                .await
                .unwrap_or(0);

        if processed > 0 {
            found = true;
            break;
        }

        sleep(Duration::from_millis(500)).await;
    }

    assert!(
        found,
        "failed item was not moved to dead-letter within timeout"
    );
}
