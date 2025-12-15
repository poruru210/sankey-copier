use crate::config::LoggingConfig;
use std::time::Duration;

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
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                // Only consider files that start with the configured prefix
                entry
                    .file_name()
                    .to_str()
                    .map(|name| name.starts_with(&logging_config.file_prefix))
                    .unwrap_or(false)
            })
            .filter_map(|entry| {
                // Get file metadata and modified time
                let metadata = entry.metadata().ok()?;
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
