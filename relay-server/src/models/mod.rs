mod connection;
mod global_settings;
mod mt_installation;
pub mod status_engine;
mod trade_group;
mod trade_group_member;

pub use connection::*;
pub use global_settings::*;
pub use mt_installation::*;
pub use trade_group::*;
pub use trade_group_member::*;

// Re-export shared types from DLL
pub use sankey_copier_zmq::{
    OrderType, SymbolMapping, TradeAction, TradeFilters, WarningCode, STATUS_CONNECTED,
    STATUS_DISABLED, STATUS_ENABLED, STATUS_NO_CONFIG,
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Slave configuration with associated Master account information.
/// Used for config distribution to Slave EAs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlaveConfigWithMaster {
    pub master_account: String,
    pub slave_account: String,
    #[serde(default)]
    pub status: i32,
    #[serde(default)]
    pub enabled_flag: bool,
    /// Detailed warning codes from the Status Engine (empty when healthy)
    #[serde(default)]
    pub warning_codes: Vec<WarningCode>,
    pub slave_settings: SlaveSettings,
}

/// Trade signal message structure
/// Note: Some fields are optional because Close/Modify actions may not include all data.
/// The mt-bridge serializer sends None for fields not applicable to the action type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeSignal {
    pub action: TradeAction,
    pub ticket: i64,
    #[serde(default)]
    pub symbol: Option<String>,
    #[serde(default)]
    pub order_type: Option<OrderType>,
    #[serde(default)]
    pub lots: Option<f64>,
    #[serde(default)]
    pub open_price: Option<f64>,
    #[serde(default)]
    pub stop_loss: Option<f64>,
    #[serde(default)]
    pub take_profit: Option<f64>,
    #[serde(default)]
    pub magic_number: Option<i32>,
    #[serde(default)]
    pub comment: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub source_account: String,
    /// Close ratio for partial close (0.0-1.0)
    /// None or 1.0 = full close, 0.0 < ratio < 1.0 = partial close
    #[serde(default)]
    pub close_ratio: Option<f64>,
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
}
