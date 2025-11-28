// E2E tests for symbol transformation
// Tests that Relay Server correctly transforms symbols from Master to Slave

use sankey_copier_relay_server::models::{
    MasterSettings, OrderType, SlaveSettings, SymbolConverter, SymbolMapping, TradeAction,
    TradeFilters, TradeSignal,
};

#[test]
fn test_basic_transformation_with_mapping_and_suffix() {
    // Scenario: XAUUSDm → GOLD#
    // Master suffix 'm' removed, mapping XAUUSD→GOLD applied, Slave suffix '#' added

    let master_settings = MasterSettings {
        symbol_prefix: Some(String::new()),
        symbol_suffix: Some("m".to_string()),
        config_version: 0,
    };

    let slave_settings = SlaveSettings {
        symbol_prefix: Some(String::new()),
        symbol_suffix: Some("#".to_string()),
        symbol_mappings: vec![SymbolMapping {
            source_symbol: "XAUUSD".to_string(),
            target_symbol: "GOLD".to_string(),
        }],
        ..Default::default()
    };

    let converter = SymbolConverter {
        prefix_remove: master_settings.symbol_prefix.clone(),
        suffix_remove: master_settings.symbol_suffix.clone(),
        prefix_add: slave_settings.symbol_prefix.clone(),
        suffix_add: slave_settings.symbol_suffix.clone(),
    };

    let result = converter.convert("XAUUSDm", &slave_settings.symbol_mappings);
    assert_eq!(result, "GOLD#", "XAUUSDm should transform to GOLD#");
}

#[test]
fn test_no_mapping_with_suffix() {
    // Scenario: USDJPYm → USDJPY#
    // Master suffix 'm' removed, no mapping, Slave suffix '#' added

    let master_settings = MasterSettings {
        symbol_prefix: Some(String::new()),
        symbol_suffix: Some("m".to_string()),
        config_version: 0,
    };

    let slave_settings = SlaveSettings {
        symbol_prefix: Some(String::new()),
        symbol_suffix: Some("#".to_string()),
        symbol_mappings: vec![], // No mappings
        ..Default::default()
    };

    let converter = SymbolConverter {
        prefix_remove: master_settings.symbol_prefix.clone(),
        suffix_remove: master_settings.symbol_suffix.clone(),
        prefix_add: slave_settings.symbol_prefix.clone(),
        suffix_add: slave_settings.symbol_suffix.clone(),
    };

    let result = converter.convert("USDJPYm", &slave_settings.symbol_mappings);
    assert_eq!(result, "USDJPY#", "USDJPYm should transform to USDJPY#");
}

#[test]
fn test_prefix_transformation() {
    // Scenario: pro.EURUSD → FX.EURUSD.m
    // Master prefix 'pro.' removed, Slave prefix 'FX.' added, Slave suffix '.m' added

    let master_settings = MasterSettings {
        symbol_prefix: Some("pro.".to_string()),
        symbol_suffix: Some(String::new()),
        config_version: 0,
    };

    let slave_settings = SlaveSettings {
        symbol_prefix: Some("FX.".to_string()),
        symbol_suffix: Some(".m".to_string()),
        symbol_mappings: vec![],
        ..Default::default()
    };

    let converter = SymbolConverter {
        prefix_remove: master_settings.symbol_prefix.clone(),
        suffix_remove: master_settings.symbol_suffix.clone(),
        prefix_add: slave_settings.symbol_prefix.clone(),
        suffix_add: slave_settings.symbol_suffix.clone(),
    };

    let result = converter.convert("pro.EURUSD", &slave_settings.symbol_mappings);
    assert_eq!(
        result, "FX.EURUSD.m",
        "pro.EURUSD should transform to FX.EURUSD.m"
    );
}

#[test]
fn test_complex_mapping_with_suffix() {
    // Scenario: XAUUSD.raw → GOLD-ECN
    // Master suffix '.raw' removed, mapping XAUUSD→GOLD applied, Slave suffix '-ECN' added

    let master_settings = MasterSettings {
        symbol_prefix: Some(String::new()),
        symbol_suffix: Some(".raw".to_string()),
        config_version: 0,
    };

    let slave_settings = SlaveSettings {
        symbol_prefix: Some(String::new()),
        symbol_suffix: Some("-ECN".to_string()),
        symbol_mappings: vec![SymbolMapping {
            source_symbol: "XAUUSD".to_string(),
            target_symbol: "GOLD".to_string(),
        }],
        ..Default::default()
    };

    let converter = SymbolConverter {
        prefix_remove: master_settings.symbol_prefix.clone(),
        suffix_remove: master_settings.symbol_suffix.clone(),
        prefix_add: slave_settings.symbol_prefix.clone(),
        suffix_add: slave_settings.symbol_suffix.clone(),
    };

    let result = converter.convert("XAUUSD.raw", &slave_settings.symbol_mappings);
    assert_eq!(
        result, "GOLD-ECN",
        "XAUUSD.raw should transform to GOLD-ECN"
    );
}

#[test]
fn test_no_transformation_needed() {
    // Scenario: EURUSD → EURUSD
    // No prefix/suffix on either side, no mapping

    let master_settings = MasterSettings {
        symbol_prefix: Some(String::new()),
        symbol_suffix: Some(String::new()),
        config_version: 0,
    };

    let slave_settings = SlaveSettings {
        symbol_prefix: Some(String::new()),
        symbol_suffix: Some(String::new()),
        symbol_mappings: vec![],
        ..Default::default()
    };

    let converter = SymbolConverter {
        prefix_remove: master_settings.symbol_prefix.clone(),
        suffix_remove: master_settings.symbol_suffix.clone(),
        prefix_add: slave_settings.symbol_prefix.clone(),
        suffix_add: slave_settings.symbol_suffix.clone(),
    };

    let result = converter.convert("EURUSD", &slave_settings.symbol_mappings);
    assert_eq!(result, "EURUSD", "EURUSD should remain unchanged");
}

#[test]
fn test_trade_signal_transformation_integration() {
    // Integration test: Full trade signal transformation

    let signal = TradeSignal {
        action: TradeAction::Open,
        ticket: 123456,
        symbol: Some("XAUUSDm".to_string()),
        order_type: Some(OrderType::Buy),
        lots: Some(0.1),
        open_price: Some(2650.0),
        stop_loss: None,
        take_profit: None,
        magic_number: Some(0),
        comment: None,
        timestamp: chrono::Utc::now(),
        source_account: "master_account".to_string(),
        close_ratio: None,
    };

    let master_settings = MasterSettings {
        symbol_prefix: Some(String::new()),
        symbol_suffix: Some("m".to_string()),
        config_version: 0,
    };

    let slave_settings = SlaveSettings {
        symbol_prefix: Some(String::new()),
        symbol_suffix: Some("#".to_string()),
        symbol_mappings: vec![SymbolMapping {
            source_symbol: "XAUUSD".to_string(),
            target_symbol: "GOLD".to_string(),
        }],
        lot_multiplier: Some(1.0),
        reverse_trade: false,
        filters: TradeFilters::default(),
        ..Default::default()
    };

    let converter = SymbolConverter {
        prefix_remove: master_settings.symbol_prefix.clone(),
        suffix_remove: master_settings.symbol_suffix.clone(),
        prefix_add: slave_settings.symbol_prefix.clone(),
        suffix_add: slave_settings.symbol_suffix.clone(),
    };

    let transformed_symbol = converter.convert(
        signal.symbol.as_ref().unwrap(),
        &slave_settings.symbol_mappings,
    );

    assert_eq!(
        transformed_symbol, "GOLD#",
        "Trade signal symbol should transform from XAUUSDm to GOLD#"
    );
}
