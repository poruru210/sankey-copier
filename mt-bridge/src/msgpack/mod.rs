// Location: mt-bridge/src/msgpack/mod.rs
// Purpose: Module definition and public API exports for MessagePack functionality
// Why: Provides a clean public interface while organizing code into focused modules

// Module declarations

#[cfg(test)]
mod tests;

// Re-export types and traits from crate level for backwards compatibility
pub use crate::traits::{ConfigMessage, MasterConfig, SlaveConfig};
pub use crate::types::*;

// GlobalConfigMessage is defined here in msgpack module
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GlobalConfigMessage {
    // Add fields as needed
}
