// test_server.rs
//
// Test helper for spawning relay-server instances for E2E testing.
// Provides automatic port allocation and server lifecycle management.

use anyhow::Result;
use sankey_copier_relay_server::{
    api::{create_router, AppState},
    config::Config,
    connection_manager::ConnectionManager,
    db::Database,
    engine::CopyEngine,
    log_buffer::create_log_buffer,
    message_handler::MessageHandler,
    zeromq::{ZmqConfigPublisher, ZmqMessage, ZmqSender, ZmqServer},
};
use std::net::TcpListener;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::task::JoinHandle;

/// Test server instance with dynamically allocated ports
pub struct TestServer {
    pub http_port: u16,
    pub zmq_pull_port: u16,
    pub zmq_pub_trade_port: u16,
    pub zmq_pub_config_port: u16,
    pub db: Arc<Database>,
    _server_handle: JoinHandle<()>,
    _zmq_handle: JoinHandle<()>,
}

impl TestServer {
    /// Start a new test server with dynamic port allocation
    pub async fn start() -> Result<Self> {
        // Find available ports
        let http_port = find_available_port()?;
        let zmq_pull_port = find_available_port()?;
        let zmq_pub_trade_port = find_available_port()?;
        let zmq_pub_config_port = find_available_port()?;

        // Initialize in-memory database
        let db = Arc::new(Database::new("sqlite::memory:").await?);

        // Initialize ConnectionManager
        let connection_manager = Arc::new(ConnectionManager::new(30));

        // Create channels
        let (zmq_tx, mut zmq_rx) = mpsc::unbounded_channel::<ZmqMessage>();
        let (broadcast_tx, _) = broadcast::channel::<String>(100);

        // Initialize ZeroMQ server
        let zmq_server = ZmqServer::new(zmq_tx)?;
        zmq_server
            .start_receiver(&format!("tcp://127.0.0.1:{}", zmq_pull_port))
            .await?;

        // Initialize ZeroMQ sender (PUB socket for trades)
        let zmq_sender = Arc::new(ZmqSender::new(&format!(
            "tcp://127.0.0.1:{}",
            zmq_pub_trade_port
        ))?);

        // Initialize ZeroMQ config sender (PUB socket for configs)
        let zmq_config_sender = Arc::new(ZmqConfigPublisher::new(&format!(
            "tcp://127.0.0.1:{}",
            zmq_pub_config_port
        ))?);

        // Initialize copy engine
        let copy_engine = Arc::new(CopyEngine::new());

        // Settings cache
        let settings_cache = Arc::new(RwLock::new(Vec::new()));

        // Load initial settings
        {
            let settings = db.list_copy_settings().await?;
            *settings_cache.write().await = settings;
        }

        // Initialize MessageHandler
        let handler = Arc::new(MessageHandler::new(
            connection_manager.clone(),
            copy_engine.clone(),
            zmq_sender.clone(),
            settings_cache.clone(),
            broadcast_tx.clone(),
            db.clone(),
            zmq_config_sender.clone(),
        ));

        // Spawn ZMQ message processing task
        let handler_clone = handler.clone();
        let zmq_handle = tokio::spawn(async move {
            while let Some(msg) = zmq_rx.recv().await {
                handler_clone.handle_message(msg).await;
            }
        });

        // Create log buffer
        let log_buffer = create_log_buffer();

        // Create app state
        let state = AppState {
            db: db.clone(),
            tx: broadcast_tx.clone(),
            settings_cache: settings_cache.clone(),
            connection_manager: connection_manager.clone(),
            config_sender: zmq_config_sender.clone(),
            log_buffer,
            allowed_origins: vec![],
            cors_disabled: true, // Disable CORS for tests
            config: Arc::new(Config::default()),
        };

        // Create router
        let app = create_router(state);

        // Spawn HTTP server
        let server_handle = tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", http_port))
                .await
                .expect("Failed to bind HTTP server");

            axum::serve(listener, app)
                .await
                .expect("HTTP server failed");
        });

        // Wait a bit for server to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        Ok(TestServer {
            http_port,
            zmq_pull_port,
            zmq_pub_trade_port,
            zmq_pub_config_port,
            db,
            _server_handle: server_handle,
            _zmq_handle: zmq_handle,
        })
    }

    /// Get the ZMQ PULL address (for EA to connect)
    pub fn zmq_pull_address(&self) -> String {
        format!("tcp://localhost:{}", self.zmq_pull_port)
    }

    /// Get the ZMQ PUB address for trades
    pub fn zmq_pub_trade_address(&self) -> String {
        format!("tcp://localhost:{}", self.zmq_pub_trade_port)
    }

    /// Get the ZMQ PUB address for configs
    pub fn zmq_pub_config_address(&self) -> String {
        format!("tcp://localhost:{}", self.zmq_pub_config_port)
    }

    /// Get the HTTP API base URL
    pub fn http_base_url(&self) -> String {
        format!("http://localhost:{}", self.http_port)
    }
}

// Drop implementation removed - let Rust's natural cleanup handle resources
// rust-zmq uses Arc references to manage Context/Socket lifecycle automatically
// Explicit abort() prevents proper Socket cleanup, causing Context termination to block

/// Find an available TCP port by binding to port 0 and letting OS choose
fn find_available_port() -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    Ok(port)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_starts() {
        let server = TestServer::start()
            .await
            .expect("Failed to start test server");
        assert!(server.http_port > 0);
        assert!(server.zmq_pull_port > 0);
        assert!(server.zmq_pub_trade_port > 0);
        assert!(server.zmq_pub_config_port > 0);

        println!("Test server started successfully on ports:");
        println!("  HTTP: {}", server.http_port);
        println!("  ZMQ PULL: {}", server.zmq_pull_port);
        println!("  ZMQ PUB (trades): {}", server.zmq_pub_trade_port);
        println!("  ZMQ PUB (configs): {}", server.zmq_pub_config_port);
    }
}
