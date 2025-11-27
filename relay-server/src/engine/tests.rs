use super::*;
use crate::models::*;
use chrono::Utc;

fn create_test_signal() -> TradeSignal {
    TradeSignal {
        action: TradeAction::Open,
        ticket: 12345,
        symbol: Some("EURUSD".to_string()),
        order_type: Some(OrderType::Buy),
        lots: Some(0.1),
        open_price: Some(1.1000),
        stop_loss: Some(1.0950),
        take_profit: Some(1.1050),
        magic_number: Some(0),
        comment: Some("Test trade".to_string()),
        timestamp: Utc::now(),
        source_account: "MASTER_001".to_string(),
        close_ratio: None,
    }
}

fn create_test_member() -> TradeGroupMember {
    TradeGroupMember {
        id: 1,
        trade_group_id: "MASTER_001".to_string(),
        slave_account: "SLAVE_001".to_string(),
        slave_settings: SlaveSettings {
            lot_calculation_mode: LotCalculationMode::default(),
            lot_multiplier: Some(1.0),
            reverse_trade: false,
            symbol_prefix: None,
            symbol_suffix: None,
            symbol_mappings: vec![],
            filters: TradeFilters {
                allowed_symbols: None,
                blocked_symbols: None,
                allowed_magic_numbers: None,
                blocked_magic_numbers: None,
            },
            config_version: 0,
            source_lot_min: None,
            source_lot_max: None,
            sync_mode: crate::models::SyncMode::Skip,
            limit_order_expiry_min: None,
            market_sync_max_pips: None,
            max_slippage: None,
            copy_pending_orders: false,
        },
        status: 2, // STATUS_CONNECTED
        created_at: Utc::now().to_rfc3339(),
        updated_at: Utc::now().to_rfc3339(),
    }
}

fn create_test_master_settings() -> MasterSettings {
    MasterSettings {
        symbol_prefix: None,
        symbol_suffix: None,
        config_version: 0,
    }
}

#[test]
fn test_should_copy_trade_enabled() {
    let engine = CopyEngine::new();
    let signal = create_test_signal();
    let member = create_test_member();

    assert!(engine.should_copy_trade(&signal, &member));
}

#[test]
fn test_should_copy_trade_disabled() {
    let engine = CopyEngine::new();
    let signal = create_test_signal();
    let mut member = create_test_member();
    member.status = 0; // STATUS_DISABLED

    assert!(!engine.should_copy_trade(&signal, &member));
}

#[test]
fn test_should_copy_trade_allowed_symbols() {
    let engine = CopyEngine::new();
    let signal = create_test_signal();
    let mut member = create_test_member();
    member.slave_settings.filters.allowed_symbols =
        Some(vec!["EURUSD".to_string(), "GBPUSD".to_string()]);

    assert!(engine.should_copy_trade(&signal, &member));
}

#[test]
fn test_should_copy_trade_symbol_not_allowed() {
    let engine = CopyEngine::new();
    let signal = create_test_signal();
    let mut member = create_test_member();
    member.slave_settings.filters.allowed_symbols = Some(vec!["GBPUSD".to_string()]);

    assert!(!engine.should_copy_trade(&signal, &member));
}

#[test]
fn test_should_copy_trade_blocked_symbols() {
    let engine = CopyEngine::new();
    let signal = create_test_signal();
    let mut member = create_test_member();
    member.slave_settings.filters.blocked_symbols = Some(vec!["EURUSD".to_string()]);

    assert!(!engine.should_copy_trade(&signal, &member));
}

#[test]
fn test_should_copy_trade_allowed_magic_numbers() {
    let engine = CopyEngine::new();
    let signal = create_test_signal();
    let mut member = create_test_member();
    member.slave_settings.filters.allowed_magic_numbers = Some(vec![0, 123]);

    assert!(engine.should_copy_trade(&signal, &member));
}

#[test]
fn test_should_copy_trade_magic_not_allowed() {
    let engine = CopyEngine::new();
    let signal = create_test_signal();
    let mut member = create_test_member();
    member.slave_settings.filters.allowed_magic_numbers = Some(vec![123, 456]);

    assert!(!engine.should_copy_trade(&signal, &member));
}

#[test]
fn test_should_copy_trade_blocked_magic_numbers() {
    let engine = CopyEngine::new();
    let signal = create_test_signal();
    let mut member = create_test_member();
    member.slave_settings.filters.blocked_magic_numbers = Some(vec![0]);

    assert!(!engine.should_copy_trade(&signal, &member));
}

#[test]
fn test_transform_signal_lot_multiplier() {
    let engine = CopyEngine::new();
    let signal = create_test_signal();
    let mut member = create_test_member();
    member.slave_settings.lot_multiplier = Some(2.0);
    let master_settings = create_test_master_settings();

    let converter = SymbolConverter {
        prefix_remove: None,
        suffix_remove: None,
        prefix_add: None,
        suffix_add: None,
    };

    let result = engine
        .transform_signal(signal.clone(), &member, &master_settings, &converter)
        .unwrap();
    assert_eq!(result.lots, Some(0.2));
    assert_eq!(result.symbol.as_deref(), Some("EURUSD"));
}

#[test]
fn test_transform_signal_reverse_trade() {
    let engine = CopyEngine::new();
    let signal = create_test_signal();
    let mut member = create_test_member();
    member.slave_settings.reverse_trade = true;
    let master_settings = create_test_master_settings();

    let converter = SymbolConverter {
        prefix_remove: None,
        suffix_remove: None,
        prefix_add: None,
        suffix_add: None,
    };

    let result = engine
        .transform_signal(signal.clone(), &member, &master_settings, &converter)
        .unwrap();
    assert!(matches!(result.order_type, Some(OrderType::Sell)));
}

#[test]
fn test_reverse_order_type() {
    assert!(matches!(
        CopyEngine::reverse_order_type(&OrderType::Buy),
        OrderType::Sell
    ));
    assert!(matches!(
        CopyEngine::reverse_order_type(&OrderType::Sell),
        OrderType::Buy
    ));
    assert!(matches!(
        CopyEngine::reverse_order_type(&OrderType::BuyLimit),
        OrderType::SellLimit
    ));
    assert!(matches!(
        CopyEngine::reverse_order_type(&OrderType::SellLimit),
        OrderType::BuyLimit
    ));
    assert!(matches!(
        CopyEngine::reverse_order_type(&OrderType::BuyStop),
        OrderType::SellStop
    ));
    assert!(matches!(
        CopyEngine::reverse_order_type(&OrderType::SellStop),
        OrderType::BuyStop
    ));
}

#[test]
fn test_partial_close_signal_preserves_ratio() {
    // Test that close_ratio is preserved in Close signals
    let engine = CopyEngine::new();
    let mut signal = create_test_signal();
    signal.action = TradeAction::Close;
    signal.close_ratio = Some(0.5); // 50% partial close

    let master_settings = create_test_master_settings();
    let member = create_test_member();
    let converter = SymbolConverter {
        prefix_remove: None,
        suffix_remove: None,
        prefix_add: None,
        suffix_add: None,
    };

    let transformed = engine
        .transform_signal(signal.clone(), &member, &master_settings, &converter)
        .unwrap();

    // close_ratio should be preserved
    assert_eq!(transformed.close_ratio, Some(0.5));
}

#[test]
fn test_lot_multiplier_not_applied_to_close_signal() {
    // Test that lot multiplier is NOT applied to Close signals
    // (Close uses ratio-based calculation, not lot multiplier)
    let engine = CopyEngine::new();
    let mut signal = create_test_signal();
    signal.action = TradeAction::Close;
    signal.lots = Some(1.0);
    signal.close_ratio = Some(0.5);

    let master_settings = create_test_master_settings();
    let mut member = create_test_member();
    member.slave_settings.lot_multiplier = Some(2.0); // 2x multiplier
    let converter = SymbolConverter {
        prefix_remove: None,
        suffix_remove: None,
        prefix_add: None,
        suffix_add: None,
    };

    let transformed = engine
        .transform_signal(signal.clone(), &member, &master_settings, &converter)
        .unwrap();

    // Lots should NOT be multiplied for Close signals
    assert_eq!(transformed.lots, Some(1.0)); // Unchanged
    assert_eq!(transformed.close_ratio, Some(0.5)); // Preserved
}

#[test]
fn test_lot_multiplier_applied_to_open_signal() {
    // Verify lot multiplier IS applied to Open signals (using existing test pattern)
    let engine = CopyEngine::new();
    let signal = create_test_signal(); // action = Open, lots = 0.1

    let master_settings = create_test_master_settings();
    let mut member = create_test_member();
    member.slave_settings.lot_multiplier = Some(2.0); // 2x multiplier
    let converter = SymbolConverter {
        prefix_remove: None,
        suffix_remove: None,
        prefix_add: None,
        suffix_add: None,
    };

    let transformed = engine
        .transform_signal(signal.clone(), &member, &master_settings, &converter)
        .unwrap();

    // Lots should be multiplied for Open signals: 0.1 * 2.0 = 0.2
    assert_eq!(transformed.lots, Some(0.2));
}
