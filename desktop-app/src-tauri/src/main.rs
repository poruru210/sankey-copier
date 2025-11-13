// SANKEY Copier Desktop Application
// Tauri-based desktop app that launches Next.js server on dynamic port
// and displays the web UI in a native window

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::{Child, Command};
use std::sync::Mutex;
use std::time::Duration;
use std::thread;
use tauri::{Manager, State};

// Application state to track the Node.js process
struct AppState {
    nextjs_process: Mutex<Option<Child>>,
    port: u16,
}

/// Find an available port for the Next.js server
/// Uses portpicker crate to find an unused port
fn find_available_port() -> u16 {
    portpicker::pick_unused_port()
        .expect("Failed to find available port")
}

/// Check if a port is responding by attempting to connect
/// This is used to wait for the Next.js server to be ready
fn is_port_ready(port: u16) -> bool {
    std::net::TcpStream::connect(format!("127.0.0.1:{}", port))
        .is_ok()
}

/// Wait for the Next.js server to start responding
/// Tries for up to 30 seconds with 500ms intervals
fn wait_for_server(port: u16, max_attempts: u32) -> bool {
    println!("Waiting for Next.js server to start on port {}...", port);

    for attempt in 0..max_attempts {
        if is_port_ready(port) {
            println!("✓ Server ready on port {} after {} attempts", port, attempt + 1);
            return true;
        }

        // Show progress every 2 seconds (4 attempts)
        if attempt % 4 == 0 && attempt > 0 {
            println!("  Still waiting... ({}/{} attempts)", attempt + 1, max_attempts);
        }

        thread::sleep(Duration::from_millis(500));
    }

    eprintln!("✗ Server failed to start within {} seconds", max_attempts / 2);
    false
}

/// Start the Next.js server as a child process
/// Returns the Child process handle and the port number
fn start_nextjs_server() -> Result<(Child, u16), String> {
    let port = find_available_port();

    // Determine the path to the Next.js server
    // In production, it's relative to the app installation directory
    let exe_dir = std::env::current_exe()
        .map_err(|e| format!("Failed to get exe directory: {}", e))?
        .parent()
        .ok_or("Failed to get parent directory")?
        .to_path_buf();

    let server_path = exe_dir.join("web-ui").join("server.js");

    if !server_path.exists() {
        return Err(format!("Server not found at: {}", server_path.display()));
    }

    println!("Starting Next.js server on port {} from {}", port, server_path.display());

    // Start Node.js process with the server script
    let child = Command::new("node")
        .arg(server_path)
        .env("PORT", port.to_string())
        .env("HOSTNAME", "127.0.0.1")
        .spawn()
        .map_err(|e| format!("Failed to start Node.js: {}. Is Node.js installed?", e))?;

    Ok((child, port))
}

#[tauri::command]
fn get_server_url(state: State<AppState>) -> String {
    format!("http://localhost:{}", state.port)
}

fn main() {
    // Start the Next.js server
    let (nextjs_child, port) = match start_nextjs_server() {
        Ok((child, port)) => (child, port),
        Err(e) => {
            eprintln!("Failed to start server: {}", e);

            // Show error message to user
            let error_message = format!(
                "Failed to start SANKEY Copier Desktop:\n\n{}\n\n\
                 Please ensure:\n\
                 - Node.js v20 LTS is installed\n\
                 - Installation directory contains web-ui folder\n\n\
                 インストールディレクトリに web-ui フォルダが存在することを確認してください。",
                e
            );

            // Note: We can't show a GUI dialog before Tauri initializes,
            // so we print to stderr and exit. The error will be visible
            // if the user runs from command line.
            eprintln!("\n{}\n", error_message);

            std::process::exit(1);
        }
    };

    println!("Next.js server started, waiting for it to be ready...");

    // Wait for server to be ready (max 30 seconds)
    if !wait_for_server(port, 60) {
        eprintln!("Server failed to start within timeout period");
        eprintln!("Next.js サーバーが起動しませんでした。Node.js がインストールされているか確認してください。");
        std::process::exit(1);
    }

    let app_state = AppState {
        nextjs_process: Mutex::new(Some(nextjs_child)),
        port,
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![get_server_url])
        .setup(move |app| {
            let window = app.get_webview_window("main")
                .expect("Failed to get main window");

            // Navigate to the Next.js server
            let url = format!("http://localhost:{}", port);
            println!("Loading URL: {}", url);
            window.eval(&format!("window.location.href = '{}'", url))?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                println!("Window destroyed, cleaning up...");

                // Get the app state and kill the Node.js process
                let state: State<AppState> = window.state();
                if let Ok(mut process) = state.nextjs_process.lock() {
                    if let Some(mut child) = process.take() {
                        println!("Killing Next.js process...");
                        let _ = child.kill();
                        let _ = child.wait();
                        println!("Next.js process terminated");
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("Error while running Tauri application");
}
