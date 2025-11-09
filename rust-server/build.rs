use std::process::Command;

fn main() {
    // Get Git version using describe --always --dirty
    let git_version = Command::new("git")
        .args(&["describe", "--always", "--dirty"])
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
        .unwrap_or_else(|| "unknown".to_string());

    // Set environment variable for use in code
    println!("cargo:rustc-env=GIT_VERSION={}", git_version);

    // Rerun if .git/HEAD changes
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/refs/heads");
}
