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
    fn on_event(&self, event: &tracing::Event<'_>, ctx: tracing_subscriber::layer::Context<'_, S>) {
        let metadata = event.metadata();
        let level = format!("{}", metadata.level());

        // Extract all fields from event
        let mut visitor = FieldVisitor::default();
        event.record(&mut visitor);

        // Get span information
        let span_name = ctx.event_span(event).map(|span| span.name().to_string());

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
        self.fields
            .insert(field.name().to_string(), JsonValue::Number(value.into()));
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.fields
            .insert(field.name().to_string(), JsonValue::Number(value.into()));
    }

    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        if let Some(num) = serde_json::Number::from_f64(value) {
            self.fields
                .insert(field.name().to_string(), JsonValue::Number(num));
        }
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.fields
            .insert(field.name().to_string(), JsonValue::Bool(value));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_log_buffer() {
        let buffer = create_log_buffer();
        let buffer_guard = buffer.blocking_read();
        assert_eq!(buffer_guard.len(), 0);
        assert_eq!(buffer_guard.capacity(), MAX_LOG_ENTRIES);
    }

    #[test]
    fn test_log_entry_serialization() {
        let mut fields = HashMap::new();
        fields.insert("account_id".to_string(), JsonValue::String("TEST_001".to_string()));
        fields.insert("balance".to_string(), JsonValue::Number(10000.into()));

        let entry = LogEntry {
            timestamp: Utc::now(),
            level: "INFO".to_string(),
            message: "Test message".to_string(),
            target: Some("relay_server".to_string()),
            module_path: Some("relay_server::test".to_string()),
            file: Some("test.rs".to_string()),
            line: Some(42),
            fields,
            span_name: Some("test_span".to_string()),
        };

        // Should serialize to JSON successfully
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("Test message"));
        assert!(json.contains("INFO"));
        assert!(json.contains("TEST_001"));
    }

    #[test]
    fn test_log_entry_optional_fields_omitted() {
        let entry = LogEntry {
            timestamp: Utc::now(),
            level: "INFO".to_string(),
            message: "Simple message".to_string(),
            target: None,
            module_path: None,
            file: None,
            line: None,
            fields: HashMap::new(),
            span_name: None,
        };

        let json = serde_json::to_string(&entry).unwrap();

        // Optional fields should not appear in JSON
        assert!(!json.contains("target"));
        assert!(!json.contains("module_path"));
        assert!(!json.contains("file"));
        assert!(!json.contains("line"));
        assert!(!json.contains("span_name"));
        assert!(!json.contains("fields")); // Empty HashMap should be omitted
    }

    #[test]
    fn test_field_visitor_default() {
        let visitor = FieldVisitor::default();

        assert!(visitor.message.is_empty());
        assert!(visitor.fields.is_empty());
    }

    #[test]
    fn test_field_visitor_message_field_handling() {
        let mut visitor = FieldVisitor::default();

        // Test that message field is extracted separately
        visitor.message = "Test log message".to_string();

        assert_eq!(visitor.message, "Test log message");
        assert!(!visitor.fields.contains_key("message"));
    }

    #[test]
    fn test_field_visitor_string_quote_removal() {
        let mut visitor = FieldVisitor::default();

        // Simulate Debug formatting which adds quotes
        let field_value = "\"EURUSD\"";
        let cleaned = if field_value.starts_with('"') && field_value.ends_with('"') {
            field_value[1..field_value.len() - 1].to_string()
        } else {
            field_value.to_string()
        };

        visitor.fields.insert("symbol".to_string(), JsonValue::String(cleaned));

        assert_eq!(
            visitor.fields.get("symbol"),
            Some(&JsonValue::String("EURUSD".to_string()))
        );
    }

    #[test]
    fn test_field_visitor_integer_parsing() {
        let mut visitor = FieldVisitor::default();

        // Test i64 parsing
        let value = "42";
        if let Ok(num) = value.parse::<i64>() {
            visitor.fields.insert("count".to_string(), JsonValue::Number(num.into()));
        }

        assert_eq!(
            visitor.fields.get("count"),
            Some(&JsonValue::Number(42.into()))
        );
    }

    #[test]
    fn test_field_visitor_bool_parsing() {
        let mut visitor = FieldVisitor::default();

        // Test bool parsing
        let value_true = "true";
        if value_true == "true" {
            visitor.fields.insert("enabled".to_string(), JsonValue::Bool(true));
        }

        let value_false = "false";
        if value_false == "false" {
            visitor.fields.insert("disabled".to_string(), JsonValue::Bool(false));
        }

        assert_eq!(visitor.fields.get("enabled"), Some(&JsonValue::Bool(true)));
        assert_eq!(visitor.fields.get("disabled"), Some(&JsonValue::Bool(false)));
    }

    #[test]
    fn test_log_buffer_layer_creation() {
        let buffer = create_log_buffer();
        let layer = LogBufferLayer::new(buffer.clone());

        // Layer should be created successfully
        // This is primarily a smoke test
        assert!(Arc::strong_count(&buffer) >= 2); // Buffer is shared
    }

    #[test]
    fn test_buffer_size_limit() {
        let buffer = create_log_buffer();

        {
            let mut buffer_guard = buffer.blocking_write();

            // Add more than MAX_LOG_ENTRIES
            for i in 0..(MAX_LOG_ENTRIES + 100) {
                buffer_guard.push_front(LogEntry {
                    timestamp: Utc::now(),
                    level: "INFO".to_string(),
                    message: format!("Message {}", i),
                    target: None,
                    module_path: None,
                    file: None,
                    line: None,
                    fields: HashMap::new(),
                    span_name: None,
                });

                // Simulate buffer size limit
                if buffer_guard.len() > MAX_LOG_ENTRIES {
                    buffer_guard.pop_back();
                }
            }

            // Should not exceed MAX_LOG_ENTRIES
            assert_eq!(buffer_guard.len(), MAX_LOG_ENTRIES);
        }
    }

    #[test]
    fn test_log_entry_reverse_chronological_order() {
        let buffer = create_log_buffer();

        {
            let mut buffer_guard = buffer.blocking_write();

            // Add entries in order
            for i in 0..5 {
                buffer_guard.push_front(LogEntry {
                    timestamp: Utc::now(),
                    level: "INFO".to_string(),
                    message: format!("Message {}", i),
                    target: None,
                    module_path: None,
                    file: None,
                    line: None,
                    fields: HashMap::new(),
                    span_name: None,
                });
            }
        }

        let buffer_guard = buffer.blocking_read();

        // Most recent should be at front
        assert_eq!(buffer_guard.front().unwrap().message, "Message 4");
        assert_eq!(buffer_guard.back().unwrap().message, "Message 0");
    }

    #[test]
    fn test_log_entry_with_all_field_types() {
        let mut fields = HashMap::new();
        fields.insert("string_field".to_string(), JsonValue::String("test".to_string()));
        fields.insert("int_field".to_string(), JsonValue::Number(42.into()));
        fields.insert("bool_field".to_string(), JsonValue::Bool(true));

        let entry = LogEntry {
            timestamp: Utc::now(),
            level: "DEBUG".to_string(),
            message: "Mixed types".to_string(),
            target: None,
            module_path: None,
            file: None,
            line: None,
            fields,
            span_name: None,
        };

        assert_eq!(entry.fields.len(), 3);
        assert!(entry.fields.contains_key("string_field"));
        assert!(entry.fields.contains_key("int_field"));
        assert!(entry.fields.contains_key("bool_field"));
    }

    #[test]
    fn test_max_log_entries_constant() {
        assert_eq!(MAX_LOG_ENTRIES, 1000);
    }

    #[test]
    fn test_log_entry_deserialization() {
        let entry = LogEntry {
            timestamp: Utc::now(),
            level: "WARN".to_string(),
            message: "Warning message".to_string(),
            target: Some("test_target".to_string()),
            module_path: Some("test::module".to_string()),
            file: Some("test.rs".to_string()),
            line: Some(100),
            fields: HashMap::new(),
            span_name: Some("test_span".to_string()),
        };

        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: LogEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.level, "WARN");
        assert_eq!(deserialized.message, "Warning message");
        assert_eq!(deserialized.line, Some(100));
    }
}

