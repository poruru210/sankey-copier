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

        // First, query the translation table to get the language/codepage
        let translation_query: Vec<u16> = OsStr::new("\\VarFileInfo\\Translation")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let mut trans_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
        let mut trans_len = 0u32;

        let trans_result = VerQueryValueW(
            buffer.as_ptr() as *const _,
            windows::core::PCWSTR(translation_query.as_ptr()),
            &mut trans_ptr,
            &mut trans_len,
        );

        // If we can't get translation info, try common codepages
        let codepages = if trans_result.as_bool() && !trans_ptr.is_null() && trans_len >= 4 {
            // Translation table is an array of DWORD values (language_id << 16 | codepage)
            let trans_data = std::slice::from_raw_parts(trans_ptr as *const u32, (trans_len / 4) as usize);
            trans_data.iter().map(|&lang_cp| {
                let lang = (lang_cp & 0xFFFF) as u16;
                let cp = (lang_cp >> 16) as u16;
                format!("{:04x}{:04x}", lang, cp)
            }).collect::<Vec<_>>()
        } else {
            // Fallback to common codepages
            vec!["040904b0".to_string(), "000004b0".to_string(), "040904e4".to_string()]
        };

        // Try each codepage to get ProductVersion
        for codepage in &codepages {
            let query_path = format!("\\StringFileInfo\\{}\\ProductVersion", codepage);
            let query_path_wide: Vec<u16> = OsStr::new(&query_path)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            let mut value_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
            let mut value_len = 0u32;

            let result = VerQueryValueW(
                buffer.as_ptr() as *const _,
                windows::core::PCWSTR(query_path_wide.as_ptr()),
                &mut value_ptr,
                &mut value_len,
            );

            if result.as_bool() && !value_ptr.is_null() && value_len > 0 {
                // Convert to Rust string
                let value_slice = std::slice::from_raw_parts(
                    value_ptr as *const u16,
                    value_len as usize,
                );

                if let Ok(version_str) = String::from_utf16(value_slice) {
                    let trimmed = version_str.trim_end_matches('\0').to_string();
                    if !trimmed.is_empty() {
                        return Some(trimmed);
                    }
                }
            }
        }

        None
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
