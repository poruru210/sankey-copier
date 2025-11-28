//! VictoriaLogs integration for direct HTTP log shipping from EA
//!
//! This module provides FFI functions for MQL EAs to send logs directly
//! to VictoriaLogs via HTTP, bypassing ZMQ for simplicity.
//!
//! Architecture:
//! - EA calls vlogs_add_entry() to buffer log entries
//! - EA calls vlogs_flush() periodically to send buffered entries
//! - Logs are sent as JSON Lines to VictoriaLogs /insert/jsonline endpoint

use chrono::Utc;
use serde::Serialize;
use std::sync::{LazyLock, Mutex};

/// Log entry structure matching VictoriaLogs JSON Line format
#[derive(Debug, Clone, Serialize)]
struct LogEntry {
    #[serde(rename = "_time")]
    time: String,
    #[serde(rename = "_msg")]
    msg: String,
    level: String,
    source: String,
    category: String,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    context: Option<serde_json::Value>,
}

/// VictoriaLogs configuration
#[derive(Debug, Default)]
struct VLogsConfig {
    enabled: bool,
    endpoint: String,
    source: String,
}

// Global state for configuration and log buffer
static CONFIG: LazyLock<Mutex<VLogsConfig>> = LazyLock::new(|| Mutex::new(VLogsConfig::default()));
static BUFFER: LazyLock<Mutex<Vec<LogEntry>>> = LazyLock::new(|| Mutex::new(Vec::new()));

// HTTP client (created lazily on first flush)
static CLIENT: LazyLock<reqwest::blocking::Client> = LazyLock::new(|| {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new())
});

/// Convert UTF-16 pointer (from MQL) to Rust String
///
/// # Safety
/// Pointer must be valid and null-terminated
unsafe fn utf16_to_string(ptr: *const u16) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    let mut len = 0;
    while *ptr.add(len) != 0 {
        len += 1;
    }
    if len == 0 {
        return Some(String::new());
    }
    let slice = std::slice::from_raw_parts(ptr, len);
    String::from_utf16(slice).ok()
}

/// Configure VictoriaLogs endpoint and source identifier
///
/// # Parameters
/// - endpoint: VictoriaLogs URL (e.g., "http://localhost:9428/insert/jsonline")
/// - source: Source identifier for logs (e.g., "ea:IC_Markets_12345")
///
/// # Returns
/// 1 on success, 0 on failure
#[no_mangle]
pub unsafe extern "C" fn vlogs_configure(endpoint: *const u16, source: *const u16) -> i32 {
    let endpoint_str = match utf16_to_string(endpoint) {
        Some(s) if !s.is_empty() => s,
        _ => {
            eprintln!("vlogs_configure: invalid or empty endpoint");
            return 0;
        }
    };

    let source_str = match utf16_to_string(source) {
        Some(s) if !s.is_empty() => s,
        _ => {
            eprintln!("vlogs_configure: invalid or empty source");
            return 0;
        }
    };

    match CONFIG.lock() {
        Ok(mut config) => {
            config.endpoint = endpoint_str;
            config.source = source_str;
            config.enabled = true;
            1
        }
        Err(e) => {
            eprintln!("vlogs_configure: failed to lock config: {}", e);
            0
        }
    }
}

/// Disable VictoriaLogs logging
///
/// # Returns
/// 1 on success, 0 on failure
#[no_mangle]
pub extern "C" fn vlogs_disable() -> i32 {
    match CONFIG.lock() {
        Ok(mut config) => {
            config.enabled = false;
            1
        }
        Err(e) => {
            eprintln!("vlogs_disable: failed to lock config: {}", e);
            0
        }
    }
}

/// Add a log entry to the buffer
///
/// # Parameters
/// - level: Log level ("DEBUG", "INFO", "WARN", "ERROR")
/// - category: Log category ("Trade", "Config", "Sync", "System")
/// - message: Log message
/// - context_json: Optional JSON string with additional context (can be empty)
///
/// # Returns
/// Current buffer size, or -1 on failure
#[no_mangle]
pub unsafe extern "C" fn vlogs_add_entry(
    level: *const u16,
    category: *const u16,
    message: *const u16,
    context_json: *const u16,
) -> i32 {
    // Check if enabled first
    let (enabled, source) = match CONFIG.lock() {
        Ok(config) => (config.enabled, config.source.clone()),
        Err(_) => return -1,
    };

    if !enabled {
        return 0;
    }

    let level_str = utf16_to_string(level).unwrap_or_else(|| "INFO".to_string());
    let category_str = utf16_to_string(category).unwrap_or_else(|| "System".to_string());
    let message_str = match utf16_to_string(message) {
        Some(s) => s,
        None => return -1,
    };

    // Parse context JSON if provided
    let context = utf16_to_string(context_json)
        .filter(|s| !s.is_empty())
        .and_then(|s| serde_json::from_str(&s).ok());

    let entry = LogEntry {
        time: Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
        msg: message_str,
        level: level_str,
        source,
        category: category_str,
        context,
    };

    match BUFFER.lock() {
        Ok(mut buffer) => {
            buffer.push(entry);
            buffer.len() as i32
        }
        Err(e) => {
            eprintln!("vlogs_add_entry: failed to lock buffer: {}", e);
            -1
        }
    }
}

/// Flush buffered log entries to VictoriaLogs
///
/// Sends all buffered entries as JSON Lines to the configured endpoint.
/// Clears the buffer after successful send.
///
/// # Returns
/// 1 on success, 0 on failure or if disabled, -1 on error
#[no_mangle]
pub extern "C" fn vlogs_flush() -> i32 {
    // Get config
    let endpoint = match CONFIG.lock() {
        Ok(config) => {
            if !config.enabled {
                return 0;
            }
            config.endpoint.clone()
        }
        Err(_) => return -1,
    };

    // Get and clear buffer
    let entries = match BUFFER.lock() {
        Ok(mut buffer) => {
            if buffer.is_empty() {
                return 1; // Nothing to flush, success
            }
            std::mem::take(&mut *buffer)
        }
        Err(_) => return -1,
    };

    // Send to VictoriaLogs
    match send_entries(&endpoint, &entries) {
        Ok(_) => 1,
        Err(e) => {
            eprintln!("vlogs_flush: failed to send: {}", e);
            // Put entries back in buffer for retry
            if let Ok(mut buffer) = BUFFER.lock() {
                for entry in entries {
                    buffer.push(entry);
                }
            }
            0
        }
    }
}

/// Get current buffer size
///
/// # Returns
/// Number of entries in buffer, or -1 on error
#[no_mangle]
pub extern "C" fn vlogs_buffer_size() -> i32 {
    match BUFFER.lock() {
        Ok(buffer) => buffer.len() as i32,
        Err(_) => -1,
    }
}

/// Internal function to send entries to VictoriaLogs
fn send_entries(endpoint: &str, entries: &[LogEntry]) -> Result<(), String> {
    // Build JSON Lines body
    let body = entries
        .iter()
        .filter_map(|e| serde_json::to_string(e).ok())
        .collect::<Vec<_>>()
        .join("\n");

    // Send HTTP POST request
    let response = CLIENT
        .post(endpoint)
        .header("Content-Type", "application/x-ndjson")
        .body(body)
        .send()
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!("HTTP error: {}", response.status()))
    }
}

// ============================================================================
// Internal API for testing (not exported via FFI)
// ============================================================================

/// Configure VictoriaLogs (internal API for tests)
/// Handles poisoned mutex by recovering the guard
#[allow(dead_code)]
pub(crate) fn configure(endpoint: &str, source: &str) {
    let mut config = CONFIG.lock().unwrap_or_else(|e| e.into_inner());
    config.endpoint = endpoint.to_string();
    config.source = source.to_string();
    config.enabled = true;
}

/// Add log entry (internal API for tests)
/// Respects the enabled flag - does nothing if disabled
#[allow(dead_code)]
pub(crate) fn add_entry(
    level: &str,
    category: &str,
    message: &str,
    context: Option<serde_json::Value>,
) {
    // Check if enabled and get source
    let (enabled, source) = match CONFIG.lock() {
        Ok(config) => (config.enabled, config.source.clone()),
        Err(e) => {
            let config = e.into_inner();
            (config.enabled, config.source.clone())
        }
    };

    if !enabled {
        return;
    }

    let entry = LogEntry {
        time: Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
        msg: message.to_string(),
        level: level.to_string(),
        source,
        category: category.to_string(),
        context,
    };

    match BUFFER.lock() {
        Ok(mut buffer) => buffer.push(entry),
        Err(e) => e.into_inner().push(entry),
    }
}

/// Flush and return result (internal API for tests)
/// Handles poisoned mutex by recovering the guard
#[allow(dead_code)]
pub(crate) fn flush() -> i32 {
    // Get config (with poison recovery)
    let endpoint = {
        let config = CONFIG.lock().unwrap_or_else(|e| e.into_inner());
        if !config.enabled {
            return 0;
        }
        config.endpoint.clone()
    };

    // Get and clear buffer (with poison recovery)
    let entries = {
        let mut buffer = BUFFER.lock().unwrap_or_else(|e| e.into_inner());
        if buffer.is_empty() {
            return 1; // Nothing to flush, success
        }
        std::mem::take(&mut *buffer)
    };

    // Send to VictoriaLogs
    match send_entries(&endpoint, &entries) {
        Ok(_) => 1,
        Err(e) => {
            eprintln!("vlogs_flush: failed to send: {}", e);
            // Put entries back in buffer for retry
            let mut buffer = BUFFER.lock().unwrap_or_else(|e| e.into_inner());
            for entry in entries {
                buffer.push(entry);
            }
            0
        }
    }
}

/// Clear buffer (internal API for tests)
/// Handles poisoned mutex by recovering the guard
#[allow(dead_code)]
pub(crate) fn clear_buffer() {
    let mut buffer = BUFFER.lock().unwrap_or_else(|e| e.into_inner());
    buffer.clear();
}

/// Reset config (internal API for tests)
/// Handles poisoned mutex by recovering the guard
#[allow(dead_code)]
pub(crate) fn reset_config() {
    {
        let mut config = CONFIG.lock().unwrap_or_else(|e| e.into_inner());
        *config = VLogsConfig::default();
    }
    clear_buffer();
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn setup() {
        reset_config();
    }

    #[test]
    #[serial]
    fn test_configure_and_add_entry() {
        setup();

        configure("http://localhost:9428/insert/jsonline", "test-source");

        add_entry("INFO", "Trade", "Test message", None);

        let buffer = BUFFER.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer[0].msg, "Test message");
        assert_eq!(buffer[0].level, "INFO");
        assert_eq!(buffer[0].category, "Trade");
        assert_eq!(buffer[0].source, "test-source");
    }

    #[test]
    #[serial]
    fn test_add_entry_with_context() {
        setup();

        configure("http://localhost:9428/insert/jsonline", "test-source");

        let context = serde_json::json!({"ticket": 12345, "symbol": "EURUSD"});
        add_entry("INFO", "Trade", "Open position", Some(context));

        let buffer = BUFFER.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(buffer.len(), 1);
        assert!(buffer[0].context.is_some());

        let ctx = buffer[0].context.as_ref().unwrap();
        assert_eq!(ctx["ticket"], 12345);
        assert_eq!(ctx["symbol"], "EURUSD");
    }

    #[test]
    #[serial]
    fn test_disabled_does_not_buffer() {
        setup();

        // Don't configure - disabled by default
        add_entry("INFO", "Trade", "Should not be buffered", None);

        let buffer = BUFFER.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    #[serial]
    fn test_flush_with_mockito() {
        setup();

        let mut server = mockito::Server::new();
        let mock = server
            .mock("POST", "/insert/jsonline")
            .match_header("content-type", "application/x-ndjson")
            .with_status(204)
            .create();

        let endpoint = format!("{}/insert/jsonline", server.url());
        configure(&endpoint, "test-source");

        add_entry("INFO", "Trade", "Test message", None);

        let result = flush();
        assert_eq!(result, 1);

        mock.assert();

        // Buffer should be cleared
        let buffer = BUFFER.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    #[serial]
    fn test_flush_empty_buffer() {
        setup();

        configure("http://localhost:9428/insert/jsonline", "test-source");

        // Flush without any entries
        let result = flush();
        assert_eq!(result, 1); // Success (nothing to do)
    }

    #[test]
    #[serial]
    fn test_flush_failure_preserves_buffer() {
        setup();

        // Configure with invalid endpoint that will fail quickly
        configure("http://127.0.0.1:1/insert/jsonline", "test-source");

        add_entry("INFO", "Trade", "Test message", None);

        let result = flush();
        assert_eq!(result, 0); // Failure

        // Buffer should still have the entry
        let buffer = BUFFER.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(buffer.len(), 1);
    }

    #[test]
    #[serial]
    fn test_json_line_format() {
        setup();

        configure("http://localhost:9428/insert/jsonline", "ea:IC_12345");

        let context = serde_json::json!({"ticket": 99});
        add_entry("ERROR", "System", "Connection failed", Some(context));

        let buffer = BUFFER.lock().unwrap_or_else(|e| e.into_inner());
        let entry = &buffer[0];

        // Serialize and check format
        let json = serde_json::to_string(entry).unwrap();
        assert!(json.contains("\"_time\":"));
        assert!(json.contains("\"_msg\":\"Connection failed\""));
        assert!(json.contains("\"level\":\"ERROR\""));
        assert!(json.contains("\"source\":\"ea:IC_12345\""));
        assert!(json.contains("\"category\":\"System\""));
        assert!(json.contains("\"ticket\":99"));
    }
}
