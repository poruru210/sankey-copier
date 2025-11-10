use crate::models::{
    Architecture, DetectionMethod, InstalledComponents, MtInstallation, MtType,
};
use anyhow::Result;
use std::path::Path;
use sysinfo::System;

/// MT4/MT5プロセス検出器
pub struct MtDetector {
    system: System,
}

impl MtDetector {
    /// 新しい検出器を作成
    pub fn new() -> Self {
        Self {
            system: System::new_all(),
        }
    }

    /// 起動中のMT4/MT5プロセスを検出
    pub fn detect_running_installations(&mut self) -> Result<Vec<MtInstallation>> {
        // プロセス情報を更新
        self.system.refresh_all();

        let mut installations = Vec::new();
        let mut seen_paths = std::collections::HashSet::new();

        // すべてのプロセスをスキャン
        for (pid, process) in self.system.processes() {
            if let Some(exe_path) = process.exe() {
                let exe_name = exe_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");

                // terminal.exe または terminal64.exe を探す
                if exe_name == "terminal.exe" || exe_name == "terminal64.exe" {
                    let base_path = exe_path.parent().unwrap_or(exe_path);

                    // 既に処理済みのパスはスキップ（複数プロセスが同じインストールから起動している場合）
                    if !seen_paths.insert(base_path.to_path_buf()) {
                        continue;
                    }

                    match self.analyze_installation(base_path, Some(pid.as_u32())) {
                        Ok(installation) => {
                            installations.push(installation);
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to analyze MT installation at {:?}: {}",
                                base_path,
                                e
                            );
                        }
                    }
                }
            }
        }

        Ok(installations)
    }

    /// インストールパスを分析してMtInstallation情報を生成
    pub fn analyze_installation(
        &self,
        base_path: &Path,
        process_id: Option<u32>,
    ) -> Result<MtInstallation> {
        // MT4/MT5の判定
        let mql4_path = base_path.join("MQL4");
        let mql5_path = base_path.join("MQL5");

        let mt_type = if mql5_path.exists() {
            MtType::MT5
        } else if mql4_path.exists() {
            MtType::MT4
        } else {
            anyhow::bail!("Neither MQL4 nor MQL5 folder found");
        };

        // ビット数の判定
        let terminal64_path = base_path.join("terminal64.exe");
        let terminal_path = base_path.join("terminal.exe");

        let (platform, executable) = if terminal64_path.exists() {
            (Architecture::Bit64, terminal64_path)
        } else if terminal_path.exists() {
            (Architecture::Bit32, terminal_path)
        } else {
            anyhow::bail!("Terminal executable not found");
        };

        let base_path_str = base_path.to_string_lossy().to_string();
        let id = MtInstallation::generate_id(&mt_type, &base_path_str);

        // ブローカー名の推測
        let broker_name = MtInstallation::extract_broker_name(&base_path_str);
        let name = format!(
            "{} MetaTrader {}",
            broker_name,
            match mt_type {
                MtType::MT4 => "4",
                MtType::MT5 => "5",
            }
        );

        // バージョン情報の取得（実装可能であれば）
        let version = self.detect_version(base_path);

        // インストールされたコンポーネントをチェック
        let components = self.check_installed_components(base_path, &mt_type)?;

        // 全コンポーネントがインストールされているか
        let is_installed = components.dll
            && components.master_ea
            && components.slave_ea
            && components.includes;

        // インストール済みバージョンの取得（TODO: バージョンファイルから読み取る）
        let installed_version = if is_installed {
            Some("1.0.0".to_string()) // TODO: 実際のバージョンを取得
        } else {
            None
        };

        // 最終更新日時（TODO: ファイルのタイムスタンプから取得）
        let last_updated = if is_installed {
            Some(chrono::Utc::now())
        } else {
            None
        };

        Ok(MtInstallation {
            id,
            name,
            mt_type,
            platform,
            path: base_path_str,
            executable: executable.to_string_lossy().to_string(),
            version,
            is_running: process_id.is_some(),
            process_id,
            detection_method: DetectionMethod::Process,
            is_installed,
            installed_version,
            available_version: "1.0.0".to_string(), // TODO: 実際の利用可能バージョン
            components,
            last_updated,
        })
    }

    /// MT4/MT5のバージョンを検出
    fn detect_version(&self, _base_path: &Path) -> Option<String> {
        // terminal.exe のバージョン情報を取得する実装
        // 現時点では簡易実装
        None
    }

    /// インストール済みコンポーネントをチェック
    fn check_installed_components(
        &self,
        base_path: &Path,
        mt_type: &MtType,
    ) -> Result<InstalledComponents> {
        let mql_folder = match mt_type {
            MtType::MT4 => "MQL4",
            MtType::MT5 => "MQL5",
        };

        let mql_path = base_path.join(mql_folder);

        // DLLチェック
        let dll_path = mql_path.join("Libraries").join("sankey_copier_zmq.dll");
        let dll = dll_path.exists();

        // Master EAチェック
        let master_ea_ext = match mt_type {
            MtType::MT4 => "ex4",
            MtType::MT5 => "ex5",
        };
        let master_ea_path = mql_path
            .join("Experts")
            .join(format!("SankeyCopierMaster.{}", master_ea_ext));
        let master_ea = master_ea_path.exists();

        // Slave EAチェック
        let slave_ea_path = mql_path
            .join("Experts")
            .join(format!("SankeyCopierSlave.{}", master_ea_ext));
        let slave_ea = slave_ea_path.exists();

        // Include filesチェック
        let includes_path = mql_path.join("Include").join("SankeyCopier");
        let includes = includes_path.exists()
            && includes_path.join("SankeyCopierCommon.mqh").exists()
            && includes_path.join("SankeyCopierMessages.mqh").exists()
            && includes_path.join("SankeyCopierTrade.mqh").exists();

        Ok(InstalledComponents {
            dll,
            master_ea,
            slave_ea,
            includes,
        })
    }
}

impl Default for MtDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mt_detector_creation() {
        let detector = MtDetector::new();
        assert!(detector.system.processes().len() > 0);
    }
}
