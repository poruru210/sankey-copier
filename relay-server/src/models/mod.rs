mod connection;
mod mt_installation;

pub use connection::*;
pub use mt_installation::*;

// Re-export shared types from DLL
pub use sankey_copier_zmq::{SymbolMapping, TradeFilters};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderType {
    Buy,
    Sell,
    BuyLimit,
    SellLimit,
    BuyStop,
    SellStop,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TradeAction {
    Open,
    Close,
    Modify,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeSignal {
    pub action: TradeAction,
    pub ticket: i64,
    pub symbol: String,
    pub order_type: OrderType,
    pub lots: f64,
    pub open_price: f64,
    pub stop_loss: Option<f64>,
    pub take_profit: Option<f64>,
    pub magic_number: i32,
    pub comment: String,
    pub timestamp: DateTime<Utc>,
    pub source_account: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopySettings {
    pub id: i32,
    pub status: i32, // 0=DISABLED, 1=ENABLED (Master disconnected), 2=CONNECTED (Master connected)
    pub master_account: String,
    pub slave_account: String,
    pub lot_multiplier: Option<f64>,
    pub reverse_trade: bool,
    pub symbol_mappings: Vec<SymbolMapping>,
    pub filters: TradeFilters,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolConverter {
    pub prefix_remove: Option<String>,
    pub suffix_remove: Option<String>,
    pub prefix_add: Option<String>,
    pub suffix_add: Option<String>,
}

impl SymbolConverter {
    pub fn convert(&self, symbol: &str, mappings: &[SymbolMapping]) -> String {
        // Check for exact mapping first
        if let Some(mapping) = mappings.iter().find(|m| m.source_symbol == symbol) {
            return mapping.target_symbol.clone();
        }

        // Apply prefix/suffix transformations
        let mut result = symbol.to_string();

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

        let mappings = vec![SymbolMapping {
            source_symbol: "MT5_EURUSD".to_string(),
            target_symbol: "CUSTOM_EURUSD".to_string(),
        }];

        // Exact mapping should take priority over prefix/suffix rules
        let result = converter.convert("MT5_EURUSD", &mappings);
        assert_eq!(result, "CUSTOM_EURUSD");
    }
}
