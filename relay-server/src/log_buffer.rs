use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::Subscriber;
use tracing_subscriber::Layer;

const MAX_LOG_ENTRIES: usize = 1000;

/// A structured log entry with comprehensive metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub fields: HashMap<String, JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span_name: Option<String>,
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
    S: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let metadata = event.metadata();
        let level = format!("{}", metadata.level());

        // Extract all fields from event
        let mut visitor = FieldVisitor::default();
        event.record(&mut visitor);

        // Get span information
        let span_name = ctx
            .event_span(event)
            .map(|span| span.name().to_string());

        let entry = LogEntry {
            timestamp: Utc::now(),
            level,
            message: visitor.message,
            target: Some(metadata.target().to_string()),
            module_path: metadata.module_path().map(|s| s.to_string()),
            file: metadata.file().map(|s| s.to_string()),
            line: metadata.line(),
            fields: visitor.fields,
            span_name,
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

/// Visitor to extract all fields from tracing event
#[derive(Default)]
struct FieldVisitor {
    message: String,
    fields: HashMap<String, JsonValue>,
}

impl tracing::field::Visit for FieldVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        let field_name = field.name();
        let field_value = format!("{:?}", value);

        if field_name == "message" {
            self.message = field_value.clone();
            // Remove surrounding quotes if present
            if self.message.starts_with('"') && self.message.ends_with('"') {
                self.message = self.message[1..self.message.len() - 1].to_string();
            }
        } else {
            // Store other fields in the fields map
            // Try to parse as JSON value, fallback to string
            let json_value = if let Ok(num) = field_value.parse::<i64>() {
                JsonValue::Number(num.into())
            } else if let Ok(num) = field_value.parse::<f64>() {
                serde_json::Number::from_f64(num)
                    .map(JsonValue::Number)
                    .unwrap_or_else(|| JsonValue::String(field_value.clone()))
            } else if field_value == "true" {
                JsonValue::Bool(true)
            } else if field_value == "false" {
                JsonValue::Bool(false)
            } else {
                // Remove surrounding quotes for strings
                let cleaned = if field_value.starts_with('"') && field_value.ends_with('"') {
                    field_value[1..field_value.len() - 1].to_string()
                } else {
                    field_value
                };
                JsonValue::String(cleaned)
            };

            self.fields.insert(field_name.to_string(), json_value);
        }
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.fields.insert(
            field.name().to_string(),
            JsonValue::Number(value.into()),
        );
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.fields.insert(
            field.name().to_string(),
            JsonValue::Number(value.into()),
        );
    }

    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        if let Some(num) = serde_json::Number::from_f64(value) {
            self.fields.insert(
                field.name().to_string(),
                JsonValue::Number(num),
            );
        }
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.fields.insert(
            field.name().to_string(),
            JsonValue::Bool(value),
        );
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        } else {
            self.fields.insert(
                field.name().to_string(),
                JsonValue::String(value.to_string()),
            );
        }
    }
}
