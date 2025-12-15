#[cfg(test)]
mod tests {
    use crate::domain::models::{MasterSettings, SlaveSettings, SymbolConverter};

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
}
