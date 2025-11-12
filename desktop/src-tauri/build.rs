fn main() {
    // Pass version information to the build
    if let Ok(package_version) = std::env::var("PACKAGE_VERSION") {
        println!("cargo:rustc-env=PACKAGE_VERSION={}", package_version);
    }
    if let Ok(file_version) = std::env::var("FILE_VERSION") {
        println!("cargo:rustc-env=FILE_VERSION={}", file_version);
    }
    if let Ok(build_info) = std::env::var("BUILD_INFO") {
        println!("cargo:rustc-env=BUILD_INFO={}", build_info);
    }

    tauri_build::build()
}
