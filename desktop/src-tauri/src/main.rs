// Prevents additional console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Manager, WindowEvent};

// Version information from build environment
const VERSION: &str = env!("CARGO_PKG_VERSION");
const PACKAGE_VERSION: &str = match option_env!("PACKAGE_VERSION") {
    Some(v) => v,
    None => env!("CARGO_PKG_VERSION"),
};
const FILE_VERSION: &str = match option_env!("FILE_VERSION") {
    Some(v) => v,
    None => env!("CARGO_PKG_VERSION"),
};
const BUILD_INFO: &str = match option_env!("BUILD_INFO") {
    Some(v) => v,
    None => env!("CARGO_PKG_VERSION"),
};

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
    // Print version information on startup
    println!("╔════════════════════════════════════════════════════════════════");
    println!("║ SANKEY Copier Desktop Application");
    println!("║ Version: {}", VERSION);
    println!("║ Package: {}", PACKAGE_VERSION);
    println!("║ File Version: {}", FILE_VERSION);
    println!("║ Build Info: {}", BUILD_INFO);
    println!("╚════════════════════════════════════════════════════════════════");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Start server initialization in background
            thread::spawn(move || {
                // Find available port
                let port = match find_available_port() {
                    Ok(p) => {
                        println!("Using port: {}", p);
                        p
                    }
                    Err(e) => {
                        eprintln!("Failed to find available port: {}", e);
                        show_error(&app_handle, &format!("ポートの検出に失敗しました: {}", e));
                        return;
                    }
                };

                // Start Next.js server
                let node_process = match start_nextjs_server(port) {
                    Ok(p) => p,
                    Err(e) => {
                        eprintln!("Failed to start Next.js server: {}", e);
                        show_error(&app_handle, &format!("サーバーの起動に失敗しました:\n{}\n\nNode.jsがインストールされているか確認してください。", e));
                        return;
                    }
                };

                // Wait for server to be ready
                if let Err(e) = wait_for_server(port, 30) {
                    eprintln!("Server failed to start: {}", e);
                    show_error(&app_handle, &format!("サーバーの準備に失敗しました: {}", e));
                    return;
                }

                // Server is ready, update state and show main window
                let app_state = AppState {
                    node_process: Arc::new(Mutex::new(Some(node_process))),
                    web_ui_port: port,
                };

                app_handle.manage(app_state);

                // Close splash and show main window
                if let Some(splash) = app_handle.get_webview_window("splashscreen") {
                    let _ = splash.close();
                }

                if let Some(main_window) = app_handle.get_webview_window("main") {
                    let url = format!("http://localhost:{}", port);
                    println!("Loading URL: {}", url);

                    // Update window title with version
                    let _ = main_window.set_title(&format!("SANKEY Copier v{}", PACKAGE_VERSION));

                    let _ = main_window.eval(&format!("window.location.href = '{}';", url));
                    let _ = main_window.show();
                    let _ = main_window.set_focus();
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { .. } = event {
                if window.label() == "main" {
                    // Cleanup on main window close
                    if let Some(state) = window.try_state::<AppState>() {
                        if let Ok(mut process) = state.node_process.lock() {
                            if let Some(mut child) = process.take() {
                                println!("Terminating Node.js process...");
                                let _ = child.kill();
                                let _ = child.wait();
                                println!("Node.js process terminated");
                            }
                        }
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Show error message in main window
fn show_error(app: &AppHandle, message: &str) {
    // Close splash if open
    if let Some(splash) = app.get_webview_window("splashscreen") {
        let _ = splash.close();
    }

    // Show error in main window
    if let Some(main_window) = app.get_webview_window("main") {
        let error_html = format!(
            r#"
            <html>
            <head>
                <style>
                    body {{
                        margin: 0;
                        padding: 0;
                        display: flex;
                        justify-content: center;
                        align-items: center;
                        min-height: 100vh;
                        background: #f5f5f5;
                        font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
                    }}
                    .error-container {{
                        background: white;
                        padding: 40px;
                        border-radius: 12px;
                        box-shadow: 0 4px 20px rgba(0,0,0,0.1);
                        max-width: 500px;
                        text-align: center;
                    }}
                    .error-icon {{
                        font-size: 64px;
                        margin-bottom: 20px;
                    }}
                    h1 {{
                        color: #e53e3e;
                        margin-bottom: 20px;
                        font-size: 24px;
                    }}
                    p {{
                        color: #4a5568;
                        line-height: 1.6;
                        white-space: pre-wrap;
                        margin-bottom: 30px;
                    }}
                    button {{
                        background: #667eea;
                        color: white;
                        border: none;
                        padding: 12px 24px;
                        border-radius: 6px;
                        font-size: 16px;
                        cursor: pointer;
                        transition: background 0.2s;
                    }}
                    button:hover {{
                        background: #5568d3;
                    }}
                </style>
            </head>
            <body>
                <div class="error-container">
                    <div class="error-icon">❌</div>
                    <h1>起動エラー</h1>
                    <p>{}</p>
                    <button onclick="window.close()">閉じる</button>
                    <div style="margin-top: 30px; padding-top: 20px; border-top: 1px solid #e2e8f0; font-size: 12px; color: #a0aec0;">
                        Version: {} ({})<br>
                        Build: {}
                    </div>
                </div>
            </body>
            </html>
            "#,
            message.replace("\"", "&quot;"),
            PACKAGE_VERSION,
            FILE_VERSION,
            BUILD_INFO
        );

        let _ = main_window.set_title(&format!("SANKEY Copier v{} - Error", PACKAGE_VERSION));
        let _ = main_window.eval(&format!(
            "document.open(); document.write(`{}`); document.close();",
            error_html.replace("`", "\\`")
        ));
        let _ = main_window.show();
        let _ = main_window.set_focus();
    }
}
