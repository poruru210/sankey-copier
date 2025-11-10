use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

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

/// 検出方法（プロセスベースのみ）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DetectionMethod {
    Process,
}

/// インストールされたコンポーネント
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InstalledComponents {
    pub dll: bool,
    pub master_ea: bool,
    pub slave_ea: bool,
    pub includes: bool,
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
    pub version: Option<String>,
    pub is_running: bool,
    pub process_id: Option<u32>,
    pub detection_method: DetectionMethod,
    pub is_installed: bool,
    pub installed_version: Option<String>,
    pub available_version: String,
    pub components: InstalledComponents,
    pub last_updated: Option<DateTime<Utc>>,
}

/// 検出サマリー
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionSummary {
    pub total_found: usize,
    pub by_method: HashMap<String, usize>,
    pub running: usize,
    pub stopped: usize,
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
        let mut path_hash = path
            .to_lowercase()
            .replace(['\\', '/', ' ', ':'], "-");

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

    /// ブローカー名をパスから抽出
    pub fn extract_broker_name(path: &str) -> String {
        // パスからブローカー名を推測
        // 例: "D:\Trading\IC Markets MT4" -> "IC Markets"
        // 例: "C:\Program Files\XM MetaTrader 5" -> "XM"

        let path_parts: Vec<&str> = path.split(['\\', '/']).collect();

        for part in path_parts.iter().rev() {
            if part.to_lowercase().contains("metatrader")
                || part.to_lowercase().contains("mt4")
                || part.to_lowercase().contains("mt5")
            {
                // ブローカー名を含む可能性が高い部分
                let cleaned = part
                    .replace("MetaTrader 4", "")
                    .replace("MetaTrader 5", "")
                    .replace("MT4", "")
                    .replace("MT5", "")
                    .trim()
                    .to_string();

                if !cleaned.is_empty() {
                    return cleaned;
                }
            }
        }

        // ブローカー名が推測できない場合は最後のフォルダ名を使用
        path_parts
            .last()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Unknown".to_string())
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

    #[test]
    fn test_extract_broker_name() {
        assert_eq!(
            MtInstallation::extract_broker_name("D:\\Trading\\IC Markets MT4"),
            "IC Markets"
        );
        assert_eq!(
            MtInstallation::extract_broker_name("C:\\Program Files\\XM MetaTrader 5"),
            "XM"
        );
        assert_eq!(
            MtInstallation::extract_broker_name("C:\\Program Files (x86)\\FXGT MT5"),
            "FXGT"
        );
    }
}
