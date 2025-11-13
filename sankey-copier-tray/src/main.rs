//! SANKEY Copier System Tray Application
//!
//! This application provides a system tray interface for controlling
//! Windows services for SANKEY Copier, including the server and web UI.

#![windows_subsystem = "windows"] // Hide console window

use anyhow::Result;
use tray_icon::TrayIconBuilder;
use winit::event_loop::EventLoop;
use winit::application::ApplicationHandler;

mod config;
mod elevation;
mod icon;
mod menu;
mod service;
mod ui;

struct App {
    event_loop_proxy: winit::event_loop::EventLoopProxy<menu::AppEvent>,
}

impl ApplicationHandler<menu::AppEvent> for App {
    fn resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {}

    fn user_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        event: menu::AppEvent,
    ) {
        match event {
            menu::AppEvent::Exit => {
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        _event: winit::event::WindowEvent,
    ) {
        // Check for menu events
        menu::check_menu_events(&self.event_loop_proxy);
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        // Check for menu events periodically
        menu::check_menu_events(&self.event_loop_proxy);
    }
}

fn main() -> Result<()> {
    // Create event loop for Windows message pump
    let event_loop: EventLoop<menu::AppEvent> = EventLoop::with_user_event()
        .build()
        .expect("Failed to create event loop");

    let event_loop_proxy = event_loop.create_proxy();

    // Create tray icon
    let _tray_icon = create_tray_icon()?;

    // Create app handler
    let mut app = App {
        event_loop_proxy: event_loop_proxy.clone(),
    };

    // Run event loop using run_app
    event_loop
        .run_app(&mut app)
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
