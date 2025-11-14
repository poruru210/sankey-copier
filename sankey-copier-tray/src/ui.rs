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
    // Get version from EXE file properties at runtime
    let version = get_exe_product_version().unwrap_or_else(|| "Unknown".to_string());

    let message = format!(
        "SANKEY Copier Tray Application\n\n\
         Version: {}\n\n\
         MT4/MT5 Trade Copy System\n\
         Low-latency local communication with remote control\n\n\
         Copyright © 2024 SANKEY Copier Project\n\
         Licensed under MIT License",
        version
    );

    show_info(&message);
}

/// Get ProductVersion from the current EXE file at runtime
#[cfg(target_os = "windows")]
fn get_exe_product_version() -> Option<String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Foundation::MAX_PATH;
    use windows::Win32::Storage::FileSystem::{GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW};
    use windows::Win32::System::LibraryLoader::GetModuleFileNameW;

    unsafe {
        // Get the path to the current executable
        let mut filename: [u16; MAX_PATH as usize] = [0; MAX_PATH as usize];
        let len = GetModuleFileNameW(None, &mut filename);
        if len == 0 {
            return None;
        }

        // Get version info size
        let mut dummy = 0u32;
        let size = GetFileVersionInfoSizeW(
            windows::core::PCWSTR(filename.as_ptr()),
            Some(&mut dummy),
        );
        if size == 0 {
            return None;
        }

        // Allocate buffer and get version info
        let mut buffer = vec![0u8; size as usize];
        if GetFileVersionInfoW(
            windows::core::PCWSTR(filename.as_ptr()),
            0,
            size,
            buffer.as_mut_ptr() as *mut _,
        ).is_err() {
            return None;
        }

        // Query for ProductVersion string
        let query_path: Vec<u16> = OsStr::new("\\StringFileInfo\\040904b0\\ProductVersion")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let mut value_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
        let mut value_len = 0u32;

        let result = VerQueryValueW(
            buffer.as_ptr() as *const _,
            windows::core::PCWSTR(query_path.as_ptr()),
            &mut value_ptr,
            &mut value_len,
        );

        if result.as_bool() == false || value_ptr.is_null() {
            return None;
        }

        // Convert to Rust string
        let value_slice = std::slice::from_raw_parts(
            value_ptr as *const u16,
            value_len as usize,
        );

        String::from_utf16(value_slice)
            .ok()
            .map(|s| s.trim_end_matches('\0').to_string())
    }
}

#[cfg(not(target_os = "windows"))]
fn get_exe_product_version() -> Option<String> {
    None
}

#[cfg(target_os = "windows")]
#[link(name = "user32")]
extern "system" {
    fn MessageBoxW(hwnd: *mut std::ffi::c_void, text: *const u16, caption: *const u16, flags: u32) -> i32;
}
