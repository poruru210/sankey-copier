// Build script for Windows tray application
// Embeds icon and version information

#[cfg(windows)]
fn main() {
    // Only build resources on Windows
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winres::WindowsResource::new();

        // Set icon if it exists
        if std::path::Path::new("icon.ico").exists() {
            res.set_icon("icon.ico");
        }

        // Set version info
        res.set("FileVersion", env!("CARGO_PKG_VERSION"));
        res.set("ProductVersion", env!("CARGO_PKG_VERSION"));
        res.set("ProductName", "SANKEY Copier Tray");
        res.set("FileDescription", "SANKEY Copier System Tray Application");
        res.set("CompanyName", "SANKEY Copier Project");
        res.set("LegalCopyright", "Copyright (C) 2024");

        // Compile resources
        if let Err(e) = res.compile() {
            eprintln!("Warning: Failed to compile Windows resources: {}", e);
        }
    }
}

#[cfg(not(windows))]
fn main() {
    // No-op on non-Windows platforms
}
