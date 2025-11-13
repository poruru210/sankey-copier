// Build script for Windows tray application
// Embeds icon and version information

use std::process::Command;

#[cfg(windows)]
fn main() {
    // Generate version information
    let (package_version, file_version, build_info) = generate_version_info();

    // Set environment variables for use in code
    println!("cargo:rustc-env=PACKAGE_VERSION={}", package_version);
    println!("cargo:rustc-env=FILE_VERSION={}", file_version);
    println!("cargo:rustc-env=BUILD_INFO={}", build_info);

    // Rerun if .git/HEAD changes
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/refs/heads");

    // Only build resources on Windows
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winres::WindowsResource::new();

        // Set icon if it exists
        if std::path::Path::new("icon.ico").exists() {
            res.set_icon("icon.ico");
        }

        // Set string version information (StringFileInfo)
        // These are shown in Windows Explorer properties
        res.set("ProductVersion", &package_version);      // Clean version from Git tag
        res.set("FileVersion", &file_version);            // Windows 4-component version
        res.set("ProductName", "SANKEY Copier Tray");
        res.set("FileDescription", "SANKEY Copier System Tray Application");
        res.set("CompanyName", "SANKEY Copier Project");
        res.set("LegalCopyright", "Copyright (C) 2024");

        // Set numeric version information (FixedFileInfo)
        // This is used for programmatic version comparison
        if let Some(version_u64) = parse_version(&file_version) {
            res.set_version_info(winres::VersionInfo::FILEVERSION, version_u64);
            res.set_version_info(winres::VersionInfo::PRODUCTVERSION, version_u64);
        }

        // Compile resources
        if let Err(e) = res.compile() {
            eprintln!("Warning: Failed to compile Windows resources: {}", e);
        }
    }
}

#[cfg(not(windows))]
fn main() {
    // Generate version information (non-Windows platforms)
    let (package_version, file_version, build_info) = generate_version_info();

    println!("cargo:rustc-env=PACKAGE_VERSION={}", package_version);
    println!("cargo:rustc-env=FILE_VERSION={}", file_version);
    println!("cargo:rustc-env=BUILD_INFO={}", build_info);

    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/refs/heads");
}

fn generate_version_info() -> (String, String, String) {
    // Check if version information is provided via environment variables (from CI/CD)
    if let (Ok(pkg_ver), Ok(file_ver)) = (std::env::var("PACKAGE_VERSION"), std::env::var("FILE_VERSION")) {
        // Use versions from environment variables
        let build_info = format!("{}+ci", file_ver);
        return (pkg_ver, file_ver, build_info);
    }

    // Fallback: Generate from Git information
    // 1. Get base version from Git tag
    let base_version = get_tag_version().unwrap_or_else(|| "0.1.0".to_string());

    // 2. Get commit count
    let commit_count = get_commit_count().unwrap_or(0);

    // 3. Get commit hash
    let commit_hash = get_commit_hash().unwrap_or_else(|| "unknown".to_string());

    // 4. Check if working tree is dirty
    let dirty_suffix = if is_dirty() { "-dirty" } else { "" };

    // Generate three version formats
    let package_version = base_version.clone();
    let file_version = format!("{}.{}", base_version, commit_count);
    let build_info = format!("{}+build.{}.{}{}", base_version, commit_count, commit_hash, dirty_suffix);

    (package_version, file_version, build_info)
}

fn get_tag_version() -> Option<String> {
    Command::new("git")
        .args(&["describe", "--tags", "--abbrev=0", "--match", "v[0-9]*"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok()
            } else {
                None
            }
        })
        .map(|s| s.trim().trim_start_matches('v').to_string())
}

fn get_commit_count() -> Option<u32> {
    Command::new("git")
        .args(&["rev-list", "--count", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok()
            } else {
                None
            }
        })
        .and_then(|s| s.trim().parse().ok())
}

fn get_commit_hash() -> Option<String> {
    Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok()
            } else {
                None
            }
        })
        .map(|s| s.trim().to_string())
}

fn is_dirty() -> bool {
    Command::new("git")
        .args(&["diff", "--quiet"])
        .status()
        .map(|status| !status.success())
        .unwrap_or(false)
}

/// Parse version string (e.g., "1.2.3.169") into u64 for Windows VERSIONINFO
/// Format: [major.minor.patch.build] -> 0xMMMMmmmmPPPPbbbb
fn parse_version(version_str: &str) -> Option<u64> {
    let parts: Vec<u16> = version_str
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();

    if parts.len() >= 3 {
        let major = parts.get(0).copied().unwrap_or(0) as u64;
        let minor = parts.get(1).copied().unwrap_or(0) as u64;
        let patch = parts.get(2).copied().unwrap_or(0) as u64;
        let build = parts.get(3).copied().unwrap_or(0) as u64;

        // Pack into u64: high DWORD (major.minor), low DWORD (patch.build)
        Some((major << 48) | (minor << 32) | (patch << 16) | build)
    } else {
        None
    }
}
