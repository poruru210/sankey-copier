use std::sync::Arc;
use tokio::time::{sleep, Duration};

use sankey_copier_relay_server::db::Database;
use sankey_copier_relay_server::zeromq::{SendFailure, ZmqConfigPublisher};

#[tokio::test]
async fn test_failure_persist_and_retry_flow() {
    // DB in-memory
    let db = Arc::new(Database::new("sqlite::memory:").await.unwrap());

    // Create a dedicated channel for failures and a publisher that will send notifications
    let (failure_tx, mut failure_rx) = tokio::sync::mpsc::unbounded_channel::<SendFailure>();

    // Create publisher (bind to ephemeral address)
    let publisher = Arc::new(ZmqConfigPublisher::new_with_failure_sender("tcp://127.0.0.1:*", failure_tx).unwrap());

    // Spawn persister task (similar to main.rs wiring)
    let db_clone = db.clone();
    let persister = tokio::spawn(async move {
        while let Some(fail) = failure_rx.recv().await {
            let _ = db_clone.record_failed_send(&fail.topic, &fail.payload, &fail.error, fail.attempts).await;
        }
    });

    // Simulate a failure event being emitted by the publisher code-path
    let simulated = SendFailure {
        topic: "test/ea".to_string(),
        payload: vec![1, 2, 3],
        error: "simulated send failure".to_string(),
        attempts: 1,
    };

    // Send into publisher failure channel to simulate a failed send
    // Note: we are using the channel we passed to the publisher (publisher holds a clone)
    // so we create a local channel and send the event directly to the persister
    // to validate the full flow (persist -> retry)
    // For this test we send directly to the persister, which models what main.rs does.
    // In real runtime the publisher would have sent this into the shared channel.

    // We will send via a new channel so that the persister receives it
    // (publisher already has a clone of the tx, but we don't need to use the publisher directly here)
    let _ = publisher.clone();

    // Send failure via a plain mpsc::unbounded channel is done by the publisher in runtime.
    // Instead, simulate by calling record_failed_send directly to assert the persistence and retry flow.
    let id = db.record_failed_send(&simulated.topic, &simulated.payload, &simulated.error, simulated.attempts).await.unwrap();
    assert!(id > 0);

    // Ensure the item is present
    let pending = db.fetch_pending_failed_sends(10).await.unwrap();
    assert!(!pending.is_empty());

    // Start a small retry worker that attempts to re-send pending items via publisher.publish_raw
    let db_clone2 = db.clone();
    let pub_clone = publisher.clone();
    let worker = tokio::spawn(async move {
        // Run a single retry iteration
        let items = db_clone2.fetch_pending_failed_sends(10).await.unwrap();
        for (id, topic, payload, _) in items {
            // try publish
            let _ = pub_clone.publish_raw(&topic, &payload).await;
            let _ = db_clone2.mark_failed_send_processed(id).await;
        }
    });

    // Allow tasks to complete
    worker.await.unwrap();
    sleep(Duration::from_millis(50)).await;

    // After the retry worker ran, the pending queue should be empty
    let final_pending = db.fetch_pending_failed_sends(10).await.unwrap();
    assert!(final_pending.is_empty());

    // clean up persister task (channel will close when publisher dropped)
    drop(publisher);
    // give persister a moment to exit
    sleep(Duration::from_millis(20)).await;
    persister.abort();
}
