// e2e-tests/src/lib.rs
//
// Shared EA Simulator implementations for E2E testing.
// These simulators mimic the behavior of real Master/Slave EAs,
// serving as both test fixtures and implementation prototypes.
//
// DESIGN PRINCIPLE: These simulators MUST use mt-bridge FFI functions exclusively,
// matching the actual EA implementation exactly. This is intentional:
// 1. Simulators serve as EA prototype implementations for validation
// 2. Consistency with EA ensures tests catch real integration issues
// 3. Efficiency is NOT a priority - correctness and EA parity are
//
// COMPILE-TIME ENFORCEMENT:
// This crate intentionally does NOT depend on the `zmq` crate directly.
// All ZMQ operations go through mt-bridge FFI functions, ensuring that
// simulators cannot accidentally use raw ZMQ APIs.
//
// MQL5 EA CONFORMANCE (SankeyCopierMaster.mq5 / SankeyCopierSlave.mq5):
// - OnInit: ZMQ接続、トピック購読
// - OnTimer (100ms): Heartbeat判定 → RequestConfig → Config受信
// - グローバル変数: g_last_heartbeat, g_config_requested, etc.
//
// LIFECYCLE:
// 1. new() - Creates simulator and connects to ZMQ sockets (OnInit相当)
// 2. start() - Starts OnTimer loop thread (EventSetTimer相当)
// 3. OnTimer thread: Automatic heartbeat + RequestConfig + Config reception
// 4. drop() - Stops OnTimer thread and cleans up ZMQ resources (OnDeinit相当)
//
// ARCHITECTURE:
// - types: Common type definitions, constants, and re-exports
// - base: EaSimulatorBase with ZMQ connection management only
// - master: MasterEaSimulator with OnTimer loop and trade signals
// - slave: SlaveEaSimulator with OnTimer loop and config/trade reception

// Relay server process management for E2E tests
pub mod relay_server_process;

// Shared helper functions for E2E tests
pub mod helpers;

// Common types and constants
pub mod types;

// Base simulator functionality
pub mod base;

// Master EA simulator
pub mod master;

// Slave EA simulator
pub mod slave;

// Re-export main types for convenience
pub use base::EaSimulatorBase;
pub use master::MasterEaSimulator;
pub use slave::SlaveEaSimulator;
pub use types::{
    EaType, Heartbeat, HeartbeatParams, MasterConfigMessage, PositionInfo,
    PositionSnapshotMessage, RequestConfigMessage, SlaveConfig, SymbolMapping, SyncMode,
    SyncRequestMessage, TradeFilters, TradeSignalMessage, VLogsConfigMessage, BUFFER_SIZE,
    HEARTBEAT_INTERVAL_SECONDS, ONTIMER_INTERVAL_MS, STATUS_CONNECTED, STATUS_DISABLED,
    STATUS_ENABLED, STATUS_NO_CONFIG, TOPIC_BUFFER_SIZE,
};
