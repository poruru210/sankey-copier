// Build script for SANKEY Copier Desktop
// Configures Tauri build process and embeds Windows version information

use std::process::Command;

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

    // Run Tauri build (generates default Windows resources)
    tauri_build::build();

    // Patch the generated VERSIONINFO with CI-provided values
    #[cfg(windows)]
    {
        if let Err(err) = patch_windows_resource(&package_version, &file_version) {
            panic!("Failed to patch Windows VERSIONINFO: {err}");
        }
    }
}

#[cfg(windows)]
fn patch_windows_resource(package_version: &str, file_version: &str) -> Result<(), Box<dyn std::error::Error>> {
    use std::{env, fs, path::PathBuf};

    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let rc_path = out_dir.join("resource.rc");

    if !rc_path.exists() {
        return Err(format!("resource.rc not found at {}", rc_path.display()).into());
    }

    let original = fs::read_to_string(&rc_path)?;
    let newline = if original.contains("\r\n") { "\r\n" } else { "\n" };

    let mut replaced_file_numeric = false;
    let mut replaced_product_numeric = false;
    let mut replaced_file_string = false;
    let mut replaced_product_string = false;
    let file_tuple = format_file_tuple(file_version);

    let mut new_lines = Vec::new();
    for line in original.lines() {
        let trimmed = line.trim_start();
        let indent_len = line.len() - trimmed.len();
        let indent = &line[..indent_len];

        let updated = if trimmed.starts_with("FILEVERSION") {
            replaced_file_numeric = true;
            format!("{}FILEVERSION {}", indent, file_tuple)
        } else if trimmed.starts_with("PRODUCTVERSION") {
            replaced_product_numeric = true;
            format!("{}PRODUCTVERSION {}", indent, file_tuple)
        } else if trimmed.starts_with("VALUE \"FileVersion\"") {
            replaced_file_string = true;
            format!("{}VALUE \"FileVersion\", \"{}\"", indent, file_version)
        } else if trimmed.starts_with("VALUE \"ProductVersion\"") {
            replaced_product_string = true;
            format!("{}VALUE \"ProductVersion\", \"{}\"", indent, package_version)
        } else {
            line.to_string()
        };

        new_lines.push(updated);
    }

    if !replaced_file_numeric
        || !replaced_product_numeric
        || !replaced_file_string
        || !replaced_product_string
    {
        return Err("Failed to patch VERSIONINFO fields in resource.rc".into());
    }

    let mut new_contents = new_lines.join(newline);
    if original.ends_with("\r\n") && !new_contents.ends_with("\r\n") {
        new_contents.push_str("\r\n");
    } else if original.ends_with('\n') && !original.ends_with("\r\n") && !new_contents.ends_with('\n') {
        new_contents.push('\n');
    }

    fs::write(&rc_path, new_contents)?;
    recompile_resource(&rc_path, &out_dir)?;

    println!(
        "cargo:warning=Patched Windows VERSIONINFO to package={} file={}",
        package_version, file_version
    );

    Ok(())
}

#[cfg(windows)]
fn recompile_resource(rc_path: &std::path::Path, out_dir: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    use std::{env, process::Command};

    let rc_exe = find_rc_executable().ok_or("Unable to locate rc.exe. Set RC_EXE_PATH to override.")?;
    let output = out_dir.join("resource.lib");
    let mut command = Command::new(&rc_exe);
    command.arg("/nologo");

    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        command.arg(format!("/I{}", manifest_dir));
    }

    let status = command
        .arg(format!("/fo{}", output.display()))
        .arg(rc_path)
        .status()?;

    if !status.success() {
        return Err(format!("rc.exe failed with status {}", status).into());
    }

    Ok(())
}

#[cfg(windows)]
fn format_file_tuple(file_version: &str) -> String {
    let mut parts = [0u16; 4];
    for (idx, part) in file_version.split('.').take(4).enumerate() {
        parts[idx] = part.parse().unwrap_or(0);
    }

    format!("{}, {}, {}, {}", parts[0], parts[1], parts[2], parts[3])
}

#[cfg(windows)]
fn find_rc_executable() -> Option<std::path::PathBuf> {
    use std::{env, fs, path::PathBuf};

    if let Ok(custom) = env::var("RC_EXE_PATH") {
        let candidate = PathBuf::from(custom);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    let mut candidates = Vec::new();
    let mut roots = vec![
        PathBuf::from(r"C:\\Program Files (x86)\\Windows Kits\\10\\bin"),
        PathBuf::from(r"C:\\Program Files\\Windows Kits\\10\\bin"),
    ];

    if let Ok(sdk_dir) = env::var("WindowsSdkDir") {
        roots.push(PathBuf::from(sdk_dir).join("bin"));
    }

    for root in roots {
        if !root.exists() {
            continue;
        }

        let direct = root.join("x64").join("rc.exe");
        if direct.exists() {
            candidates.push((version_key(&direct), direct.clone()));
        }

        if let Ok(entries) = fs::read_dir(&root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let rc = path.join("x64").join("rc.exe");
                    if rc.exists() {
                        candidates.push((version_key(&rc), rc));
                    }
                }
            }
        }
    }

    candidates.sort_by(|a, b| a.0.cmp(&b.0));
    candidates.pop().map(|(_, path)| path)
}

#[cfg(windows)]
fn version_key(path: &std::path::Path) -> (u32, u32, u32, u32) {
    if let Some(version_dir) = path
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.file_name())
    {
        let mut parts = [0u32; 4];
        for (idx, part) in version_dir
            .to_string_lossy()
            .split('.')
            .filter_map(|s| s.parse::<u32>().ok())
            .take(4)
            .enumerate()
        {
            parts[idx] = part;
        }
        (parts[0], parts[1], parts[2], parts[3])
    } else {
        (0, 0, 0, 0)
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
