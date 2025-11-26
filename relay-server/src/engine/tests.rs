use super::*;
use crate::models::*;
use chrono::Utc;

fn create_test_signal() -> TradeSignal {
    TradeSignal {
        action: TradeAction::Open,
        ticket: 12345,
        symbol: "EURUSD".to_string(),
        order_type: OrderType::Buy,
        lots: 0.1,
        open_price: 1.1000,
        stop_loss: Some(1.0950),
        take_profit: Some(1.1050),
        magic_number: 0,
        comment: "Test trade".to_string(),
        timestamp: Utc::now(),
        source_account: "MASTER_001".to_string(),
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
    assert_eq!(result.lots, 0.2);
    assert_eq!(result.symbol, "EURUSD");
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
    assert!(matches!(result.order_type, OrderType::Sell));
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
