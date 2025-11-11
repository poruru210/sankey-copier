//! User interface utilities for showing message boxes.
//!
//! This module provides platform-specific message box implementations.

/// Show error message box
pub fn show_error(message: &str) {
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
pub fn show_info(message: &str) {
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
pub fn show_about() {
    // Use BUILD_INFO which matches the ProductVersion in Windows properties
    const VERSION: &str = env!("BUILD_INFO");

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
