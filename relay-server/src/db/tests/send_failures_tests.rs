use crate::db::Database;

#[tokio::test]
async fn test_record_and_fetch_failed_send() {
    let db = Database::new("sqlite::memory:").await.unwrap();

    let topic = "test/topic";
    let payload = vec![1u8, 2, 3, 4];
    let error = "serialize error";

    let id = db.record_failed_send(topic, &payload, error, 1).await.unwrap();
    assert!(id > 0);

    let pending = db.fetch_pending_failed_sends(10).await.unwrap();
    assert!(!pending.is_empty());
    let (f_id, f_topic, f_payload, f_attempts) = &pending[0];
    assert_eq!(f_id, &id);
    assert_eq!(f_topic, topic);
    assert_eq!(f_payload, &payload);
    assert_eq!(*f_attempts, 1);

    let rows = db.mark_failed_send_processed(id).await.unwrap();
    assert_eq!(rows, 1);

    // now there should be no pending items
    let pending2 = db.fetch_pending_failed_sends(10).await.unwrap();
    assert!(pending2.is_empty());

    // Test incrementing attempts on a new failure
    let id2 = db.record_failed_send(topic, &payload, error, 0).await.unwrap();
    let pending3 = db.fetch_pending_failed_sends(10).await.unwrap();
    assert!(!pending3.is_empty());
    let (_, _, _, att) = &pending3[0];
    assert_eq!(*att, 0);

    let inc_rows = db.increment_failed_send_attempts(id2).await.unwrap();
    assert_eq!(inc_rows, 1);

    let pending4 = db.fetch_pending_failed_sends(10).await.unwrap();
    let (_, _, _, att2) = &pending4[0];
    assert_eq!(*att2, 1);
}
