//! VictoriaLogs integration for direct HTTP log shipping from EA
//!
//! This module provides FFI functions for MQL EAs to send logs directly
//! to VictoriaLogs via HTTP, bypassing ZMQ for simplicity.
//!
//! Architecture:
//! - EA calls vlogs_add_entry() to buffer log entries
//! - EA calls vlogs_flush() periodically to send buffered entries to background thread
//! - Background thread sends logs as JSON Lines to VictoriaLogs /insert/jsonline endpoint
//! - vlogs_flush() returns immediately (non-blocking) for minimal EA impact
//!
//! Buffer Management:
//! - Maximum buffer size: 1000 entries (oldest entries dropped when full)
//! - HTTP timeout: 500ms (fails fast if VictoriaLogs is unavailable)

use chrono::Utc;
use serde::Serialize;
use std::sync::mpsc::{self, Sender};
use std::sync::{LazyLock, Mutex, Once};
use std::thread;

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

/// Maximum buffer size before oldest entries are dropped
const MAX_BUFFER_SIZE: usize = 1000;

/// HTTP timeout for VictoriaLogs requests (fails fast)
const HTTP_TIMEOUT_MS: u64 = 500;

/// HTTP connection timeout
const HTTP_CONNECT_TIMEOUT_MS: u64 = 200;

/// VictoriaLogs configuration
#[derive(Debug, Default, Clone)]
struct VLogsConfig {
    enabled: bool,
    endpoint: String,
    source: String,
}

/// Message sent to background sender thread
struct FlushMessage {
    endpoint: String,
    entries: Vec<LogEntry>,
}

// Global state for configuration and log buffer
static CONFIG: LazyLock<Mutex<VLogsConfig>> = LazyLock::new(|| Mutex::new(VLogsConfig::default()));
static BUFFER: LazyLock<Mutex<Vec<LogEntry>>> = LazyLock::new(|| Mutex::new(Vec::new()));

// Background sender channel (initialized once)
static SENDER: LazyLock<Mutex<Option<Sender<FlushMessage>>>> = LazyLock::new(|| Mutex::new(None));
static SENDER_INIT: Once = Once::new();

// HTTP client with short timeout (used by background thread only)
static CLIENT: LazyLock<reqwest::blocking::Client> = LazyLock::new(|| {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_millis(HTTP_TIMEOUT_MS))
        .connect_timeout(std::time::Duration::from_millis(HTTP_CONNECT_TIMEOUT_MS))
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new())
});

/// Initialize background sender thread (called once)
fn init_background_sender() {
    SENDER_INIT.call_once(|| {
        let (tx, rx) = mpsc::channel::<FlushMessage>();

        // Store sender for use by vlogs_flush()
        if let Ok(mut sender) = SENDER.lock() {
            *sender = Some(tx);
        }

        // Spawn background thread for HTTP sending
        thread::spawn(move || {
            while let Ok(msg) = rx.recv() {
                // Send entries (ignore failures - logs are best-effort)
                let _ = send_entries_internal(&msg.endpoint, &msg.entries);
            }
            // Channel closed when sender is dropped
        });
    });
}

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
///
/// # Safety
/// - `endpoint` must be a valid null-terminated UTF-16 string pointer
/// - `source` must be a valid null-terminated UTF-16 string pointer
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
///
/// # Safety
/// - `level` must be a valid null-terminated UTF-16 string pointer
/// - `category` must be a valid null-terminated UTF-16 string pointer
/// - `message` must be a valid null-terminated UTF-16 string pointer
/// - `context_json` must be a valid null-terminated UTF-16 string pointer (can be empty string)
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
            // Enforce buffer size limit - drop oldest entry if full
            if buffer.len() >= MAX_BUFFER_SIZE {
                buffer.remove(0);
            }
            buffer.push(entry);
            buffer.len() as i32
        }
        Err(e) => {
            eprintln!("vlogs_add_entry: failed to lock buffer: {}", e);
            -1
        }
    }
}

/// Flush buffered log entries to VictoriaLogs (non-blocking)
///
/// Sends all buffered entries to background thread for async HTTP delivery.
/// Returns immediately without waiting for HTTP response.
/// Buffer is cleared on successful handoff to background thread.
///
/// # Returns
/// 1 on success (handoff to background), 0 if disabled or buffer empty, -1 on error
#[no_mangle]
pub extern "C" fn vlogs_flush() -> i32 {
    // Initialize background sender if not done
    init_background_sender();

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

    // Send to background thread (non-blocking)
    let msg = FlushMessage { endpoint, entries };

    match SENDER.lock() {
        Ok(sender_guard) => {
            if let Some(sender) = sender_guard.as_ref() {
                match sender.send(msg) {
                    Ok(_) => 1,
                    Err(e) => {
                        eprintln!("vlogs_flush: failed to send to background: {}", e);
                        // Lost entries - acceptable for best-effort logging
                        0
                    }
                }
            } else {
                eprintln!("vlogs_flush: background sender not initialized");
                0
            }
        }
        Err(_) => -1,
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

/// Internal function to send entries to VictoriaLogs (called by background thread)
fn send_entries_internal(endpoint: &str, entries: &[LogEntry]) -> Result<(), String> {
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
        Ok(mut buffer) => {
            // Enforce buffer size limit - drop oldest entry if full
            if buffer.len() >= MAX_BUFFER_SIZE {
                buffer.remove(0);
            }
            buffer.push(entry);
        }
        Err(e) => {
            let mut buffer = e.into_inner();
            if buffer.len() >= MAX_BUFFER_SIZE {
                buffer.remove(0);
            }
            buffer.push(entry);
        }
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

    // Send to VictoriaLogs (synchronous for tests)
    match send_entries_internal(&endpoint, &entries) {
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

    #[test]
    #[serial]
    fn test_buffer_size_limit() {
        setup();

        configure("http://localhost:9428/insert/jsonline", "test-source");

        // Add more than MAX_BUFFER_SIZE entries
        for i in 0..(MAX_BUFFER_SIZE + 10) {
            add_entry("INFO", "Trade", &format!("Message {}", i), None);
        }

        // Buffer should be capped at MAX_BUFFER_SIZE
        let buffer = BUFFER.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(buffer.len(), MAX_BUFFER_SIZE);

        // Oldest entries should have been dropped - check first message is "Message 10"
        assert_eq!(buffer[0].msg, "Message 10");
        // Last message should be the newest one
        assert_eq!(
            buffer[buffer.len() - 1].msg,
            format!("Message {}", MAX_BUFFER_SIZE + 9)
        );
    }
}
