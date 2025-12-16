// Library interface for forex-copier-server
// Exposes modules for integration testing

pub mod adapters;
pub mod application;
pub mod config;
pub mod config_builder;

pub mod domain;

pub mod bootstrap;
pub mod logging;
pub mod ports;
// pub mod runtime_status_updater; // Moved to application
// pub mod victoria_logs; // Moved to adapters::outbound::observability
