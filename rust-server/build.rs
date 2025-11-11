use std::process::Command;

#[cfg(windows)]
extern crate winres;

fn main() {
    // Rerun if environment variables change
    println!("cargo:rerun-if-env-changed=PACKAGE_VERSION");
    println!("cargo:rerun-if-env-changed=FILE_VERSION");

    // Generate version information
    let (package_version, file_version, build_info) = generate_version_info();

    // Set environment variables for use in code
    println!("cargo:rustc-env=PACKAGE_VERSION={}", package_version);
    println!("cargo:rustc-env=FILE_VERSION={}", file_version);
    println!("cargo:rustc-env=BUILD_INFO={}", build_info);

    // Display version information prominently during build
    println!("cargo:warning=╔════════════════════════════════════════════════════════════════");
    println!("cargo:warning=║ Building rust-server");
    println!("cargo:warning=║ PACKAGE_VERSION: {}", package_version);
    println!("cargo:warning=║ FILE_VERSION:    {}", file_version);
    println!("cargo:warning=║ BUILD_INFO:      {}", build_info);
    println!("cargo:warning=╚════════════════════════════════════════════════════════════════");

    // Embed version information in Windows executable resources
    #[cfg(windows)]
    embed_windows_resources(&package_version, &file_version);

    // Rerun if .git/HEAD changes
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/refs/heads");
}

#[cfg(windows)]
fn embed_windows_resources(package_version: &str, file_version: &str) {
    // Parse file version into 4-component format (MAJOR.MINOR.PATCH.BUILD)
    let file_parts: Vec<&str> = file_version.split('.').collect();
    let file_ver_string = if file_parts.len() >= 4 {
        file_version.to_string()
    } else {
        // Ensure we have 4 components
        let mut parts = file_parts.to_vec();
        while parts.len() < 4 {
            parts.push("0");
        }
        parts.join(".")
    };

    let mut res = winres::WindowsResource::new();

    // Set version information
    res.set("ProductVersion", package_version)
        .set("ProductName", "SANKEY Copier Server")
        .set("FileVersion", &file_ver_string)
        .set("FileDescription", "Backend server for SANKEY Copier MT4/MT5 trade copying system")
        .set("CompanyName", "SANKEY Copier Project")
        .set("LegalCopyright", "Copyright (C) 2025 SANKEY Copier Project")
        .set("OriginalFilename", "sankey-copier-server.exe");

    // Compile the resource file
    if let Err(e) = res.compile() {
        eprintln!("Failed to compile Windows resources: {}", e);
        // Don't fail the build, just warn
    } else {
        println!("cargo:warning=Successfully embedded Windows resources");
    }
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
