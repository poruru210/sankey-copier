// e2e-tests/src/ini_config.rs

use anyhow::{Context, Result};
use ini::Ini;
use std::path::Path;

/// Represents the content of sankey_copier.ini
#[derive(Debug, Clone, Default)]
pub struct EaIniConfig {
    pub receiver_port: u16,
    pub publisher_port: u16,
    pub symbol_search_candidates: Vec<String>,
}

impl EaIniConfig {
    /// Load and parse from file path
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conf = Ini::load_from_file(&path)
            .with_context(|| format!("Failed to read INI file: {:?}", path.as_ref()))?;

        let section_zmq = conf.section(Some("ZeroMQ"));
        let receiver_port = section_zmq
            .and_then(|s| s.get("ReceiverPort"))
            .unwrap_or("0")
            .parse()
            .unwrap_or(0);

        let publisher_port = section_zmq
            .and_then(|s| s.get("PublisherPort"))
            .unwrap_or("0")
            .parse()
            .unwrap_or(0);

        let section_symbol = conf.section(Some("SymbolSearch"));
        let candidates_str = section_symbol
            .and_then(|s| s.get("Candidates"))
            .unwrap_or("");

        let candidates: Vec<String> = candidates_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Ok(Self {
            receiver_port,
            publisher_port,
            symbol_search_candidates: candidates,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_ini_file() {
        let ini_content = r#"
[ZeroMQ]
ReceiverPort=5555
PublisherPort=5556

[SymbolSearch]
Candidates=GOLD,XAUUSD,BTCUSD
"#;
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", ini_content).unwrap();

        let config = EaIniConfig::load_from_file(file.path()).unwrap();
        assert_eq!(config.receiver_port, 5555);
        assert_eq!(config.publisher_port, 5556);
        assert_eq!(
            config.symbol_search_candidates,
            vec!["GOLD", "XAUUSD", "BTCUSD"]
        );
    }
}
