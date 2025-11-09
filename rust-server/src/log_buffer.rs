use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::Subscriber;
use tracing_subscriber::Layer;

const MAX_LOG_ENTRIES: usize = 1000;

/// A log entry with timestamp, level, and message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub message: String,
}

/// Thread-safe log buffer
pub type LogBuffer = Arc<RwLock<VecDeque<LogEntry>>>;

/// Create a new empty log buffer
pub fn create_log_buffer() -> LogBuffer {
    Arc::new(RwLock::new(VecDeque::with_capacity(MAX_LOG_ENTRIES)))
}

/// Custom tracing layer that captures logs into a buffer
pub struct LogBufferLayer {
    buffer: LogBuffer,
}

impl LogBufferLayer {
    pub fn new(buffer: LogBuffer) -> Self {
        Self { buffer }
    }
}

impl<S> Layer<S> for LogBufferLayer
where
    S: Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let metadata = event.metadata();
        let level = format!("{}", metadata.level());

        // Extract message from event
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);

        let entry = LogEntry {
            timestamp: Utc::now(),
            level,
            message: visitor.message,
        };

        // Add to buffer (blocking is acceptable for logging)
        if let Ok(mut buffer) = self.buffer.try_write() {
            buffer.push_front(entry); // Add to front for reverse chronological order

            // Keep buffer size bounded
            if buffer.len() > MAX_LOG_ENTRIES {
                buffer.pop_back();
            }
        }
    }
}

/// Visitor to extract message from tracing event
#[derive(Default)]
struct MessageVisitor {
    message: String,
}

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
            // Remove surrounding quotes if present
            if self.message.starts_with('"') && self.message.ends_with('"') {
                self.message = self.message[1..self.message.len() - 1].to_string();
            }
        }
    }
}
