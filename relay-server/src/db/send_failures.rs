use anyhow::Result;
use sqlx::Row;

use super::Database;

/// Record a failed outgoing ZMQ send for later inspection or retry
impl Database {
    pub async fn record_failed_send(
        &self,
        topic: &str,
        payload: &[u8],
        error: &str,
        attempts: i32,
    ) -> Result<i64> {
        let res = sqlx::query(
            "INSERT INTO failed_outgoing_messages (topic, payload, error, attempts) VALUES (?, ?, ?, ?)",
        )
        .bind(topic)
        .bind(payload)
        .bind(error)
        .bind(attempts)
        .execute(&self.pool)
        .await?;

        Ok(res.last_insert_rowid())
    }

    pub async fn fetch_pending_failed_sends(&self, limit: i64) -> Result<Vec<(i64, String, Vec<u8>, i32, String)>> {
        let rows = sqlx::query(
            "SELECT id, topic, payload, attempts, updated_at FROM failed_outgoing_messages WHERE processed = 0 ORDER BY created_at LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut results = Vec::new();
        for row in rows {
            let id: i64 = row.get("id");
            let topic: String = row.get("topic");
            let payload: Vec<u8> = row.get("payload");
            let attempts: i32 = row.get("attempts");
            let updated: String = row.get("updated_at");
            results.push((id, topic, payload, attempts, updated));
        }

        Ok(results)
    }

    pub async fn move_failed_to_dead_letter(&self, id: i64) -> Result<usize> {

        // Insert a copy into dead letters and mark original as processed
        sqlx::query(
            "INSERT INTO failed_outgoing_dead_letters (original_id, topic, payload, error, attempts) SELECT id, topic, payload, error, attempts FROM failed_outgoing_messages WHERE id = ?",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        // Mark original as processed
        let res = sqlx::query(
            "UPDATE failed_outgoing_messages SET processed = 1, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(res.rows_affected() as usize)
    }

    pub async fn mark_failed_send_processed(&self, id: i64) -> Result<usize> {
        let res = sqlx::query("UPDATE failed_outgoing_messages SET processed = 1, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(res.rows_affected() as usize)
    }

    pub async fn increment_failed_send_attempts(&self, id: i64) -> Result<usize> {
        let res = sqlx::query("UPDATE failed_outgoing_messages SET attempts = attempts + 1, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(res.rows_affected() as usize)
    }
}
