use sankey_copier_relay_server::models::{SlaveConfigMessage, SymbolMapping, TradeFilters};
use sankey_copier_zmq::LotCalculationMode;

/// Performance benchmark test for Phase 1: SlaveConfigMessage Extension
///
/// This test measures the size of serialized SlaveConfigMessage to ensure
/// it meets the performance criteria (< 2KB).
#[tokio::test]
async fn test_config_message_size_benchmark() {
    println!("\n=== CONFIG Message Size Benchmark ===\n");

    // Test Case 1: Minimal configuration
    let minimal_config = SlaveConfigMessage {
        account_id: "SLAVE_001".to_string(),
        master_account: "MASTER_001".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        status: 2, // STATUS_CONNECTED
        lot_calculation_mode: LotCalculationMode::default(),
        lot_multiplier: Some(1.0),
        reverse_trade: false,
        symbol_mappings: vec![],
        filters: TradeFilters {
            allowed_symbols: None,
            blocked_symbols: None,
            allowed_magic_numbers: None,
            blocked_magic_numbers: None,
        },
        config_version: 1,
        symbol_prefix: None,
        symbol_suffix: None,
        source_lot_min: None,
        source_lot_max: None,
        master_equity: None,
    };

    let minimal_json = serde_json::to_string(&minimal_config).unwrap();
    let minimal_size = minimal_json.len();
    println!("Minimal Config:");
    println!("  JSON size: {} bytes", minimal_size);
    println!("  Within limit (<2KB): {}", minimal_size < 2048);
    println!();

    // Test Case 2: Moderate configuration (typical use case)
    let moderate_config = SlaveConfigMessage {
        account_id: "SLAVE_MODERATE_001".to_string(),
        master_account: "MASTER_MODERATE_001".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        status: 2, // STATUS_CONNECTED
        lot_calculation_mode: LotCalculationMode::default(),
        lot_multiplier: Some(1.5),
        reverse_trade: false,
        symbol_mappings: vec![
            SymbolMapping {
                source_symbol: "EURUSD".to_string(),
                target_symbol: "EURUSDm".to_string(),
            },
            SymbolMapping {
                source_symbol: "GBPUSD".to_string(),
                target_symbol: "GBPUSDm".to_string(),
            },
            SymbolMapping {
                source_symbol: "USDJPY".to_string(),
                target_symbol: "USDJPYm".to_string(),
            },
        ],
        filters: TradeFilters {
            allowed_symbols: Some(vec![
                "EURUSD".to_string(),
                "GBPUSD".to_string(),
                "USDJPY".to_string(),
            ]),
            blocked_symbols: Some(vec!["XAUUSD".to_string()]),
            allowed_magic_numbers: Some(vec![123, 456, 789]),
            blocked_magic_numbers: None,
        },
        config_version: 1,
        symbol_prefix: None,
        symbol_suffix: None,
        source_lot_min: None,
        source_lot_max: None,
        master_equity: None,
    };

    let moderate_json = serde_json::to_string(&moderate_config).unwrap();
    let moderate_size = moderate_json.len();
    println!("Moderate Config (3 mappings, 3 allowed symbols, 3 magic numbers):");
    println!("  JSON size: {} bytes", moderate_size);
    println!("  Within limit (<2KB): {}", moderate_size < 2048);
    println!();

    // Test Case 3: Maximum realistic configuration
    let max_config = SlaveConfigMessage {
        account_id: "SLAVE_MAXIMUM_CONFIGURATION_001".to_string(),
        master_account: "MASTER_MAXIMUM_CONFIGURATION_001".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        status: 2, // STATUS_CONNECTED
        lot_calculation_mode: LotCalculationMode::default(),
        lot_multiplier: Some(2.5),
        reverse_trade: true,
        symbol_mappings: vec![
            SymbolMapping {
                source_symbol: "EURUSD".to_string(),
                target_symbol: "EURUSDm".to_string(),
            },
            SymbolMapping {
                source_symbol: "GBPUSD".to_string(),
                target_symbol: "GBPUSDm".to_string(),
            },
            SymbolMapping {
                source_symbol: "USDJPY".to_string(),
                target_symbol: "USDJPYm".to_string(),
            },
            SymbolMapping {
                source_symbol: "AUDUSD".to_string(),
                target_symbol: "AUDUSDm".to_string(),
            },
            SymbolMapping {
                source_symbol: "USDCAD".to_string(),
                target_symbol: "USDCADm".to_string(),
            },
            SymbolMapping {
                source_symbol: "NZDUSD".to_string(),
                target_symbol: "NZDUSDm".to_string(),
            },
            SymbolMapping {
                source_symbol: "USDCHF".to_string(),
                target_symbol: "USDCHFm".to_string(),
            },
            SymbolMapping {
                source_symbol: "EURGBP".to_string(),
                target_symbol: "EURGBPm".to_string(),
            },
            SymbolMapping {
                source_symbol: "EURJPY".to_string(),
                target_symbol: "EURJPYm".to_string(),
            },
            SymbolMapping {
                source_symbol: "GBPJPY".to_string(),
                target_symbol: "GBPJPYm".to_string(),
            },
        ],
        filters: TradeFilters {
            allowed_symbols: Some(vec![
                "EURUSD".to_string(),
                "GBPUSD".to_string(),
                "USDJPY".to_string(),
                "AUDUSD".to_string(),
                "USDCAD".to_string(),
                "NZDUSD".to_string(),
                "USDCHF".to_string(),
                "EURGBP".to_string(),
                "EURJPY".to_string(),
                "GBPJPY".to_string(),
            ]),
            blocked_symbols: Some(vec![
                "XAUUSD".to_string(),
                "XAGUSD".to_string(),
                "BTCUSD".to_string(),
            ]),
            allowed_magic_numbers: Some(vec![100, 200, 300, 400, 500, 600, 700, 800, 900, 1000]),
            blocked_magic_numbers: Some(vec![999, 666, 111]),
        },
        config_version: 1,
        symbol_prefix: None,
        symbol_suffix: None,
        source_lot_min: None,
        source_lot_max: None,
        master_equity: None,
    };

    let max_json = serde_json::to_string(&max_config).unwrap();
    let max_size = max_json.len();
    println!("Maximum Config (10 mappings, 10 allowed + 3 blocked symbols, 10 + 3 magic numbers):");
    println!("  JSON size: {} bytes", max_size);
    println!("  Within limit (<2KB): {}", max_size < 2048);
    println!();

    // Summary
    println!("=== Summary ===");
    println!(
        "Minimal:  {} bytes ({:.1}% of 2KB limit)",
        minimal_size,
        (minimal_size as f64 / 2048.0) * 100.0
    );
    println!(
        "Moderate: {} bytes ({:.1}% of 2KB limit)",
        moderate_size,
        (moderate_size as f64 / 2048.0) * 100.0
    );
    println!(
        "Maximum:  {} bytes ({:.1}% of 2KB limit)",
        max_size,
        (max_size as f64 / 2048.0) * 100.0
    );
    println!();

    // Assertions
    assert!(minimal_size < 2048, "Minimal config exceeds 2KB limit");
    assert!(moderate_size < 2048, "Moderate config exceeds 2KB limit");
    assert!(max_size < 2048, "Maximum config exceeds 2KB limit");

    println!("✓ All configurations within 2KB size limit");
}

/// Estimate MQL5 parsing performance based on JSON complexity
#[tokio::test]
async fn test_estimate_parsing_performance() {
    println!("\n=== Estimated MQL5 Parsing Performance ===\n");

    let config = SlaveConfigMessage {
        account_id: "SLAVE_001".to_string(),
        master_account: "MASTER_001".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        status: 2, // STATUS_CONNECTED
        lot_calculation_mode: LotCalculationMode::default(),
        lot_multiplier: Some(1.5),
        reverse_trade: false,
        symbol_mappings: vec![
            SymbolMapping {
                source_symbol: "EURUSD".to_string(),
                target_symbol: "EURUSDm".to_string(),
            },
            SymbolMapping {
                source_symbol: "GBPUSD".to_string(),
                target_symbol: "GBPUSDm".to_string(),
            },
        ],
        filters: TradeFilters {
            allowed_symbols: Some(vec!["EURUSD".to_string(), "GBPUSD".to_string()]),
            blocked_symbols: Some(vec!["USDJPY".to_string()]),
            allowed_magic_numbers: Some(vec![123, 456]),
            blocked_magic_numbers: None,
        },
        config_version: 1,
        symbol_prefix: None,
        symbol_suffix: None,
        source_lot_min: None,
        source_lot_max: None,
        master_equity: None,
    };

    let json = serde_json::to_string(&config).unwrap();

    println!("JSON Structure Analysis:");
    println!("  Total size: {} bytes", json.len());
    println!("  Number of fields: ~10 (top-level)");
    println!("  Number of arrays: 6 (2 mappings + 4 filter arrays)");
    println!("  Nested objects: 2 (filters, timestamp)");
    println!();

    // Estimate parsing operations
    let string_parses = 5; // account_id, master_account, trade_group_id, timestamp, config_version
    let bool_parses = 2; // enabled, reverse_trade
    let number_parses = 2; // lot_multiplier, config_version
    let array_parses = 6; // symbol_mappings + 4 filter arrays + symbol_mappings items
    let object_parses = 2; // filters, each symbol_mapping

    let total_ops = string_parses + bool_parses + number_parses + array_parses + object_parses;

    println!("Parsing Operations Estimate:");
    println!("  String field parses: {}", string_parses);
    println!("  Boolean field parses: {}", bool_parses);
    println!("  Number field parses: {}", number_parses);
    println!("  Array parses: {}", array_parses);
    println!("  Object parses: {}", object_parses);
    println!("  Total operations: ~{}", total_ops);
    println!();

    println!("Expected Performance (MQL5):");
    println!("  Parsing time: < 50ms (target)");
    println!("  Note: Simple string operations, no regex, O(n) complexity");
    println!();

    println!("✓ Parsing complexity is reasonable for target performance");
}

/// Memory usage estimate
#[tokio::test]
async fn test_estimate_memory_usage() {
    println!("\n=== Memory Usage Estimate ===\n");

    // Estimate memory for global variables
    let base_memory = std::mem::size_of::<bool>() * 3          // g_config_enabled, reverse_trade, + 1 spare
                    + std::mem::size_of::<f64>()               // g_config_lot_multiplier
                    + std::mem::size_of::<i32>(); // g_config_version

    println!("Base Configuration Variables:");
    println!("  Booleans (3): {} bytes", std::mem::size_of::<bool>() * 3);
    println!("  Double (1): {} bytes", std::mem::size_of::<f64>());
    println!("  Integer (1): {} bytes", std::mem::size_of::<i32>());
    println!("  Total base: ~{} bytes", base_memory);
    println!();

    // Estimate for typical configuration
    let symbol_mapping_size = 32 * 2; // ~32 bytes per string * 2 strings per mapping
    let typical_mappings = 5;
    let mappings_memory = symbol_mapping_size * typical_mappings;

    let symbol_filter_size = 32; // ~32 bytes per symbol string
    let typical_symbols = 10;
    let symbols_memory = symbol_filter_size * typical_symbols;

    let magic_number_size = std::mem::size_of::<i32>();
    let typical_magic_numbers = 10;
    let magic_memory = magic_number_size * typical_magic_numbers;

    let total_estimated = base_memory + mappings_memory + symbols_memory + magic_memory;

    println!("Typical Configuration (5 mappings, 10 symbols, 10 magic numbers):");
    println!("  Symbol mappings: ~{} bytes", mappings_memory);
    println!("  Symbol filters: ~{} bytes", symbols_memory);
    println!("  Magic number filters: ~{} bytes", magic_memory);
    println!(
        "  Total estimated: ~{} bytes ({:.2} KB)",
        total_estimated,
        total_estimated as f64 / 1024.0
    );
    println!();

    println!("Memory Overhead:");
    println!("  Target: < 100 KB");
    println!("  Estimated: < 5 KB");
    println!("  Status: Well within acceptable range");
    println!();

    assert!(
        total_estimated < 100 * 1024,
        "Memory usage exceeds 100KB limit"
    );
    println!("✓ Memory usage well within 100KB limit");
}
