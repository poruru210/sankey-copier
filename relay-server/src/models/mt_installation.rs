use serde::{Deserialize, Serialize};

/// MT4/MT5のタイプ
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum MtType {
    MT4,
    MT5,
}

/// アーキテクチャビット数
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Architecture {
    #[serde(rename = "32-bit")]
    Bit32,
    #[serde(rename = "64-bit")]
    Bit64,
}

/// インストールされたコンポーネント（実行に必要なもののみ）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InstalledComponents {
    pub dll: bool,
    pub master_ea: bool,
    pub slave_ea: bool,
}

/// MT4/MT5インストール情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MtInstallation {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub mt_type: MtType,
    pub platform: Architecture,
    pub path: String,
    pub executable: String,
    pub version: Option<String>, // DLLバージョン = クライアントバージョン
    pub components: InstalledComponents,
}

/// 検出サマリー
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionSummary {
    pub total_found: usize,
}

/// MT4/MT5検出結果のレスポンス
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MtInstallationsResponse {
    pub success: bool,
    pub data: Vec<MtInstallation>,
    pub detection_summary: DetectionSummary,
}

impl MtInstallation {
    /// インストールIDを生成
    pub fn generate_id(mt_type: &MtType, path: &str) -> String {
        // パスから識別可能なIDを生成
        let mut path_hash = path.to_lowercase().replace(['\\', '/', ' ', ':'], "-");

        // 連続したハイフンを単一のハイフンに置き換え
        while path_hash.contains("--") {
            path_hash = path_hash.replace("--", "-");
        }

        // 前後のハイフンを削除
        let path_hash = path_hash.trim_matches('-').to_string();

        let type_prefix = match mt_type {
            MtType::MT4 => "mt4",
            MtType::MT5 => "mt5",
        };

        format!("{}-{}", type_prefix, path_hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_id() {
        let id = MtInstallation::generate_id(&MtType::MT4, "D:\\Trading\\IC Markets MT4");
        assert_eq!(id, "mt4-d-trading-ic-markets-mt4");

        let id = MtInstallation::generate_id(&MtType::MT5, "C:\\Program Files\\XM MetaTrader 5");
        assert_eq!(id, "mt5-c-program-files-xm-metatrader-5");
    }
}
