use super::*;
use crate::models::*;
use chrono::Utc;

// =============================================================================
// Test Fixtures
// =============================================================================

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
        slave_settings: SlaveSettings::default(),
        status: 2, // STATUS_CONNECTED
        runtime_status: 2,
        warning_codes: Vec::new(),
        enabled_flag: true,
        created_at: Utc::now().to_rfc3339(),
        updated_at: Utc::now().to_rfc3339(),
    }
}

fn create_converter() -> SymbolConverter {
    SymbolConverter {
        prefix_remove: None,
        suffix_remove: None,
        prefix_add: None,
        suffix_add: None,
    }
}

// =============================================================================
// Filter Tests: Connection Status
// =============================================================================

#[test]
fn test_filter_connected_member_allowed() {
    let engine = CopyEngine::new();
    let signal = create_test_signal();
    let member = create_test_member(); // status = 2 (CONNECTED)

    assert!(engine.should_copy_trade(&signal, &member));
}

#[test]
fn test_filter_disabled_member_blocked() {
    let engine = CopyEngine::new();
    let signal = create_test_signal();
    let mut member = create_test_member();
    member.status = 0; // STATUS_DISABLED

    assert!(!engine.should_copy_trade(&signal, &member));
}

// =============================================================================
// Filter Tests: Symbol Filters
// =============================================================================

#[test]
fn test_filter_symbol_in_allowed_list() {
    let engine = CopyEngine::new();
    let signal = create_test_signal(); // EURUSD
    let mut member = create_test_member();
    member.slave_settings.filters.allowed_symbols =
        Some(vec!["EURUSD".to_string(), "GBPUSD".to_string()]);

    assert!(engine.should_copy_trade(&signal, &member));
}

#[test]
fn test_filter_symbol_not_in_allowed_list() {
    let engine = CopyEngine::new();
    let signal = create_test_signal(); // EURUSD
    let mut member = create_test_member();
    member.slave_settings.filters.allowed_symbols = Some(vec!["GBPUSD".to_string()]);

    assert!(!engine.should_copy_trade(&signal, &member));
}

#[test]
fn test_filter_symbol_in_blocked_list() {
    let engine = CopyEngine::new();
    let signal = create_test_signal(); // EURUSD
    let mut member = create_test_member();
    member.slave_settings.filters.blocked_symbols = Some(vec!["EURUSD".to_string()]);

    assert!(!engine.should_copy_trade(&signal, &member));
}

// =============================================================================
// Filter Tests: Magic Number Filters
// =============================================================================

#[test]
fn test_filter_magic_in_allowed_list() {
    let engine = CopyEngine::new();
    let signal = create_test_signal(); // magic = 0
    let mut member = create_test_member();
    member.slave_settings.filters.allowed_magic_numbers = Some(vec![0, 123]);

    assert!(engine.should_copy_trade(&signal, &member));
}

#[test]
fn test_filter_magic_not_in_allowed_list() {
    let engine = CopyEngine::new();
    let signal = create_test_signal(); // magic = 0
    let mut member = create_test_member();
    member.slave_settings.filters.allowed_magic_numbers = Some(vec![123, 456]);

    assert!(!engine.should_copy_trade(&signal, &member));
}

#[test]
fn test_filter_magic_in_blocked_list() {
    let engine = CopyEngine::new();
    let signal = create_test_signal(); // magic = 0
    let mut member = create_test_member();
    member.slave_settings.filters.blocked_magic_numbers = Some(vec![0]);

    assert!(!engine.should_copy_trade(&signal, &member));
}

// =============================================================================
// Filter Tests: Source Lot Range
// =============================================================================

#[test]
fn test_filter_lots_within_range() {
    let engine = CopyEngine::new();
    let signal = create_test_signal(); // lots = 0.1
    let mut member = create_test_member();
    member.slave_settings.source_lot_min = Some(0.05);
    member.slave_settings.source_lot_max = Some(1.0);

    assert!(engine.should_copy_trade(&signal, &member));
}

#[test]
fn test_filter_lots_below_min() {
    let engine = CopyEngine::new();
    let signal = create_test_signal(); // lots = 0.1
    let mut member = create_test_member();
    member.slave_settings.source_lot_min = Some(0.5);

    assert!(!engine.should_copy_trade(&signal, &member));
}

#[test]
fn test_filter_lots_above_max() {
    let engine = CopyEngine::new();
    let signal = create_test_signal(); // lots = 0.1
    let mut member = create_test_member();
    member.slave_settings.source_lot_max = Some(0.05);

    assert!(!engine.should_copy_trade(&signal, &member));
}

#[test]
fn test_filter_lots_not_applied_to_close_signals() {
    let engine = CopyEngine::new();
    let mut signal = create_test_signal();
    signal.action = TradeAction::Close;
    signal.lots = Some(0.1);
    let mut member = create_test_member();
    member.slave_settings.source_lot_min = Some(0.5); // Would reject if Open

    assert!(engine.should_copy_trade(&signal, &member));
}

// =============================================================================
// Filter Tests: Pending Orders
// =============================================================================

#[test]
fn test_filter_pending_order_allowed() {
    let engine = CopyEngine::new();
    let mut signal = create_test_signal();
    signal.order_type = Some(OrderType::BuyLimit);
    let mut member = create_test_member();
    member.slave_settings.copy_pending_orders = true;

    assert!(engine.should_copy_trade(&signal, &member));
}

#[test]
fn test_filter_pending_order_blocked() {
    let engine = CopyEngine::new();
    let mut signal = create_test_signal();
    signal.order_type = Some(OrderType::BuyLimit);
    let member = create_test_member(); // copy_pending_orders = false by default

    assert!(!engine.should_copy_trade(&signal, &member));
}

#[test]
fn test_filter_market_order_always_allowed() {
    let engine = CopyEngine::new();
    let signal = create_test_signal(); // Buy (market order)
    let member = create_test_member(); // copy_pending_orders = false

    assert!(engine.should_copy_trade(&signal, &member));
}

#[test]
fn test_filter_all_pending_types_blocked_when_disabled() {
    let engine = CopyEngine::new();
    let member = create_test_member(); // copy_pending_orders = false

    for order_type in [
        OrderType::BuyLimit,
        OrderType::SellLimit,
        OrderType::BuyStop,
        OrderType::SellStop,
    ] {
        let mut signal = create_test_signal();
        signal.order_type = Some(order_type.clone());
        assert!(
            !engine.should_copy_trade(&signal, &member),
            "{:?} should be blocked",
            order_type
        );
    }
}

// =============================================================================
// Transform Tests: Lot Passthrough (Slave EA handles calculation)
// =============================================================================

#[test]
fn test_transform_lots_unchanged_for_open() {
    let engine = CopyEngine::new();
    let signal = create_test_signal(); // lots = 0.1
    let mut member = create_test_member();
    member.slave_settings.lot_multiplier = Some(2.0); // Ignored

    let result = engine
        .transform_signal(signal, &member, &create_converter())
        .unwrap();

    assert_eq!(result.lots, Some(0.1));
}

#[test]
fn test_transform_lots_unchanged_for_close() {
    let engine = CopyEngine::new();
    let mut signal = create_test_signal();
    signal.action = TradeAction::Close;
    signal.lots = Some(1.0);
    signal.close_ratio = Some(0.5);
    let mut member = create_test_member();
    member.slave_settings.lot_multiplier = Some(2.0); // Ignored

    let result = engine
        .transform_signal(signal, &member, &create_converter())
        .unwrap();

    assert_eq!(result.lots, Some(1.0));
    assert_eq!(result.close_ratio, Some(0.5));
}

// =============================================================================
// Transform Tests: Order Type Passthrough (Slave EA handles reversal)
// =============================================================================

#[test]
fn test_transform_order_type_unchanged() {
    let engine = CopyEngine::new();
    let signal = create_test_signal(); // Buy
    let mut member = create_test_member();
    member.slave_settings.reverse_trade = true; // Ignored

    let result = engine
        .transform_signal(signal, &member, &create_converter())
        .unwrap();

    assert!(matches!(result.order_type, Some(OrderType::Buy)));
}

#[test]
fn test_transform_close_ratio_preserved() {
    let engine = CopyEngine::new();
    let mut signal = create_test_signal();
    signal.action = TradeAction::Close;
    signal.close_ratio = Some(0.5);
    let member = create_test_member();

    let result = engine
        .transform_signal(signal, &member, &create_converter())
        .unwrap();

    assert_eq!(result.close_ratio, Some(0.5));
}

// =============================================================================
// Transform Tests: Symbol Conversion
// =============================================================================

#[test]
fn test_transform_symbol_mapping() {
    let engine = CopyEngine::new();
    let signal = create_test_signal(); // EURUSD
    let mut member = create_test_member();
    member.slave_settings.symbol_mappings = vec![SymbolMapping {
        source_symbol: "EURUSD".to_string(),
        target_symbol: "EURUSD.fx".to_string(),
    }];

    let result = engine
        .transform_signal(signal, &member, &create_converter())
        .unwrap();

    assert_eq!(result.symbol.as_deref(), Some("EURUSD.fx"));
}

#[test]
fn test_transform_symbol_prefix_suffix() {
    let engine = CopyEngine::new();
    let mut signal = create_test_signal();
    signal.symbol = Some("pro.EURUSD.m".to_string());
    let member = create_test_member();
    let converter = SymbolConverter {
        prefix_remove: Some("pro.".to_string()),
        suffix_remove: Some(".m".to_string()),
        prefix_add: Some("fx.".to_string()),
        suffix_add: Some(".micro".to_string()),
    };

    let result = engine
        .transform_signal(signal, &member, &converter)
        .unwrap();

    // pro.EURUSD.m -> EURUSD -> fx.EURUSD.micro
    assert_eq!(result.symbol.as_deref(), Some("fx.EURUSD.micro"));
}

#[test]
fn test_transform_no_symbol_handled_gracefully() {
    let engine = CopyEngine::new();
    let mut signal = create_test_signal();
    signal.symbol = None;
    let member = create_test_member();
    let converter = SymbolConverter {
        prefix_remove: None,
        suffix_remove: None,
        prefix_add: Some("fx.".to_string()),
        suffix_add: None,
    };

    let result = engine
        .transform_signal(signal, &member, &converter)
        .unwrap();

    assert_eq!(result.symbol, None);
}
