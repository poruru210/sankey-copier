//! Tray menu creation and event handling.
//!
//! This module manages the system tray menu structure and handles user interactions.

use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu};
use winit::event_loop::EventLoopProxy;

use crate::config;
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

/// Create tray menu with all items
pub fn create_menu() -> Result<Menu> {
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

    let ui_submenu = Submenu::with_items(
        "UI",
        true,
        &[&ui_open_item, &ui_start_item, &ui_stop_item, &ui_restart_item],
    )?;
    menu.append(&ui_submenu)?;

    // Service Submenu
    let service_start_item = MenuItem::new("Start", true, None);
    ids.insert(service_start_item.id().clone(), "service_start".to_string());

    let service_stop_item = MenuItem::new("Stop", true, None);
    ids.insert(service_stop_item.id().clone(), "service_stop".to_string());

    let service_restart_item = MenuItem::new("Restart", true, None);
    ids.insert(
        service_restart_item.id().clone(),
        "service_restart".to_string(),
    );

    let service_submenu = Submenu::with_items(
        "Service",
        true,
        &[&service_start_item, &service_stop_item, &service_restart_item],
    )?;
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
pub fn handle_menu_event(id: &MenuId, event_loop_proxy: &EventLoopProxy<AppEvent>) {
    let ids = MENU_IDS.lock().unwrap();
    let action = ids.get(id).map(|s| s.as_str()).unwrap_or("");

    match action {
        // UI submenu actions
        "ui_open" => {
            if let Err(e) = open_web_interface() {
                ui::show_error(&format!("Failed to open web interface: {}", e));
            }
        }

        "ui_start" => {
            if let Err(e) = service::start_webui_service() {
                ui::show_error(&format!("Failed to start Web UI: {}", e));
            } else {
                ui::show_info("Web UI started successfully");
            }
        }

        "ui_stop" => {
            if let Err(e) = service::stop_webui_service() {
                ui::show_error(&format!("Failed to stop Web UI: {}", e));
            } else {
                ui::show_info("Web UI stopped successfully");
            }
        }

        "ui_restart" => {
            if let Err(e) = service::restart_webui_service() {
                ui::show_error(&format!("Failed to restart Web UI: {}", e));
            } else {
                ui::show_info("Web UI restarted successfully");
            }
        }

        // Service submenu actions
        "service_start" => {
            if let Err(e) = service::start_server_service() {
                ui::show_error(&format!("Failed to start Server: {}", e));
            } else {
                ui::show_info("Server started successfully");
            }
        }

        "service_stop" => {
            if let Err(e) = service::stop_server_service() {
                ui::show_error(&format!("Failed to stop Server: {}", e));
            } else {
                ui::show_info("Server stopped successfully");
            }
        }

        "service_restart" => {
            if let Err(e) = service::restart_server_service() {
                ui::show_error(&format!("Failed to restart Server: {}", e));
            } else {
                ui::show_info("Server restarted successfully");
            }
        }

        // Check status
        "status" => match service::get_service_status() {
            Ok(status) => ui::show_info(&status),
            Err(e) => ui::show_error(&format!("Failed to get status: {}", e)),
        },

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

/// Open web interface in default browser
fn open_web_interface() -> Result<()> {
    let web_url = config::get_web_url();
    webbrowser::open(&web_url).map_err(|e| anyhow::anyhow!("Failed to open browser: {}", e))
}

/// Check for menu events and handle them
pub fn check_menu_events(event_loop_proxy: &EventLoopProxy<AppEvent>) {
    if let Ok(menu_event) = MenuEvent::receiver().try_recv() {
        handle_menu_event(&menu_event.id, event_loop_proxy);
    }
}
