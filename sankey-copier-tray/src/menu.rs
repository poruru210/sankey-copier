//! Tray menu creation and event handling.
//!
//! This module manages the system tray menu structure and handles user interactions.

use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu};
use winit::event_loop::EventLoopProxy;

use crate::service;
use crate::ui;

/// Global menu ID map to track menu items
static MENU_IDS: once_cell::sync::Lazy<Arc<Mutex<HashMap<MenuId, String>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

/// Custom event for app control
#[derive(Debug, Clone)]
pub enum AppEvent {
    Exit,
}

/// Get status indicator symbol
fn get_status_indicator(status: &str) -> &str {
    match status {
        "Running" => "🟢",      // Running (green circle)
        "Stopped" => "🔴",      // Stopped (red circle)
        "Starting..." => "🟡",  // Starting (yellow circle)
        "Stopping..." => "🟡",  // Stopping (yellow circle)
        _ => "⚪",              // Unknown (white circle)
    }
}

/// Create tray menu with all items
pub fn create_menu() -> Result<Menu> {
    let menu = Menu::new();
    let mut ids = MENU_IDS.lock().unwrap();

    // Status item (non-clickable)
    let status_item = MenuItem::new("SANKEY Copier", false, None);
    menu.append(&status_item)?;

    // Separator
    menu.append(&PredefinedMenuItem::separator())?;

    // Open Desktop App
    let open_desktop_item = MenuItem::new("Open Desktop App", true, None);
    ids.insert(open_desktop_item.id().clone(), "open_desktop".to_string());
    menu.append(&open_desktop_item)?;

    // Separator
    menu.append(&PredefinedMenuItem::separator())?;

    // Get server status
    let server_status = service::query_service_status_safe(service::SERVER_SERVICE);

    // Server Submenu
    let service_start_item = MenuItem::new("Start", true, None);
    ids.insert(service_start_item.id().clone(), "service_start".to_string());

    let service_stop_item = MenuItem::new("Stop", true, None);
    ids.insert(service_stop_item.id().clone(), "service_stop".to_string());

    let service_restart_item = MenuItem::new("Restart", true, None);
    ids.insert(
        service_restart_item.id().clone(),
        "service_restart".to_string(),
    );

    let service_title = format!("Service {}", get_status_indicator(&server_status));
    let service_submenu = Submenu::with_items(
        &service_title,
        true,
        &[&service_start_item, &service_stop_item, &service_restart_item],
    )?;
    menu.append(&service_submenu)?;

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
pub fn handle_menu_event(id: &MenuId, event_loop_proxy: &EventLoopProxy<AppEvent>) {
    let ids = MENU_IDS.lock().unwrap();
    let action = ids.get(id).map(|s| s.as_str()).unwrap_or("");

    match action {
        // Open Desktop App
        "open_desktop" => {
            if let Err(e) = open_desktop_app() {
                ui::show_error(&format!("Failed to open Desktop App: {}", e));
            }
        }

        // Server submenu actions
        "service_start" => {
            if let Err(e) = service::start_server_service() {
                ui::show_error(&format!("Failed to start Server: {}", e));
            }
        }

        "service_stop" => {
            if let Err(e) = service::stop_server_service() {
                ui::show_error(&format!("Failed to stop Server: {}", e));
            }
        }

        "service_restart" => {
            if let Err(e) = service::restart_server_service() {
                ui::show_error(&format!("Failed to restart Server: {}", e));
            }
        }

        // About
        "about" => {
            ui::show_about();
        }

        // Quit
        "quit" => {
            let _ = event_loop_proxy.send_event(AppEvent::Exit);
        }

        _ => {}
    }
}

/// Open Desktop App
fn open_desktop_app() -> Result<()> {
    // Get the path to the Desktop App executable
    let desktop_app_path = get_desktop_app_path()?;

    // Launch the Desktop App
    std::process::Command::new(&desktop_app_path)
        .spawn()
        .map_err(|e| anyhow::anyhow!("Failed to launch Desktop App: {}", e))?;

    Ok(())
}

/// Get the path to the Desktop App executable
fn get_desktop_app_path() -> Result<std::path::PathBuf> {
    // Try to find sankey-copier-desktop.exe in the same directory as the tray app
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let desktop_path = exe_dir.join("sankey-copier-desktop.exe");
            if desktop_path.exists() {
                return Ok(desktop_path);
            }
        }
    }

    // Fallback to default installation path
    let default_path = std::path::PathBuf::from("C:\\Program Files\\SANKEY Copier\\sankey-copier-desktop.exe");
    if default_path.exists() {
        return Ok(default_path);
    }

    Err(anyhow::anyhow!("Desktop App executable not found"))
}

/// Check for menu events and handle them
pub fn check_menu_events(event_loop_proxy: &EventLoopProxy<AppEvent>) {
    if let Ok(menu_event) = MenuEvent::receiver().try_recv() {
        handle_menu_event(&menu_event.id, event_loop_proxy);
    }
}
