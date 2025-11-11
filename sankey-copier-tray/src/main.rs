// SANKEY Copier System Tray Application
// Controls Windows services for SANKEY Copier

#![windows_subsystem = "windows"] // Hide console window

use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::process::Command;
use std::thread;
use std::time::Duration;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, MenuId, PredefinedMenuItem, Submenu},
    Icon, TrayIconBuilder,
};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use winit::event_loop::{ControlFlow, EventLoop, EventLoopProxy};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;
use windows::Win32::Foundation::HWND;
use windows::core::{PCWSTR, w};

const SERVER_SERVICE: &str = "SankeyCopierServer";
const WEBUI_SERVICE: &str = "SankeyCopierWebUI";
const DEFAULT_PORT: u16 = 8080;
const NSSM_PATH: &str = "C:\\Program Files\\SANKEY Copier\\nssm.exe";

#[derive(Debug, Deserialize)]
struct Config {
    #[serde(default)]
    server: ServerConfig,
    #[serde(default)]
    webui: WebUIConfig,
}

#[derive(Debug, Deserialize)]
struct ServerConfig {
    #[serde(default = "default_server_port")]
    port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self { port: 8080 }
    }
}

#[derive(Debug, Deserialize)]
struct WebUIConfig {
    #[serde(default = "default_webui_port")]
    port: u16,
}

impl Default for WebUIConfig {
    fn default() -> Self {
        Self { port: 3000 }
    }
}

// Custom event for app control
#[derive(Debug, Clone)]
enum AppEvent {
    Exit,
}

// Global menu ID map
static MENU_IDS: once_cell::sync::Lazy<Arc<Mutex<HashMap<MenuId, String>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

// Global web URL
static WEB_URL: once_cell::sync::Lazy<String> = once_cell::sync::Lazy::new(|| {
    let port = load_port_from_config().unwrap_or(8080);  // Default to WebUI port (8080)
    // Use 127.0.0.1 instead of localhost to force IPv4
    format!("http://127.0.0.1:{}", port)
});

fn default_server_port() -> u16 {
    8080
}

fn default_webui_port() -> u16 {
    3000
}

/// Load Web UI port number from config.toml
fn load_port_from_config() -> Option<u16> {
    // Try to find config.toml in standard locations
    let config_paths = [
        "config.toml",
        "../config.toml",
        "C:\\Program Files\\SANKEY Copier\\config.toml",
    ];

    for path in &config_paths {
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(config) = toml::from_str::<Config>(&content) {
                // Return webui.port, not server.port
                return Some(config.webui.port);
            }
        }
    }

    None
}

fn main() -> Result<()> {
    // Create event loop for Windows message pump
    let event_loop: EventLoop<AppEvent> = EventLoop::with_user_event()
        .build()
        .expect("Failed to create event loop");

    let event_loop_proxy = event_loop.create_proxy();

    // Create tray icon
    let _tray_icon = create_tray_icon()?;

    // Monitor service status and update tray icon
    let _status_thread = thread::spawn(|| {
        loop {
            thread::sleep(Duration::from_secs(5));
            // TODO: Update icon based on service status
        }
    });

    // Run event loop
    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Poll);

        // Handle user events (like exit)
        match event {
            winit::event::Event::UserEvent(AppEvent::Exit) => {
                elwt.exit();
                return;
            }
            _ => {}
        }

        // Check for menu events
        if let Ok(menu_event) = MenuEvent::receiver().try_recv() {
            handle_menu_event(&menu_event.id, &event_loop_proxy);
        }
    }).expect("Event loop error");

    Ok(())
}

/// Create system tray icon with menu
fn create_tray_icon() -> Result<tray_icon::TrayIcon> {
    // Load icon
    let icon = load_icon()?;

    // Create menu
    let menu = create_menu()?;

    // Build tray icon
    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("SANKEY Copier")
        .with_icon(icon)
        .with_menu_on_left_click(true)  // Show menu on left click as well as right click
        .build()?;

    Ok(tray_icon)
}

/// Create tray menu
fn create_menu() -> Result<Menu> {
    let menu = Menu::new();
    let mut ids = MENU_IDS.lock().unwrap();

    // Status item (non-clickable)
    let status_item = MenuItem::new("SANKEY Copier", false, None);
    menu.append(&status_item)?;

    // Separator
    menu.append(&PredefinedMenuItem::separator())?;

    // UI Submenu (includes Open Web Interface and service controls for UI)
    let ui_open_item = MenuItem::new("Open", true, None);
    ids.insert(ui_open_item.id().clone(), "ui_open".to_string());

    let ui_start_item = MenuItem::new("Start", true, None);
    ids.insert(ui_start_item.id().clone(), "ui_start".to_string());

    let ui_stop_item = MenuItem::new("Stop", true, None);
    ids.insert(ui_stop_item.id().clone(), "ui_stop".to_string());

    let ui_restart_item = MenuItem::new("Restart", true, None);
    ids.insert(ui_restart_item.id().clone(), "ui_restart".to_string());

    let ui_submenu = Submenu::with_items("UI", true, &[
        &ui_open_item,
        &ui_start_item,
        &ui_stop_item,
        &ui_restart_item,
    ])?;
    menu.append(&ui_submenu)?;

    // Service Submenu
    let service_start_item = MenuItem::new("Start", true, None);
    ids.insert(service_start_item.id().clone(), "service_start".to_string());

    let service_stop_item = MenuItem::new("Stop", true, None);
    ids.insert(service_stop_item.id().clone(), "service_stop".to_string());

    let service_restart_item = MenuItem::new("Restart", true, None);
    ids.insert(service_restart_item.id().clone(), "service_restart".to_string());

    let service_submenu = Submenu::with_items("Service", true, &[
        &service_start_item,
        &service_stop_item,
        &service_restart_item,
    ])?;
    menu.append(&service_submenu)?;

    // Separator
    menu.append(&PredefinedMenuItem::separator())?;

    // Check Status
    let status_check_item = MenuItem::new("Check Status", true, None);
    ids.insert(status_check_item.id().clone(), "status".to_string());
    menu.append(&status_check_item)?;

    // Separator
    menu.append(&PredefinedMenuItem::separator())?;

    // About
    let about_item = MenuItem::new("About", true, None);
    ids.insert(about_item.id().clone(), "about".to_string());
    menu.append(&about_item)?;

    // Exit - use MenuItem instead of PredefinedMenuItem for better control
    let quit_item = MenuItem::new("Exit", true, None);
    ids.insert(quit_item.id().clone(), "quit".to_string());
    menu.append(&quit_item)?;

    Ok(menu)
}

/// Handle menu events
fn handle_menu_event(id: &MenuId, event_loop_proxy: &EventLoopProxy<AppEvent>) {
    let ids = MENU_IDS.lock().unwrap();
    let action = ids.get(id).map(|s| s.as_str()).unwrap_or("");

    match action {
        // UI submenu actions
        "ui_open" => {
            if let Err(e) = open_web_interface() {
                show_error(&format!("Failed to open web interface: {}", e));
            }
        }

        "ui_start" => {
            if let Err(e) = start_webui_service() {
                show_error(&format!("Failed to start Web UI: {}", e));
            } else {
                show_info("Web UI started successfully");
            }
        }

        "ui_stop" => {
            if let Err(e) = stop_webui_service() {
                show_error(&format!("Failed to stop Web UI: {}", e));
            } else {
                show_info("Web UI stopped successfully");
            }
        }

        "ui_restart" => {
            if let Err(e) = restart_webui_service() {
                show_error(&format!("Failed to restart Web UI: {}", e));
            } else {
                show_info("Web UI restarted successfully");
            }
        }

        // Service submenu actions
        "service_start" => {
            if let Err(e) = start_server_service() {
                show_error(&format!("Failed to start Server: {}", e));
            } else {
                show_info("Server started successfully");
            }
        }

        "service_stop" => {
            if let Err(e) = stop_server_service() {
                show_error(&format!("Failed to stop Server: {}", e));
            } else {
                show_info("Server stopped successfully");
            }
        }

        "service_restart" => {
            if let Err(e) = restart_server_service() {
                show_error(&format!("Failed to restart Server: {}", e));
            } else {
                show_info("Server restarted successfully");
            }
        }

        // Check status
        "status" => {
            match get_service_status() {
                Ok(status) => show_info(&status),
                Err(e) => show_error(&format!("Failed to get status: {}", e)),
            }
        }

        // About
        "about" => {
            show_about();
        }

        // Quit
        "quit" => {
            let _ = event_loop_proxy.send_event(AppEvent::Exit);
        }

        _ => {}
    }
}

/// Load application icon
fn load_icon() -> Result<Icon> {
    // Try to load embedded icon first
    const ICON_DATA: &[u8] = include_bytes!("../icon.ico");

    // Parse ICO file to get the best quality image
    if let Ok(image) = image::load_from_memory(ICON_DATA) {
        let rgba = image.to_rgba8();
        let (width, height) = rgba.dimensions();
        let icon_data = rgba.into_raw();

        return Icon::from_rgba(icon_data, width, height)
            .context("Failed to create icon from embedded image");
    }

    // Fall back to loading from file system
    let icon_path = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.join("icon.ico")));

    if let Some(path) = icon_path {
        if path.exists() {
            if let Ok(image) = image::open(&path) {
                let rgba = image.to_rgba8();
                let (width, height) = rgba.dimensions();
                let icon_data = rgba.into_raw();

                return Icon::from_rgba(icon_data, width, height)
                    .context("Failed to create icon from file");
            }
        }
    }

    // Final fallback to simple colored icon
    create_default_icon()
}

/// Create a simple default icon
fn create_default_icon() -> Result<Icon> {
    const SIZE: u32 = 32;
    let mut rgba = vec![0u8; (SIZE * SIZE * 4) as usize];

    // Create a simple blue square icon
    for y in 0..SIZE {
        for x in 0..SIZE {
            let idx = ((y * SIZE + x) * 4) as usize;
            // Blue color with alpha
            rgba[idx] = 50;      // R
            rgba[idx + 1] = 100; // G
            rgba[idx + 2] = 200; // B
            rgba[idx + 3] = 255; // A
        }
    }

    Icon::from_rgba(rgba, SIZE, SIZE).context("Failed to create default icon")
}

/// Open web interface in default browser
fn open_web_interface() -> Result<()> {
    webbrowser::open(&*WEB_URL).context("Failed to open browser")
}

/// Start Web UI service
fn start_webui_service() -> Result<()> {
    run_elevated_nssm_command("start", &[WEBUI_SERVICE])
}

/// Stop Web UI service
fn stop_webui_service() -> Result<()> {
    run_elevated_nssm_command("stop", &[WEBUI_SERVICE])
}

/// Restart Web UI service
fn restart_webui_service() -> Result<()> {
    run_elevated_nssm_command("restart", &[WEBUI_SERVICE])
}

/// Start Server service
fn start_server_service() -> Result<()> {
    run_elevated_nssm_command("start", &[SERVER_SERVICE])
}

/// Stop Server service
fn stop_server_service() -> Result<()> {
    run_elevated_nssm_command("stop", &[SERVER_SERVICE])
}

/// Restart Server service
fn restart_server_service() -> Result<()> {
    run_elevated_nssm_command("restart", &[SERVER_SERVICE])
}

/// Run NSSM command with UAC elevation
fn run_elevated_nssm_command(action: &str, services: &[&str]) -> Result<()> {
    // Check if NSSM exists
    if !std::path::Path::new(NSSM_PATH).exists() {
        anyhow::bail!("NSSM not found at: {}", NSSM_PATH);
    }

    // Build parameters for NSSM
    // For multiple services, we need to call nssm multiple times
    let mut commands = Vec::new();
    for service in services {
        commands.push(format!("\"{}\" {} {}", NSSM_PATH, action, service));
    }
    let command_string = commands.join(" && timeout /t 1 /nobreak >nul && ");

    // Execute via elevated cmd.exe (NSSM requires this for service control)
    run_elevated_batch_command(&command_string)
}

/// Run batch command with UAC elevation
fn run_elevated_batch_command(commands: &str) -> Result<()> {
    // Create a temporary batch file to run the commands
    let temp_dir = std::env::temp_dir();
    let batch_file = temp_dir.join(format!("sankey_copier_{}.bat",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()));

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

/// Old single-service functions (now unused)
#[allow(dead_code)]
fn start_service(service_name: &str) -> Result<()> {
    run_elevated_batch_command(&format!("sc start {}", service_name))
}

#[allow(dead_code)]
fn stop_service(service_name: &str) -> Result<()> {
    run_elevated_batch_command(&format!("sc stop {}", service_name))
}

/// Old implementation for reference (now unused)
#[allow(dead_code)]
fn run_sc_command_direct(operation: &str, service_name: &str) -> Result<()> {
    let output = Command::new("sc")
        .args(&[operation, service_name])
        .output()
        .context("Failed to execute sc command")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Ignore "service not running" errors
        if !stderr.contains("1062") && !stderr.contains("stopped") {
            anyhow::bail!("Service stop failed: {}", stderr);
        }
    }

    Ok(())
}

/// Get service status
fn get_service_status() -> Result<String> {
    let server_status = query_service_status(SERVER_SERVICE)?;
    let webui_status = query_service_status(WEBUI_SERVICE)?;

    Ok(format!(
        "Service Status:\n\nServer: {}\nWeb UI: {}",
        server_status, webui_status
    ))
}

/// Query status of a single service using NSSM
fn query_service_status(service_name: &str) -> Result<String> {
    // Try NSSM first if available
    if std::path::Path::new(NSSM_PATH).exists() {
        let output = Command::new(NSSM_PATH)
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
            }.to_string());
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

/// Show error message box
fn show_error(message: &str) {
    #[cfg(target_os = "windows")]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use std::ptr;

        let wide_title: Vec<u16> = OsStr::new("SANKEY Copier - Error")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let wide_message: Vec<u16> = OsStr::new(message)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            MessageBoxW(
                ptr::null_mut(),
                wide_message.as_ptr(),
                wide_title.as_ptr(),
                0x00000010, // MB_ICONERROR
            );
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        eprintln!("Error: {}", message);
    }
}

/// Show info message box
fn show_info(message: &str) {
    #[cfg(target_os = "windows")]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use std::ptr;

        let wide_title: Vec<u16> = OsStr::new("SANKEY Copier")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let wide_message: Vec<u16> = OsStr::new(message)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            MessageBoxW(
                ptr::null_mut(),
                wide_message.as_ptr(),
                wide_title.as_ptr(),
                0x00000040, // MB_ICONINFORMATION
            );
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        println!("Info: {}", message);
    }
}

/// Show about dialog
fn show_about() {
    const VERSION: &str = env!("CARGO_PKG_VERSION");

    let message = format!(
        "SANKEY Copier Tray Application\n\n\
         Version: {}\n\n\
         MT4/MT5 Trade Copy System\n\
         Low-latency local communication with remote control\n\n\
         Copyright © 2024 SANKEY Copier Project\n\
         Licensed under MIT License",
        VERSION
    );

    show_info(&message);
}

#[cfg(target_os = "windows")]
#[link(name = "user32")]
extern "system" {
    fn MessageBoxW(hwnd: *mut std::ffi::c_void, text: *const u16, caption: *const u16, flags: u32) -> i32;
}
