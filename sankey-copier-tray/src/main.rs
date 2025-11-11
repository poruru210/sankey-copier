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
    menu::{Menu, MenuEvent, MenuItem, MenuId, PredefinedMenuItem},
    Icon, TrayIconBuilder,
};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use winit::event_loop::{ControlFlow, EventLoop, EventLoopProxy};
use winit::platform::windows::EventLoopBuilderExtWindows;

const SERVER_SERVICE: &str = "SankeyCopierServer";
const WEBUI_SERVICE: &str = "SankeyCopierWebUI";
const DEFAULT_PORT: u16 = 8080;

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

    // Open Web Interface
    let open_item = MenuItem::new("Open Web Interface", true, None);
    ids.insert(open_item.id().clone(), "open".to_string());
    menu.append(&open_item)?;

    // Separator
    menu.append(&PredefinedMenuItem::separator())?;

    // Service controls
    let start_item = MenuItem::new("Start Services", true, None);
    ids.insert(start_item.id().clone(), "start".to_string());
    menu.append(&start_item)?;

    let stop_item = MenuItem::new("Stop Services", true, None);
    ids.insert(stop_item.id().clone(), "stop".to_string());
    menu.append(&stop_item)?;

    let restart_item = MenuItem::new("Restart Services", true, None);
    ids.insert(restart_item.id().clone(), "restart".to_string());
    menu.append(&restart_item)?;

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
        // Open web interface
        "open" => {
            if let Err(e) = open_web_interface() {
                show_error(&format!("Failed to open web interface: {}", e));
            }
        }

        // Start services
        "start" => {
            if let Err(e) = start_services() {
                show_error(&format!("Failed to start services: {}", e));
            } else {
                show_info("Services started successfully");
            }
        }

        // Stop services
        "stop" => {
            if let Err(e) = stop_services() {
                show_error(&format!("Failed to stop services: {}", e));
            } else {
                show_info("Services stopped successfully");
            }
        }

        // Restart services
        "restart" => {
            if let Err(e) = restart_services() {
                show_error(&format!("Failed to restart services: {}", e));
            } else {
                show_info("Services restarted successfully");
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
    // Try to load icon from file, fall back to embedded icon
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
                    .context("Failed to create icon from image");
            }
        }
    }

    // Fall back to simple colored icon
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

/// Start Windows services
fn start_services() -> Result<()> {
    start_service(SERVER_SERVICE)?;
    thread::sleep(Duration::from_secs(2)); // Wait for server to start
    start_service(WEBUI_SERVICE)?;
    Ok(())
}

/// Stop Windows services
fn stop_services() -> Result<()> {
    stop_service(WEBUI_SERVICE)?;
    thread::sleep(Duration::from_secs(1));
    stop_service(SERVER_SERVICE)?;
    Ok(())
}

/// Restart Windows services
fn restart_services() -> Result<()> {
    stop_services()?;
    thread::sleep(Duration::from_secs(2));
    start_services()?;
    Ok(())
}

/// Start a single Windows service
fn start_service(service_name: &str) -> Result<()> {
    let output = Command::new("sc")
        .args(&["start", service_name])
        .output()
        .context("Failed to execute sc command")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Service start failed: {}", stderr);
    }

    Ok(())
}

/// Stop a single Windows service
fn stop_service(service_name: &str) -> Result<()> {
    let output = Command::new("sc")
        .args(&["stop", service_name])
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

/// Query status of a single service
fn query_service_status(service_name: &str) -> Result<String> {
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
