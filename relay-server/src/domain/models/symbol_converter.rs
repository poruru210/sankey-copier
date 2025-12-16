use crate::domain::models::{MasterSettings, SlaveSettings, SymbolMapping};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolConverter {
    pub prefix_remove: Option<String>,
    pub suffix_remove: Option<String>,
    pub prefix_add: Option<String>,
    pub suffix_add: Option<String>,
}

impl SymbolConverter {
    pub fn from_settings(master_settings: &MasterSettings, slave_settings: &SlaveSettings) -> Self {
        Self {
            prefix_remove: master_settings.symbol_prefix.clone(),
            suffix_remove: master_settings.symbol_suffix.clone(),
            prefix_add: slave_settings.symbol_prefix.clone(),
            suffix_add: slave_settings.symbol_suffix.clone(),
        }
    }

    pub fn convert(&self, symbol: &str, mappings: &[SymbolMapping]) -> String {
        let mut result = symbol.to_string();

        // 1. Remove Master's prefix/suffix
        if let Some(prefix) = &self.prefix_remove {
            result = result
                .strip_prefix(prefix.as_str())
                .unwrap_or(&result)
                .to_string();
        }

        if let Some(suffix) = &self.suffix_remove {
            result = result
                .strip_suffix(suffix.as_str())
                .unwrap_or(&result)
                .to_string();
        }

        // 2. Apply Mapping (on the clean symbol)
        if let Some(mapping) = mappings.iter().find(|m| m.source_symbol == result) {
            result = mapping.target_symbol.clone();
        }

        // 3. Add Slave's prefix/suffix
        if let Some(prefix) = &self.prefix_add {
            result = format!("{}{}", prefix, result);
        }

        if let Some(suffix) = &self.suffix_add {
            result = format!("{}{}", result, suffix);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_converter_exact_mapping() {
        let converter = SymbolConverter {
            prefix_remove: None,
            suffix_remove: None,
            prefix_add: None,
            suffix_add: None,
        };

        let mappings = vec![SymbolMapping {
            source_symbol: "EURUSD".to_string(),
            target_symbol: "EURUSD.fx".to_string(),
        }];

        let result = converter.convert("EURUSD", &mappings);
        assert_eq!(result, "EURUSD.fx");
    }

    #[test]
    fn test_symbol_converter_prefix_remove() {
        let converter = SymbolConverter {
            prefix_remove: Some("MT5_".to_string()),
            suffix_remove: None,
            prefix_add: None,
            suffix_add: None,
        };

        let result = converter.convert("MT5_EURUSD", &[]);
        assert_eq!(result, "EURUSD");
    }

    #[test]
    fn test_symbol_converter_suffix_remove() {
        let converter = SymbolConverter {
            prefix_remove: None,
            suffix_remove: Some(".fx".to_string()),
            prefix_add: None,
            suffix_add: None,
        };

        let result = converter.convert("EURUSD.fx", &[]);
        assert_eq!(result, "EURUSD");
    }

    #[test]
    fn test_symbol_converter_prefix_add() {
        let converter = SymbolConverter {
            prefix_remove: None,
            suffix_remove: None,
            prefix_add: Some("FX_".to_string()),
            suffix_add: None,
        };

        let result = converter.convert("EURUSD", &[]);
        assert_eq!(result, "FX_EURUSD");
    }

    #[test]
    fn test_symbol_converter_suffix_add() {
        let converter = SymbolConverter {
            prefix_remove: None,
            suffix_remove: None,
            prefix_add: None,
            suffix_add: Some(".pro".to_string()),
        };

        let result = converter.convert("EURUSD", &[]);
        assert_eq!(result, "EURUSD.pro");
    }

    #[test]
    fn test_symbol_converter_combined() {
        let converter = SymbolConverter {
            prefix_remove: Some("MT5_".to_string()),
            suffix_remove: Some(".fx".to_string()),
            prefix_add: Some("FX_".to_string()),
            suffix_add: Some(".pro".to_string()),
        };

        let result = converter.convert("MT5_EURUSD.fx", &[]);
        assert_eq!(result, "FX_EURUSD.pro");
    }

    #[test]
    fn test_symbol_converter_mapping_priority() {
        let converter = SymbolConverter {
            prefix_remove: Some("MT5_".to_string()),
            suffix_remove: None,
            prefix_add: None,
            suffix_add: None,
        };

        // Mapping should match the CLEANED symbol
        let mappings = vec![SymbolMapping {
            source_symbol: "EURUSD".to_string(),
            target_symbol: "CUSTOM_EURUSD".to_string(),
        }];

        // 1. Remove MT5_ -> EURUSD
        // 2. Map EURUSD -> CUSTOM_EURUSD
        let result = converter.convert("MT5_EURUSD", &mappings);
        assert_eq!(result, "CUSTOM_EURUSD");
    }
    #[test]
    fn test_symbol_converter_from_settings() {
        let master_settings = MasterSettings {
            symbol_prefix: Some("M_".to_string()),
            symbol_suffix: Some(".m".to_string()),
            ..Default::default()
        };

        let slave_settings = SlaveSettings {
            symbol_prefix: Some("S_".to_string()),
            symbol_suffix: Some(".s".to_string()),
            ..Default::default()
        };

        let converter = SymbolConverter::from_settings(&master_settings, &slave_settings);

        assert_eq!(converter.prefix_remove, Some("M_".to_string()));
        assert_eq!(converter.suffix_remove, Some(".m".to_string()));
        assert_eq!(converter.prefix_add, Some("S_".to_string()));
        assert_eq!(converter.suffix_add, Some(".s".to_string()));
    }

    #[test]
    fn test_symbol_converter_defaults() {
        let master_settings = MasterSettings::default();
        let slave_settings = SlaveSettings::default();

        let converter = SymbolConverter::from_settings(&master_settings, &slave_settings);

        assert_eq!(converter.prefix_remove, None);
        assert_eq!(converter.suffix_remove, None);
        assert_eq!(converter.prefix_add, None);
        assert_eq!(converter.suffix_add, None);
    }

    #[test]
    fn test_complex_mapping_with_suffix() {
        // Scenario: XAUUSD.raw → GOLD-ECN
        let converter = SymbolConverter {
            prefix_remove: None,
            suffix_remove: Some(".raw".to_string()),
            prefix_add: None,
            suffix_add: Some("-ECN".to_string()),
        };

        let mappings = vec![SymbolMapping {
            source_symbol: "XAUUSD".to_string(),
            target_symbol: "GOLD".to_string(),
        }];

        let result = converter.convert("XAUUSD.raw", &mappings);
        assert_eq!(
            result, "GOLD-ECN",
            "XAUUSD.raw should transform to GOLD-ECN"
        );
    }

    #[test]
    fn test_trade_signal_transformation_integration() {
        use crate::domain::models::{OrderType, TradeAction, TradeFilters, TradeSignal};

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
            prefix_remove: None,
            suffix_remove: Some("m".to_string()), // Master settings would provide this
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
}
