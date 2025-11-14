// SANKEY Copier Desktop Application
// Tauri-based desktop app that serves static Next.js export
// No Node.js runtime required - all pages pre-rendered at build time

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .run(tauri::generate_context!())
        .expect("Error while running Tauri application");
}
