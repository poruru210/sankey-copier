// Prevents additional console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Manager, State, WindowEvent};

struct AppState {
    node_process: Arc<Mutex<Option<Child>>>,
    web_ui_port: u16,
}

/// Find an available port by binding to port 0 (OS assigns free port)
fn find_available_port() -> Result<u16, String> {
    TcpListener::bind("127.0.0.1:0")
        .map_err(|e| format!("Failed to find available port: {}", e))?
        .local_addr()
        .map(|addr| addr.port())
        .map_err(|e| format!("Failed to get port: {}", e))
}

/// Start the Next.js standalone server
fn start_nextjs_server(port: u16) -> Result<Child, String> {
    // Determine web-ui path based on whether we're in dev or production
    let web_ui_path = if cfg!(debug_assertions) {
        // Development: use workspace root
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("web-ui")
            .join(".next")
            .join("standalone")
    } else {
        // Production: web-ui is bundled alongside the executable
        std::env::current_exe()
            .map_err(|e| format!("Failed to get executable path: {}", e))?
            .parent()
            .ok_or_else(|| "Failed to get executable directory".to_string())?
            .join("web-ui")
    };

    let server_js = web_ui_path.join("server.js");

    if !server_js.exists() {
        return Err(format!(
            "Web UI server.js not found at: {}",
            server_js.display()
        ));
    }

    println!("Starting Next.js server at: {}", server_js.display());
    println!("Port: {}", port);

    let mut child = Command::new("node")
        .arg(server_js.to_str().unwrap())
        .env("PORT", port.to_string())
        .env("HOSTNAME", "127.0.0.1")
        .current_dir(&web_ui_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start Node.js process: {}. Make sure Node.js is installed and in PATH.", e))?;

    // Capture stdout in a separate thread for logging
    if let Some(stdout) = child.stdout.take() {
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if let Ok(line) = line {
                    println!("[Web UI] {}", line);
                }
            }
        });
    }

    // Capture stderr in a separate thread for logging
    if let Some(stderr) = child.stderr.take() {
        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                if let Ok(line) = line {
                    eprintln!("[Web UI Error] {}", line);
                }
            }
        });
    }

    println!("Node.js process started with PID: {:?}", child.id());
    Ok(child)
}

/// Wait for the server to be ready by polling the port
fn wait_for_server(port: u16, timeout_secs: u64) -> Result<(), String> {
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(timeout_secs);

    loop {
        if start.elapsed() > timeout {
            return Err(format!(
                "Server did not start within {} seconds",
                timeout_secs
            ));
        }

        // Try to connect to the port
        if TcpListener::bind(format!("127.0.0.1:{}", port)).is_err() {
            // Port is in use, meaning server is running
            println!("Server is ready on port {}", port);
            return Ok(());
        }

        thread::sleep(Duration::from_millis(100));
    }
}

fn main() {
    // Find available port
    let port = find_available_port().expect("Failed to find available port");
    println!("Using port: {}", port);

    // Start Next.js server
    let node_process = start_nextjs_server(port).expect("Failed to start Next.js server");

    // Wait for server to be ready
    wait_for_server(port, 30).expect("Server failed to start");

    let app_state = AppState {
        node_process: Arc::new(Mutex::new(Some(node_process))),
        web_ui_port: port,
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .setup(move |app| {
            let window = app.get_webview_window("main").unwrap();

            // Navigate to the dynamically determined port
            let url = format!("http://localhost:{}", port);
            println!("Loading URL: {}", url);
            window.eval(&format!("window.location.href = '{}';", url))
                .map_err(|e| format!("Failed to navigate: {}", e))?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { .. } = event {
                // Cleanup on window close
                let state: State<AppState> = window.state();
                if let Ok(mut process) = state.node_process.lock() {
                    if let Some(mut child) = process.take() {
                        println!("Terminating Node.js process...");
                        let _ = child.kill();
                        let _ = child.wait();
                        println!("Node.js process terminated");
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
