// Library interface for forex-copier-server
// Exposes modules for integration testing

pub mod adapters;
pub mod application;
pub mod config;
pub mod config_builder;
pub mod connection_manager;
pub mod domain;

pub mod log_buffer;

pub mod mt_detector;
pub mod mt_installer;
pub mod port_resolver;
pub mod ports;
pub mod runtime_status_updater;
pub mod victoria_logs;
