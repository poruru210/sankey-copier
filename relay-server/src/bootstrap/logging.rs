use crate::adapters::infrastructure::log_buffer::{create_log_buffer, LogBuffer};
use crate::adapters::outbound::observability::victoria_logs;
use crate::logging;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub fn setup(config: &crate::config::Config) -> (LogBuffer, Option<Arc<AtomicBool>>) {
    // Create log buffer
    let log_buffer = create_log_buffer();

    // Initialize VictoriaLogs
    let (vlogs_layer, _vlogs_handles, vlogs_enabled_flag) =
        victoria_logs::init(&config.victoria_logs);

    // Initialize logging
    // logging::init consumes vlogs_layer, so we might need to adjust or just pass it through
    // Wait, logging::init takes vlogs_layer and adds it to registry.
    // But we might need vlogs_layer for VLogsController?
    // No, VLogsController needs vlogs_enabled_flag and config.
    // logging::init performs the global subscriber set.

    // Check if we need to return vlogs_layer or if init consumes it?
    // logging::init signature: pub fn init<L>(config: &LoggingConfig, log_buffer: LogBuffer, vlogs_layer: Option<L>)
    // checks: d:\projects\sankey-copier2\relay-server\src\logging.rs (I assume)

    // Let's assume we call logging::init here.
    logging::init(&config.logging, log_buffer.clone(), vlogs_layer);

    tracing::info!("Starting SANKEY Copier Server...");
    tracing::info!("Server Version: {}", env!("BUILD_INFO"));
    tracing::info!("Loaded configuration from config.toml");

    if config.logging.enabled {
        tracing::info!(
            "File logging enabled: directory={}, prefix={}, rotation={}",
            config.logging.directory,
            config.logging.file_prefix,
            config.logging.rotation
        );
    }

    if let Some(enabled) = &vlogs_enabled_flag {
        tracing::info!(
            "VictoriaLogs configured: host={}, batch_size={}, flush_interval={}s, initial_enabled={}",
            config.victoria_logs.host,
            config.victoria_logs.batch_size,
            config.victoria_logs.flush_interval_secs,
            enabled.load(std::sync::atomic::Ordering::Relaxed)
        );
    }

    // We don't really need to return vlogs_layer since it's already installed globally.
    // But we return None for the 3rd tuple element to match signature if needed, or change signature.
    // Changing signature to: (LogBuffer, Option<Arc<AtomicBool>>, ())

    (log_buffer, vlogs_enabled_flag)
}
