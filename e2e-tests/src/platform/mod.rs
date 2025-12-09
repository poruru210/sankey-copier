// e2e-tests/src/platform/mod.rs
//
// MQL5 Platform Simulator & Test Environment
//
// 1. MQL5 Platform Simulation:
//    MQL5のイベント駆動モデルとスレッドモデルを再現するためのプラットフォーム基盤。
//    - OnInit, OnDeinit, OnTick, OnTimer, OnTrade, OnTradeTransaction
//    - すべてのイベントは単一のスレッドで順次実行されます。
//
// 2. Test Environment (Sandbox):
//    Relay Serverのプロセス管理と、Master/Slave EAの隔離実行環境を提供します。

pub mod traits;
pub mod types;
pub mod runner;

// Test Environment Modules
pub mod relay_server;
pub mod sandbox;

// テスト用モジュール
#[cfg(test)]
mod tests;
