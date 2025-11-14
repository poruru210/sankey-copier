//! SANKEY Copier System Tray Application
//!
//! This application provides a system tray interface for controlling
//! the rust-server Windows service and launching the Desktop App.

#![windows_subsystem = "windows"] // Hide console window

use anyhow::Result;
use std::time::{Duration, Instant};
use tray_icon::TrayIconBuilder;
use winit::event_loop::EventLoop;
use winit::application::ApplicationHandler;
use windows::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS};
use windows::Win32::System::Threading::CreateMutexW;
use windows::core::PCWSTR;

mod elevation;
mod icon;
mod menu;
mod service;
mod ui;

struct App {
    event_loop_proxy: winit::event_loop::EventLoopProxy<menu::AppEvent>,
    tray_icon: tray_icon::TrayIcon,
    last_status_check: Instant,
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

        // Update service status every 500ms by rebuilding the menu
        if self.last_status_check.elapsed() >= Duration::from_millis(500) {
            if let Ok(new_menu) = menu::create_menu() {
                self.tray_icon.set_menu(Some(Box::new(new_menu)));
            }
            self.last_status_check = Instant::now();
        }
    }
}

fn main() -> Result<()> {
    // Check for single instance using Windows mutex
    // If another instance is already running, exit silently (no warning needed)
    let mutex_name: Vec<u16> = "Global\\SankeyCopierTrayApp"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let mutex = CreateMutexW(
            None,
            true, // Initially owned by this thread
            PCWSTR(mutex_name.as_ptr()),
        );

        // Check if mutex already exists (another instance is running)
        if let Ok(handle) = mutex {
            let last_error = GetLastError();
            if last_error == ERROR_ALREADY_EXISTS {
                // Another instance is already running, exit silently
                let _ = CloseHandle(handle);
                return Ok(());
            }
            // Mutex will be automatically released when the process exits
            // No need to explicitly close the handle during normal operation
        }
    }

    // Create event loop for Windows message pump
    let event_loop: EventLoop<menu::AppEvent> = EventLoop::with_user_event()
        .build()
        .expect("Failed to create event loop");

    let event_loop_proxy = event_loop.create_proxy();

    // Create tray icon
    let tray_icon = create_tray_icon()?;

    // Create app handler
    let mut app = App {
        event_loop_proxy: event_loop_proxy.clone(),
        tray_icon,
        last_status_check: Instant::now(),
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
