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
}
