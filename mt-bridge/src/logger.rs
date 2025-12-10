use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;

// Global mutex to safeguard file writing
static LOGGER_LOCK: Mutex<()> = Mutex::new(());

/// Log a message to "mt-bridge.log" in the CWD (which is typically Terminal data folder for MT5)
pub fn log_to_file(msg: &str) {
    let _guard = LOGGER_LOCK.lock().unwrap();
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("mt-bridge.log")
    {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let _ = writeln!(file, "[{}] {}", timestamp, msg);
    }
}
