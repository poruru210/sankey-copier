//! Windows service management for SANKEY Copier.
//!
//! This module provides functions to start, stop, restart, and query
//! the status of the relay-server Windows service.

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Command;

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
// Service Status
// ============================================================================

/// Query status of a single service using NSSM (safe version that never fails)
///
/// This is a wrapper around query_service_status that returns "Unknown" on error
/// instead of propagating the error. Useful for non-critical status checks.
pub fn query_service_status_safe(service_name: &str) -> String {
    query_service_status(service_name).unwrap_or_else(|_| "Unknown".to_string())
}

/// Query status of a single service using NSSM
fn query_service_status(service_name: &str) -> Result<String> {
    // Try NSSM first if available
    if let Some(nssm_path) = get_nssm_path() {
        let output = Command::new(&nssm_path)
            .args(&["status", service_name])
            .output()
            .context("Failed to execute nssm command")?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let status = stdout.trim();

            // NSSM returns: SERVICE_RUNNING, SERVICE_STOPPED, SERVICE_START_PENDING, etc.
            return Ok(match status {
                "SERVICE_RUNNING" => "Running",
                "SERVICE_STOPPED" => "Stopped",
                "SERVICE_START_PENDING" => "Starting...",
                "SERVICE_STOP_PENDING" => "Stopping...",
                "SERVICE_PAUSE_PENDING" => "Pausing...",
                "SERVICE_CONTINUE_PENDING" => "Resuming...",
                "SERVICE_PAUSED" => "Paused",
                _ => status,
            }
            .to_string());
        }
    }

    // Fallback to sc.exe if NSSM is not available
    let output = Command::new("sc")
        .args(&["query", service_name])
        .output()
        .context("Failed to execute sc command")?;

    if !output.status.success() {
        return Ok("Not Installed".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse state from output
    for line in stdout.lines() {
        if line.contains("STATE") {
            if line.contains("RUNNING") {
                return Ok("Running".to_string());
            } else if line.contains("STOPPED") {
                return Ok("Stopped".to_string());
            } else if line.contains("START_PENDING") {
                return Ok("Starting...".to_string());
            } else if line.contains("STOP_PENDING") {
                return Ok("Stopping...".to_string());
            }
        }
    }

    Ok("Unknown".to_string())
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
