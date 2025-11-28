//! VictoriaLogs integration for relay-server
//!
//! This module provides a tracing Layer that ships logs to VictoriaLogs
//! via HTTP. Logs are buffered and sent in batches for efficiency.
//!
//! Architecture:
//! - VictoriaLogsLayer captures tracing events
//! - Events are sent to a background task via mpsc channel
//! - Background task batches and sends logs via HTTP

use crate::config::VictoriaLogsConfig;
use chrono::Utc;
use serde::Serialize;
use tokio::sync::mpsc;
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

/// Log entry structure matching VictoriaLogs JSON Line format
#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    #[serde(rename = "_time")]
    pub time: String,
    #[serde(rename = "_msg")]
    pub msg: String,
    pub level: String,
    pub source: String,
    pub target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
}

/// Internal message type for the background task
#[derive(Debug)]
#[allow(dead_code)]
enum LogMessage {
    Entry(LogEntry),
    Flush,
    Shutdown,
}

/// VictoriaLogs tracing layer
///
/// Captures tracing events and sends them to a background task
/// for batching and HTTP delivery.
pub struct VictoriaLogsLayer {
    sender: mpsc::Sender<LogMessage>,
    source: String,
}

impl VictoriaLogsLayer {
    /// Create a new VictoriaLogs layer and spawn the background task
    ///
    /// Returns the layer and a handle to the background task
    pub fn new(config: &VictoriaLogsConfig) -> (Self, tokio::task::JoinHandle<()>) {
        let (tx, rx) = mpsc::channel(1000);

        let task_config = config.clone();
        let handle = tokio::spawn(async move {
            background_sender(rx, task_config).await;
        });

        let layer = Self {
            sender: tx,
            source: config.source.clone(),
        };

        (layer, handle)
    }

    /// Request a flush of buffered logs
    #[allow(dead_code)]
    pub async fn flush(&self) {
        let _ = self.sender.send(LogMessage::Flush).await;
    }

    /// Request shutdown of the background task
    #[allow(dead_code)]
    pub async fn shutdown(&self) {
        let _ = self.sender.send(LogMessage::Shutdown).await;
    }
}

impl<S> Layer<S> for VictoriaLogsLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        // Extract event metadata
        let metadata = event.metadata();
        let level = match *metadata.level() {
            Level::ERROR => "ERROR",
            Level::WARN => "WARN",
            Level::INFO => "INFO",
            Level::DEBUG => "DEBUG",
            Level::TRACE => "TRACE",
        };

        // Build message from event fields
        let mut message = String::new();
        let mut visitor = MessageVisitor(&mut message);
        event.record(&mut visitor);

        let entry = LogEntry {
            time: Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
            msg: message,
            level: level.to_string(),
            source: self.source.clone(),
            target: metadata.target().to_string(),
            file: metadata.file().map(String::from),
            line: metadata.line(),
        };

        // Non-blocking send - drop if channel is full
        let _ = self.sender.try_send(LogMessage::Entry(entry));
    }
}

/// Visitor to extract message from tracing event fields
struct MessageVisitor<'a>(&'a mut String);

impl<'a> tracing::field::Visit for MessageVisitor<'a> {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.0.push_str(value);
        } else {
            if !self.0.is_empty() {
                self.0.push(' ');
            }
            self.0.push_str(field.name());
            self.0.push('=');
            self.0.push_str(value);
        }
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.0.push_str(&format!("{:?}", value));
        } else {
            if !self.0.is_empty() {
                self.0.push(' ');
            }
            self.0.push_str(field.name());
            self.0.push('=');
            self.0.push_str(&format!("{:?}", value));
        }
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        if !self.0.is_empty() {
            self.0.push(' ');
        }
        self.0.push_str(field.name());
        self.0.push('=');
        self.0.push_str(&value.to_string());
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        if !self.0.is_empty() {
            self.0.push(' ');
        }
        self.0.push_str(field.name());
        self.0.push('=');
        self.0.push_str(&value.to_string());
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        if !self.0.is_empty() {
            self.0.push(' ');
        }
        self.0.push_str(field.name());
        self.0.push('=');
        self.0.push_str(&value.to_string());
    }
}

/// Background task that batches and sends logs to VictoriaLogs
async fn background_sender(mut rx: mpsc::Receiver<LogMessage>, config: VictoriaLogsConfig) {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let mut buffer: Vec<LogEntry> = Vec::with_capacity(config.batch_size);
    let flush_interval = tokio::time::Duration::from_secs(config.flush_interval_secs);
    let mut flush_timer = tokio::time::interval(flush_interval);

    loop {
        tokio::select! {
            msg = rx.recv() => {
                match msg {
                    Some(LogMessage::Entry(entry)) => {
                        buffer.push(entry);
                        if buffer.len() >= config.batch_size {
                            send_batch(&client, &config.endpoint, &mut buffer).await;
                        }
                    }
                    Some(LogMessage::Flush) => {
                        send_batch(&client, &config.endpoint, &mut buffer).await;
                    }
                    Some(LogMessage::Shutdown) | None => {
                        // Final flush before shutdown
                        send_batch(&client, &config.endpoint, &mut buffer).await;
                        break;
                    }
                }
            }
            _ = flush_timer.tick() => {
                if !buffer.is_empty() {
                    send_batch(&client, &config.endpoint, &mut buffer).await;
                }
            }
        }
    }
}

/// Send a batch of log entries to VictoriaLogs
async fn send_batch(client: &reqwest::Client, endpoint: &str, buffer: &mut Vec<LogEntry>) {
    if buffer.is_empty() {
        return;
    }

    // Build JSON Lines body
    let body = buffer
        .iter()
        .filter_map(|e| serde_json::to_string(e).ok())
        .collect::<Vec<_>>()
        .join("\n");

    // Send HTTP POST request
    match client
        .post(endpoint)
        .header("Content-Type", "application/x-ndjson")
        .body(body)
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                buffer.clear();
            } else {
                // Keep buffer for retry on next interval
                tracing::warn!(
                    "VictoriaLogs HTTP error: {} - keeping {} entries for retry",
                    response.status(),
                    buffer.len()
                );
            }
        }
        Err(e) => {
            // Keep buffer for retry on next interval
            tracing::warn!(
                "VictoriaLogs send failed: {} - keeping {} entries for retry",
                e,
                buffer.len()
            );
        }
    }
}

/// Shared handle for controlling the VictoriaLogs layer
#[derive(Clone)]
#[allow(dead_code)]
pub struct VictoriaLogsHandle {
    sender: mpsc::Sender<LogMessage>,
}

impl VictoriaLogsHandle {
    /// Create a new handle from a layer
    pub fn new(layer: &VictoriaLogsLayer) -> Self {
        Self {
            sender: layer.sender.clone(),
        }
    }

    /// Request a flush of buffered logs
    #[allow(dead_code)]
    pub async fn flush(&self) {
        let _ = self.sender.send(LogMessage::Flush).await;
    }

    /// Request shutdown of the background task
    #[allow(dead_code)]
    pub async fn shutdown(&self) {
        let _ = self.sender.send(LogMessage::Shutdown).await;
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn create_test_config(endpoint: &str) -> VictoriaLogsConfig {
        VictoriaLogsConfig {
            enabled: true,
            endpoint: endpoint.to_string(),
            batch_size: 10,
            flush_interval_secs: 1,
            source: "test-relay".to_string(),
        }
    }

    #[test]
    fn test_log_entry_serialization() {
        let entry = LogEntry {
            time: "2025-01-15T10:30:45.123Z".to_string(),
            msg: "Test message".to_string(),
            level: "INFO".to_string(),
            source: "test-source".to_string(),
            target: "relay_server::test".to_string(),
            file: Some("test.rs".to_string()),
            line: Some(42),
        };

        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"_time\":\"2025-01-15T10:30:45.123Z\""));
        assert!(json.contains("\"_msg\":\"Test message\""));
        assert!(json.contains("\"level\":\"INFO\""));
        assert!(json.contains("\"source\":\"test-source\""));
        assert!(json.contains("\"target\":\"relay_server::test\""));
        assert!(json.contains("\"file\":\"test.rs\""));
        assert!(json.contains("\"line\":42"));
    }

    #[test]
    fn test_log_entry_without_optional_fields() {
        let entry = LogEntry {
            time: "2025-01-15T10:30:45.123Z".to_string(),
            msg: "Test message".to_string(),
            level: "INFO".to_string(),
            source: "test-source".to_string(),
            target: "relay_server::test".to_string(),
            file: None,
            line: None,
        };

        let json = serde_json::to_string(&entry).unwrap();
        assert!(!json.contains("\"file\""));
        assert!(!json.contains("\"line\""));
    }

    #[tokio::test]
    #[serial]
    async fn test_send_batch_to_mockito() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/insert/jsonline")
            .match_header("content-type", "application/x-ndjson")
            .with_status(204)
            .create_async()
            .await;

        let client = reqwest::Client::new();
        let endpoint = format!("{}/insert/jsonline", server.url());

        let mut buffer = vec![
            LogEntry {
                time: "2025-01-15T10:30:45.123Z".to_string(),
                msg: "Test message 1".to_string(),
                level: "INFO".to_string(),
                source: "test".to_string(),
                target: "test".to_string(),
                file: None,
                line: None,
            },
            LogEntry {
                time: "2025-01-15T10:30:46.123Z".to_string(),
                msg: "Test message 2".to_string(),
                level: "WARN".to_string(),
                source: "test".to_string(),
                target: "test".to_string(),
                file: None,
                line: None,
            },
        ];

        send_batch(&client, &endpoint, &mut buffer).await;

        mock.assert_async().await;
        assert!(buffer.is_empty());
    }

    #[tokio::test]
    #[serial]
    async fn test_send_batch_failure_preserves_buffer() {
        // Use invalid endpoint to trigger failure
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(100))
            .build()
            .unwrap();
        let endpoint = "http://127.0.0.1:1/insert/jsonline";

        let mut buffer = vec![LogEntry {
            time: "2025-01-15T10:30:45.123Z".to_string(),
            msg: "Test message".to_string(),
            level: "INFO".to_string(),
            source: "test".to_string(),
            target: "test".to_string(),
            file: None,
            line: None,
        }];

        send_batch(&client, endpoint, &mut buffer).await;

        // Buffer should be preserved on failure
        assert_eq!(buffer.len(), 1);
    }

    #[tokio::test]
    #[serial]
    async fn test_layer_creation() {
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("POST", "/insert/jsonline")
            .with_status(204)
            .create_async()
            .await;

        let config = create_test_config(&format!("{}/insert/jsonline", server.url()));
        let (layer, handle) = VictoriaLogsLayer::new(&config);

        // Verify handle can be created
        let vlogs_handle = VictoriaLogsHandle::new(&layer);

        // Request shutdown
        vlogs_handle.shutdown().await;

        // Wait for task to complete
        let _ = tokio::time::timeout(std::time::Duration::from_secs(2), handle).await;
    }
}
