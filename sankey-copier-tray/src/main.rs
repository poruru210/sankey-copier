//! SANKEY Copier System Tray Application
//!
//! This application provides a system tray interface for controlling
//! Windows services for SANKEY Copier, including the server and web UI.

#![windows_subsystem = "windows"] // Hide console window

use anyhow::Result;
use tray_icon::TrayIconBuilder;
use winit::event_loop::{ControlFlow, EventLoop};

mod config;
mod elevation;
mod icon;
mod menu;
mod service;
mod ui;

fn main() -> Result<()> {
    // Create event loop for Windows message pump
    let event_loop: EventLoop<menu::AppEvent> = EventLoop::with_user_event()
        .build()
        .expect("Failed to create event loop");

    let event_loop_proxy = event_loop.create_proxy();

    // Create tray icon
    let _tray_icon = create_tray_icon()?;

    // Run event loop
    event_loop
        .run(move |event, elwt| {
            elwt.set_control_flow(ControlFlow::Poll);

            // Handle user events (like exit)
            match event {
                winit::event::Event::UserEvent(menu::AppEvent::Exit) => {
                    elwt.exit();
                    return;
                }
                _ => {}
            }

            // Check for menu events
            menu::check_menu_events(&event_loop_proxy);
        })
        .expect("Event loop error");

    Ok(())
}

/// Create system tray icon with menu
fn create_tray_icon() -> Result<tray_icon::TrayIcon> {
    // Load icon
    let icon = icon::load_icon()?;

    // Create menu
    let menu = menu::create_menu()?;

    // Build tray icon
    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("SANKEY Copier")
        .with_icon(icon)
        .with_menu_on_left_click(true) // Show menu on left click as well as right click
        .build()?;

    Ok(tray_icon)
}
