//! Windows service management for SANKEY Copier.
//!
//! This module provides functions to start, stop, and restart
//! the rust-server Windows service using NSSM.

use anyhow::Result;
use std::path::PathBuf;

use crate::elevation::run_elevated_batch_command;

pub const SERVER_SERVICE: &str = "SankeyCopierServer";

/// Get the path to nssm.exe
///
/// Searches for nssm.exe in the following order:
/// 1. Same directory as the executable (recommended)
/// 2. Default installation path
///
/// Returns the first valid path found, or None if not found.
fn get_nssm_path() -> Option<PathBuf> {
    // Try to find nssm.exe in the same directory as the executable
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let nssm_path = exe_dir.join("nssm.exe");
            if nssm_path.exists() {
                return Some(nssm_path);
            }
        }
    }

    // Fallback to default installation path
    let default_path = PathBuf::from("C:\\Program Files\\SANKEY Copier\\nssm.exe");
    if default_path.exists() {
        return Some(default_path);
    }

    None
}

// ============================================================================
// Server Service Control
// ============================================================================

/// Start Server service
pub fn start_server_service() -> Result<()> {
    run_elevated_nssm_command("start", &[SERVER_SERVICE])
}

/// Stop Server service
pub fn stop_server_service() -> Result<()> {
    run_elevated_nssm_command("stop", &[SERVER_SERVICE])
}

/// Restart Server service
pub fn restart_server_service() -> Result<()> {
    run_elevated_nssm_command("restart", &[SERVER_SERVICE])
}

// ============================================================================
// Internal Helpers
// ============================================================================

/// Run NSSM command with UAC elevation
fn run_elevated_nssm_command(action: &str, services: &[&str]) -> Result<()> {
    // Get NSSM path
    let nssm_path = get_nssm_path()
        .ok_or_else(|| anyhow::anyhow!("NSSM not found. Please ensure nssm.exe is in the same directory as the tray application or at C:\\Program Files\\SANKEY Copier\\"))?;

    // Build parameters for NSSM
    // For multiple services, we need to call nssm multiple times
    let mut commands = Vec::new();
    for service in services {
        commands.push(format!("\"{}\" {} {}", nssm_path.display(), action, service));
    }
    let command_string = commands.join(" && timeout /t 1 /nobreak >nul && ");

    // Execute via elevated cmd.exe (NSSM requires this for service control)
    run_elevated_batch_command(&command_string)
}
