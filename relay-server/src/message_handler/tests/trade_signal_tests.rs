//! Tests for trade signal message handling

use super::*;
use crate::models::{LotCalculationMode, SlaveSettings, TradeFilters};

#[tokio::test]
async fn test_handle_trade_signal_with_matching_setting() {
    let handler = create_test_handler().await;
    let signal = create_test_trade_signal();

    // Create TradeGroup for master and add member to database
    {
        handler.db.create_trade_group("MASTER_001").await.unwrap();

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
            sync_mode: crate::models::SyncMode::Skip,
            limit_order_expiry_min: None,
            market_sync_max_pips: None,
            max_slippage: None,
            copy_pending_orders: false,
        };
        handler
            .db
            .add_member("MASTER_001", "SLAVE_001", slave_settings)
            .await
            .unwrap();
    }

    // Process trade signal (should not panic)
    handler.handle_trade_signal(signal).await;
}

#[tokio::test]
async fn test_handle_trade_signal_no_matching_master() {
    let handler = create_test_handler().await;
    let mut signal = create_test_trade_signal();
    signal.source_account = "OTHER_MASTER".to_string();

    // Create TradeGroup for MASTER_001 (but signal is from OTHER_MASTER)
    {
        handler.db.create_trade_group("MASTER_001").await.unwrap();

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
            sync_mode: crate::models::SyncMode::Skip,
            limit_order_expiry_min: None,
            market_sync_max_pips: None,
            max_slippage: None,
            copy_pending_orders: false,
        };
        handler
            .db
            .add_member("MASTER_001", "SLAVE_001", slave_settings)
            .await
            .unwrap();
    }

    // Process trade signal (should be filtered out, no panic)
    handler.handle_trade_signal(signal).await;
}

#[tokio::test]
async fn test_handle_trade_signal_disabled_setting() {
    let handler = create_test_handler().await;
    let signal = create_test_trade_signal();

    // Create TradeGroup and add member with DISABLED status
    {
        handler.db.create_trade_group("MASTER_001").await.unwrap();

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
            sync_mode: crate::models::SyncMode::Skip,
            limit_order_expiry_min: None,
            market_sync_max_pips: None,
            max_slippage: None,
            copy_pending_orders: false,
        };
        // Add member and then disable it
        handler
            .db
            .add_member("MASTER_001", "SLAVE_001", slave_settings)
            .await
            .unwrap();
        handler
            .db
            .update_member_status("MASTER_001", "SLAVE_001", 0)
            .await
            .unwrap(); // STATUS_DISABLED = 0
    }

    // Process trade signal (should be filtered out, no panic)
    handler.handle_trade_signal(signal).await;
}
