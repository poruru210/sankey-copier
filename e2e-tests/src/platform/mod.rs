// e2e-tests/src/platform/mod.rs
//
// MQL5 Platform Simulator
//
// MQL5のイベント駆動モデルとスレッドモデルを再現するためのプラットフォーム基盤。
// 以下のイベントをサポート:
// - OnInit
// - OnDeinit
// - OnTick
// - OnTimer
// - OnTrade
// - OnTradeTransaction
//
// すべてのイベントは単一のスレッドで順次実行されます。

pub mod traits;
pub mod types;
pub mod runner;

// テスト用モジュール
#[cfg(test)]
mod tests;
