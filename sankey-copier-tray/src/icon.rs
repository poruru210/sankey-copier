//! Icon loading and management for the tray icon.
//!
//! This module handles loading icons from embedded resources or the file system.

use anyhow::{Context, Result};
use tray_icon::Icon;

/// Load application icon
///
/// Tries to load the icon in the following order:
/// 1. Embedded icon from binary
/// 2. Icon file from application directory
/// 3. Default generated icon
pub fn load_icon() -> Result<Icon> {
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
