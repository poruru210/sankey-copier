use crate::adapters::infrastructure::log_buffer::{LogBuffer, LogBufferLayer};
use crate::adapters::outbound::observability::victoria_logs::VictoriaLogsLayer;
use crate::config::LoggingConfig;
use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize logging with log buffer layer, optional file output, and VictoriaLogs
pub fn init(config: &LoggingConfig, log_buffer: LogBuffer, vlogs_layer: Option<VictoriaLogsLayer>) {
    // Default to info level for all modules; can be overridden via RUST_LOG env var
    let env_filter =
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());

    let subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .with(LogBufferLayer::new(log_buffer))
        .with(vlogs_layer);

    // Add file logging layer if enabled in config
    if config.enabled {
        use std::fs;
        use tracing_appender::rolling;

        // Create log directory if it doesn't exist
        if let Err(e) = fs::create_dir_all(&config.directory) {
            eprintln!("Failed to create log directory {}: {}", config.directory, e);
        }

        // Clean up old log files based on retention policy
        cleanup_old_logs(config);

        // Create file appender based on rotation strategy
        let file_appender = match config.rotation.as_str() {
            "hourly" => rolling::hourly(&config.directory, &config.file_prefix),
            "never" => rolling::never(&config.directory, &config.file_prefix),
            _ => rolling::daily(&config.directory, &config.file_prefix), // default to daily
        };

        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

        subscriber
            .with(
                tracing_subscriber::fmt::layer()
                    .with_writer(non_blocking)
                    .with_ansi(false),
            ) // Disable ANSI colors in file output
            .init();

        // Store guard to prevent it from being dropped
        // In a real application, you'd want to keep this alive for the entire program lifetime
        // However, standard locking in tracing_appender usually handles this fine for file rolling
        // But for non_blocking, we must keep the guard.
        // MEMORY LEAK INTENTIONAL: distinct from main.rs where it was just forgotten.
        // Ideally we return the guard, but for simplicity we leak it here as this is a long-running server.
        std::mem::forget(_guard);
    } else {
        subscriber.init();
    }
}

/// Clean up old log files based on retention policy
pub fn cleanup_old_logs(logging_config: &LoggingConfig) {
    use std::fs;
    use std::time::SystemTime;

    // Skip cleanup if both max_files and max_age_days are 0 (unlimited)
    if logging_config.max_files == 0 && logging_config.max_age_days == 0 {
        return;
    }

    let log_dir = std::path::Path::new(&logging_config.directory);
    if !log_dir.exists() {
        return;
    }

    // Read all files in the log directory
    let mut log_files: Vec<_> = match fs::read_dir(log_dir) {
        Ok(entries) => entries
            .filter_map(|entry_res| {
                let entry = entry_res.ok()?;
                let metadata = entry.metadata().ok()?;

                // Ensure it's a file, not a directory
                if !metadata.is_file() {
                    return None;
                }

                let file_name = entry.file_name();
                let name = file_name.to_str()?;

                // Check prefix matches
                if !name.starts_with(&logging_config.file_prefix) {
                    return None;
                }

                let modified = metadata.modified().ok()?;
                Some((entry.path(), modified))
            })
            .collect(),
        Err(e) => {
            eprintln!("Failed to read log directory: {}", e);
            return;
        }
    };

    // Sort by modified time (newest first)
    log_files.sort_by(|a, b| b.1.cmp(&a.1));

    let now = SystemTime::now();
    let max_age_duration = Duration::from_secs((logging_config.max_age_days as u64) * 24 * 60 * 60);
    let mut deleted_count = 0;

    // Delete old files based on retention policy
    for (idx, (path, modified)) in log_files.iter().enumerate() {
        let mut should_delete = false;

        // Check if exceeds max file count
        if logging_config.max_files > 0 && idx >= logging_config.max_files as usize {
            should_delete = true;
        }

        // Check if exceeds max age
        if logging_config.max_age_days > 0 {
            if let Ok(age) = now.duration_since(*modified) {
                if age > max_age_duration {
                    should_delete = true;
                }
            }
        }

        if should_delete {
            match fs::remove_file(path) {
                Ok(_) => {
                    deleted_count += 1;
                    eprintln!("Deleted old log file: {:?}", path);
                }
                Err(e) => {
                    eprintln!("Failed to delete log file {:?}: {}", path, e);
                }
            }
        }
    }

    if deleted_count > 0 {
        eprintln!("Cleaned up {} old log file(s)", deleted_count);
    }
}
