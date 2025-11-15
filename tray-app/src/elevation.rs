//! UAC elevation utilities for Windows.
//!
//! This module provides functions to run commands with administrator privileges.

use anyhow::{Context, Result};
use std::fs;
use std::thread;
use std::time::Duration;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;

/// Run batch command with UAC elevation
///
/// Creates a temporary batch file and executes it with administrator privileges
/// using Windows ShellExecuteW with the "runas" verb.
pub fn run_elevated_batch_command(commands: &str) -> Result<()> {
    // Create a temporary batch file to run the commands
    let temp_dir = std::env::temp_dir();
    let batch_file = temp_dir.join(format!(
        "sankey_copier_{}.bat",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    ));

    // Write commands to batch file
    fs::write(&batch_file, format!("@echo off\r\n{}\r\n", commands))
        .context("Failed to create temporary batch file")?;

    // Execute the batch file with elevation
    unsafe {
        let operation_w = w!("runas"); // Request elevation
        let file_w = w!("cmd.exe");
        let params = format!("/c \"{}\"", batch_file.display());
        let params_utf16: Vec<u16> = params.encode_utf16().chain(std::iter::once(0)).collect();
        let params_w = PCWSTR(params_utf16.as_ptr());

        let result = ShellExecuteW(
            HWND(std::ptr::null_mut()),
            operation_w,
            file_w,
            params_w,
            PCWSTR::null(),
            SW_HIDE,
        );

        // ShellExecute returns a value > 32 on success
        if result.0 as i32 <= 32 {
            let _ = fs::remove_file(&batch_file);
            anyhow::bail!("Failed to elevate command");
        }
    }

    // Wait for the command to execute
    thread::sleep(Duration::from_millis(2000));

    // Clean up temporary batch file
    let _ = fs::remove_file(&batch_file);

    Ok(())
}
