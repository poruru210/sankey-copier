//! Tests for trade signal message handling

use super::*;
use crate::domain::models::{LotCalculationMode, SlaveSettings, TradeFilters};

#[tokio::test]
async fn test_handle_trade_signal_with_matching_setting() {
    // Use TestContext for proper ZeroMQ resource cleanup
    let ctx = create_test_context().await;
    let signal = create_test_trade_signal();

    // Create TradeGroup for master and add member to database
    ctx.db.create_trade_group("MASTER_001").await.unwrap();

    let slave_settings = SlaveSettings {
        lot_calculation_mode: LotCalculationMode::default(),
        config_version: 1,
        symbol_prefix: None,
        symbol_suffix: None,
        lot_multiplier: Some(1.0),
        reverse_trade: false,
        symbol_mappings: vec![],
        filters: TradeFilters {
            allowed_symbols: None,
            blocked_symbols: None,
            allowed_magic_numbers: None,
            blocked_magic_numbers: None,
        },
        source_lot_min: None,
        source_lot_max: None,
        sync_mode: crate::domain::models::SyncMode::Skip,
        limit_order_expiry_min: None,
        market_sync_max_pips: None,
        max_slippage: None,
        copy_pending_orders: false,
        // Trade Execution defaults
        max_retries: 3,
        max_signal_delay_ms: 5000,
        use_pending_order_for_delayed: false,
    };
    ctx.db
        .add_member("MASTER_001", "SLAVE_001", slave_settings, 0)
        .await
        .unwrap();

    // Process trade signal (should not panic)
    ctx.handle_trade_signal(signal).await;

    // Explicit cleanup
    ctx.cleanup().await;
}

#[tokio::test]
async fn test_handle_trade_signal_no_matching_master() {
    let ctx = create_test_context().await;
    let mut signal = create_test_trade_signal();
    signal.source_account = "OTHER_MASTER".to_string();

    // Create TradeGroup for MASTER_001 (but signal is from OTHER_MASTER)
    ctx.db.create_trade_group("MASTER_001").await.unwrap();

    let slave_settings = SlaveSettings {
        lot_calculation_mode: LotCalculationMode::default(),
        config_version: 1,
        symbol_prefix: None,
        symbol_suffix: None,
        lot_multiplier: Some(1.0),
        reverse_trade: false,
        symbol_mappings: vec![],
        filters: TradeFilters {
            allowed_symbols: None,
            blocked_symbols: None,
            allowed_magic_numbers: None,
            blocked_magic_numbers: None,
        },
        source_lot_min: None,
        source_lot_max: None,
        sync_mode: crate::domain::models::SyncMode::Skip,
        limit_order_expiry_min: None,
        market_sync_max_pips: None,
        max_slippage: None,
        copy_pending_orders: false,
        // Trade Execution defaults
        max_retries: 3,
        max_signal_delay_ms: 5000,
        use_pending_order_for_delayed: false,
    };
    ctx.db
        .add_member("MASTER_001", "SLAVE_001", slave_settings, 0)
        .await
        .unwrap();

    // Process trade signal (should be filtered out, no panic)
    ctx.handle_trade_signal(signal).await;

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_handle_trade_signal_disabled_setting() {
    let ctx = create_test_context().await;
    let signal = create_test_trade_signal();

    // Create TradeGroup and add member with DISABLED status
    ctx.db.create_trade_group("MASTER_001").await.unwrap();

    let slave_settings = SlaveSettings {
        lot_calculation_mode: LotCalculationMode::default(),
        config_version: 1,
        symbol_prefix: None,
        symbol_suffix: None,
        lot_multiplier: Some(1.0),
        reverse_trade: false,
        symbol_mappings: vec![],
        filters: TradeFilters {
            allowed_symbols: None,
            blocked_symbols: None,
            allowed_magic_numbers: None,
            blocked_magic_numbers: None,
        },
        source_lot_min: None,
        source_lot_max: None,
        sync_mode: crate::domain::models::SyncMode::Skip,
        limit_order_expiry_min: None,
        market_sync_max_pips: None,
        max_slippage: None,
        copy_pending_orders: false,
        // Trade Execution defaults
        max_retries: 3,
        max_signal_delay_ms: 5000,
        use_pending_order_for_delayed: false,
    };
    // Add member and then disable it
    ctx.db
        .add_member("MASTER_001", "SLAVE_001", slave_settings, 0)
        .await
        .unwrap();
    ctx.db
        .update_member_runtime_status("MASTER_001", "SLAVE_001", 0)
        .await
        .unwrap(); // STATUS_DISABLED = 0

    // Process trade signal (should be filtered out, no panic)
    ctx.handle_trade_signal(signal).await;

    ctx.cleanup().await;
}
