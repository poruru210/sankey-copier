// relay-server/src/port_resolver.rs
//
// Dynamic port resolution and runtime configuration management

use crate::config::{RuntimeConfig, RuntimeZeromqConfig, ZeroMqConfig};
use anyhow::{Context, Result};
use chrono::Utc;
use std::net::TcpListener;
use std::path::Path;

/// Default path for runtime configuration file
pub const RUNTIME_CONFIG_PATH: &str = "runtime.toml";

/// Resolved ZeroMQ ports (actual ports to use)
#[derive(Debug, Clone)]
pub struct ResolvedPorts {
    pub receiver_port: u16,
    pub sender_port: u16,
    pub config_sender_port: u16,
    /// Whether ports were dynamically assigned (true) or from config (false)
    pub is_dynamic: bool,
    /// When the ports were generated (only set if is_dynamic)
    pub generated_at: Option<chrono::DateTime<Utc>>,
}

impl ResolvedPorts {
    /// Get ZMQ receiver bind address
    pub fn receiver_address(&self) -> String {
        format!("tcp://*:{}", self.receiver_port)
    }

    /// Get ZMQ sender bind address
    pub fn sender_address(&self) -> String {
        format!("tcp://*:{}", self.sender_port)
    }

    /// Get ZMQ config sender bind address
    pub fn config_sender_address(&self) -> String {
        format!("tcp://*:{}", self.config_sender_port)
    }
}

/// Resolve ZeroMQ ports from config and runtime.toml
///
/// Resolution order:
/// 1. If runtime.toml exists, use those ports
/// 2. If config has port=0 (dynamic), find available ports and save to runtime.toml
/// 3. Otherwise, use ports from config.toml directly
pub fn resolve_ports<P: AsRef<Path>>(
    config: &ZeroMqConfig,
    runtime_path: P,
) -> Result<ResolvedPorts> {
    let runtime_path = runtime_path.as_ref();

    // 1. Check if runtime.toml exists
    if RuntimeConfig::exists(runtime_path) {
        tracing::info!(
            "Loading ports from runtime config: {}",
            runtime_path.display()
        );
        let runtime = RuntimeConfig::load(runtime_path)?;
        return Ok(ResolvedPorts {
            receiver_port: runtime.zeromq.receiver_port,
            sender_port: runtime.zeromq.sender_port,
            config_sender_port: runtime.zeromq.config_sender_port,
            is_dynamic: true,
            generated_at: Some(runtime.zeromq.generated_at),
        });
    }

    // 2. Check if dynamic port assignment is needed
    if config.has_dynamic_ports() {
        tracing::info!("Dynamic port assignment enabled, finding available ports...");
        let ports = find_available_ports(3)?;

        let receiver_port = if config.receiver_port == 0 {
            ports[0]
        } else {
            config.receiver_port
        };
        let sender_port = if config.sender_port == 0 {
            ports[1]
        } else {
            config.sender_port
        };
        let config_sender_port = if config.config_sender_port == 0 {
            ports[2]
        } else {
            config.config_sender_port
        };

        // Save to runtime.toml for persistence
        let runtime = RuntimeConfig {
            zeromq: RuntimeZeromqConfig {
                receiver_port,
                sender_port,
                config_sender_port,
                generated_at: Utc::now(),
            },
        };
        runtime.save(runtime_path)?;
        tracing::info!(
            "Saved runtime config to {} with ports: receiver={}, sender={}, config_sender={}",
            runtime_path.display(),
            receiver_port,
            sender_port,
            config_sender_port
        );

        return Ok(ResolvedPorts {
            receiver_port,
            sender_port,
            config_sender_port,
            is_dynamic: true,
            generated_at: Some(runtime.zeromq.generated_at),
        });
    }

    // 3. Use fixed ports from config
    tracing::info!(
        "Using fixed ports from config: receiver={}, sender={}, config_sender={}",
        config.receiver_port,
        config.sender_port,
        config.config_sender_port
    );
    Ok(ResolvedPorts {
        receiver_port: config.receiver_port,
        sender_port: config.sender_port,
        config_sender_port: config.config_sender_port,
        is_dynamic: false,
        generated_at: None,
    })
}

/// Find N available TCP ports
fn find_available_ports(count: usize) -> Result<Vec<u16>> {
    let mut ports = Vec::with_capacity(count);
    let mut listeners = Vec::with_capacity(count);

    for _ in 0..count {
        // Bind to port 0 to let OS assign an available port
        let listener =
            TcpListener::bind("127.0.0.1:0").context("Failed to bind to available port")?;
        let port = listener.local_addr()?.port();
        ports.push(port);
        // Keep listener alive to prevent port reuse
        listeners.push(listener);
    }

    tracing::debug!("Found available ports: {:?}", ports);
    Ok(ports)
}

/// Reset ports by deleting runtime.toml
/// Next startup will re-assign ports
#[allow(dead_code)]
pub fn reset_ports<P: AsRef<Path>>(runtime_path: P) -> Result<()> {
    RuntimeConfig::delete(runtime_path)?;
    tracing::info!("Runtime config deleted, ports will be re-assigned on next startup");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_find_available_ports() {
        let ports = find_available_ports(3).unwrap();
        assert_eq!(ports.len(), 3);
        // All ports should be different
        assert_ne!(ports[0], ports[1]);
        assert_ne!(ports[1], ports[2]);
        assert_ne!(ports[0], ports[2]);
        // All ports should be > 0
        assert!(ports.iter().all(|&p| p > 0));
    }

    #[test]
    fn test_resolve_ports_fixed() {
        let dir = tempdir().unwrap();
        let runtime_path = dir.path().join("runtime.toml");

        let config = ZeroMqConfig {
            receiver_port: 5555,
            sender_port: 5556,
            config_sender_port: 5557,
            timeout_seconds: 30,
        };

        let resolved = resolve_ports(&config, &runtime_path).unwrap();
        assert_eq!(resolved.receiver_port, 5555);
        assert_eq!(resolved.sender_port, 5556);
        assert_eq!(resolved.config_sender_port, 5557);
        assert!(!resolved.is_dynamic);
        assert!(resolved.generated_at.is_none());
        // runtime.toml should NOT be created for fixed ports
        assert!(!runtime_path.exists());
    }

    #[test]
    fn test_resolve_ports_dynamic() {
        let dir = tempdir().unwrap();
        let runtime_path = dir.path().join("runtime.toml");

        let config = ZeroMqConfig {
            receiver_port: 0,      // dynamic
            sender_port: 0,        // dynamic
            config_sender_port: 0, // dynamic
            timeout_seconds: 30,
        };

        let resolved = resolve_ports(&config, &runtime_path).unwrap();
        assert!(resolved.receiver_port > 0);
        assert!(resolved.sender_port > 0);
        assert!(resolved.config_sender_port > 0);
        assert!(resolved.is_dynamic);
        assert!(resolved.generated_at.is_some());
        // runtime.toml should be created
        assert!(runtime_path.exists());
    }

    #[test]
    fn test_resolve_ports_from_runtime() {
        let dir = tempdir().unwrap();
        let runtime_path = dir.path().join("runtime.toml");

        // Pre-create runtime.toml
        let runtime = RuntimeConfig {
            zeromq: RuntimeZeromqConfig {
                receiver_port: 12345,
                sender_port: 12346,
                config_sender_port: 12347,
                generated_at: Utc::now(),
            },
        };
        runtime.save(&runtime_path).unwrap();

        // Config has different ports, but runtime.toml should take precedence
        let config = ZeroMqConfig {
            receiver_port: 5555,
            sender_port: 5556,
            config_sender_port: 5557,
            timeout_seconds: 30,
        };

        let resolved = resolve_ports(&config, &runtime_path).unwrap();
        assert_eq!(resolved.receiver_port, 12345);
        assert_eq!(resolved.sender_port, 12346);
        assert_eq!(resolved.config_sender_port, 12347);
        assert!(resolved.is_dynamic);
    }

    #[test]
    fn test_reset_ports() {
        let dir = tempdir().unwrap();
        let runtime_path = dir.path().join("runtime.toml");

        // Create runtime.toml
        let runtime = RuntimeConfig {
            zeromq: RuntimeZeromqConfig {
                receiver_port: 12345,
                sender_port: 12346,
                config_sender_port: 12347,
                generated_at: Utc::now(),
            },
        };
        runtime.save(&runtime_path).unwrap();
        assert!(runtime_path.exists());

        // Reset should delete the file
        reset_ports(&runtime_path).unwrap();
        assert!(!runtime_path.exists());
    }

    #[test]
    fn test_resolved_ports_addresses() {
        let resolved = ResolvedPorts {
            receiver_port: 5555,
            sender_port: 5556,
            config_sender_port: 5557,
            is_dynamic: false,
            generated_at: None,
        };

        assert_eq!(resolved.receiver_address(), "tcp://*:5555");
        assert_eq!(resolved.sender_address(), "tcp://*:5556");
        assert_eq!(resolved.config_sender_address(), "tcp://*:5557");
    }
}
