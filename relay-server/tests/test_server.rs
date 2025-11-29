// test_server.rs
//
// Test helper for spawning relay-server instances for E2E testing.
// Provides automatic port allocation and server lifecycle management.

use anyhow::Result;
use sankey_copier_relay_server::{
    api::{create_router, AppState},
    config::{Config, VictoriaLogsConfig},
    connection_manager::ConnectionManager,
    db::Database,
    engine::CopyEngine,
    log_buffer::create_log_buffer,
    message_handler::MessageHandler,
    port_resolver::ResolvedPorts,
    victoria_logs::VLogsController,
    zeromq::{ZmqConfigPublisher, ZmqMessage, ZmqSender, ZmqServer},
};
use std::net::TcpListener;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;

/// Test server instance with dynamically allocated ports
#[allow(dead_code)]
pub struct TestServer {
    pub http_port: u16,
    pub zmq_pull_port: u16,
    pub zmq_pub_trade_port: u16,
    pub zmq_pub_config_port: u16,
    pub db: Arc<Database>,
    zmq_server: Arc<ZmqServer>,
    server_handle: Option<JoinHandle<()>>,
    zmq_receiver_handle: Option<JoinHandle<()>>,
    zmq_handler_handle: Option<JoinHandle<()>>,
}

impl TestServer {
    /// Start a new test server with dynamic port allocation
    pub async fn start() -> Result<Self> {
        // Bind to port 0 to get available ports immediately (avoiding TOCTOU race)
        let http_listener = TcpListener::bind("127.0.0.1:0")?;
        let http_port = http_listener.local_addr()?.port();

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
        let zmq_server = Arc::new(ZmqServer::new(zmq_tx)?);
        let zmq_receiver_handle = zmq_server
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

        // Initialize MessageHandler
        let handler = Arc::new(MessageHandler::new(
            connection_manager.clone(),
            copy_engine.clone(),
            zmq_sender.clone(),
            broadcast_tx.clone(),
            db.clone(),
            zmq_config_sender.clone(),
        ));

        // Spawn ZMQ message processing task
        let handler_clone = handler.clone();
        let zmq_handler_handle = tokio::spawn(async move {
            while let Some(msg) = zmq_rx.recv().await {
                handler_clone.handle_message(msg).await;
            }
        });

        // Create log buffer
        let log_buffer = create_log_buffer();

        // Create VLogsController for testing
        let vlogs_enabled = Arc::new(AtomicBool::new(true));
        let vlogs_config = VictoriaLogsConfig {
            enabled: true,
            host: "http://localhost:9428".to_string(),
            batch_size: 100,
            flush_interval_secs: 5,
            source: "test-relay".to_string(),
        };
        let vlogs_controller = Some(VLogsController::new(vlogs_enabled, vlogs_config));

        // Create resolved ports for tests
        let resolved_ports = Arc::new(ResolvedPorts {
            receiver_port: zmq_pull_port,
            sender_port: zmq_pub_trade_port,
            config_sender_port: zmq_pub_config_port,
            is_dynamic: true,
            generated_at: Some(chrono::Utc::now()),
        });

        // Create app state
        let state = AppState {
            db: db.clone(),
            tx: broadcast_tx.clone(),
            connection_manager: connection_manager.clone(),
            config_sender: zmq_config_sender.clone(),
            log_buffer,
            allowed_origins: vec![],
            cors_disabled: true, // Disable CORS for tests
            config: Arc::new(Config::default()),
            resolved_ports,
            vlogs_controller,
        };

        // Create router
        let app = create_router(state);

        // Spawn HTTP server using the pre-bound listener
        let server_handle = tokio::spawn(async move {
            // Convert std::net::TcpListener to tokio::net::TcpListener
            http_listener
                .set_nonblocking(true)
                .expect("Failed to set non-blocking");
            let listener = tokio::net::TcpListener::from_std(http_listener)
                .expect("Failed to convert listener");

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
            zmq_server,
            server_handle: Some(server_handle),
            zmq_receiver_handle: Some(zmq_receiver_handle),
            zmq_handler_handle: Some(zmq_handler_handle),
        })
    }

    /// Get the ZMQ PULL address (for EA to connect)
    #[allow(dead_code)]
    pub fn zmq_pull_address(&self) -> String {
        format!("tcp://localhost:{}", self.zmq_pull_port)
    }

    /// Get the ZMQ PUB address for trades
    #[allow(dead_code)]
    pub fn zmq_pub_trade_address(&self) -> String {
        format!("tcp://localhost:{}", self.zmq_pub_trade_port)
    }

    /// Get the ZMQ PUB address for configs
    #[allow(dead_code)]
    pub fn zmq_pub_config_address(&self) -> String {
        format!("tcp://localhost:{}", self.zmq_pub_config_port)
    }

    /// Get the HTTP API base URL
    #[allow(dead_code)]
    pub fn http_base_url(&self) -> String {
        format!("http://localhost:{}", self.http_port)
    }

    /// Explicitly shutdown the test server and wait for all tasks to complete
    ///
    /// This method should be called at the end of each test to ensure clean shutdown.
    /// Without calling this, background tasks may continue running and cause tests to hang.
    #[allow(dead_code)]
    pub async fn shutdown(mut self) {
        tracing::info!("TestServer shutting down...");

        // Signal ZMQ receiver to shutdown
        self.zmq_server.shutdown();

        // Abort all background tasks
        if let Some(handle) = self.server_handle.take() {
            handle.abort();
        }
        if let Some(handle) = self.zmq_receiver_handle.take() {
            // Wait for ZMQ receiver to finish (should be quick after shutdown signal)
            let _ = tokio::time::timeout(tokio::time::Duration::from_millis(500), handle).await;
        }
        if let Some(handle) = self.zmq_handler_handle.take() {
            handle.abort();
        }

        // Give a bit of time for cleanup
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        tracing::info!("TestServer shutdown complete");
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        // Signal ZMQ receiver to shutdown
        self.zmq_server.shutdown();
        tracing::info!("TestServer dropping - ZMQ shutdown signaled");
    }
}

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
