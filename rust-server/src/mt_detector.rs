use crate::models::{
    Architecture, DetectionMethod, InstalledComponents, MtInstallation, MtType,
};
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(windows)]
use winreg::enums::*;
#[cfg(windows)]
use winreg::{RegKey, HKEY};

/// MT4/MT5レジストリ検出器
pub struct MtDetector;

impl MtDetector {
    pub fn new() -> Self {
        Self
    }

    /// Windowsレジストリから MT4/MT5 インストールを検出
    #[cfg(windows)]
    pub fn detect(&self) -> Result<Vec<MtInstallation>> {
        let mut installations = Vec::new();
        let mut seen_paths = std::collections::HashSet::new();

        // HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall
        if let Ok(hklm_installations) = self.scan_uninstall_registry(HKEY_LOCAL_MACHINE, r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall") {
            for installation in hklm_installations {
                if seen_paths.insert(installation.path.clone()) {
                    installations.push(installation);
                }
            }
        }

        // HKLM\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall (32-bit apps on 64-bit Windows)
        if let Ok(wow64_installations) = self.scan_uninstall_registry(HKEY_LOCAL_MACHINE, r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall") {
            for installation in wow64_installations {
                if seen_paths.insert(installation.path.clone()) {
                    installations.push(installation);
                }
            }
        }

        tracing::info!("Found {} MT4/MT5 installations in registry", installations.len());
        Ok(installations)
    }

    /// Non-Windows platforms - return empty vec
    #[cfg(not(windows))]
    pub fn detect(&self) -> Result<Vec<MtInstallation>> {
        Ok(Vec::new())
    }

    /// レジストリのUninstallキーをスキャン
    #[cfg(windows)]
    fn scan_uninstall_registry(&self, hkey: HKEY, path: &str) -> Result<Vec<MtInstallation>> {
        let mut installations = Vec::new();

        let hklm = RegKey::predef(hkey);
        let uninstall = match hklm.open_subkey(path) {
            Ok(key) => {
                tracing::debug!("Successfully opened registry key: {}", path);
                key
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to open registry key '{}': {} (error code: {:?})",
                    path,
                    e,
                    e.kind()
                );
                return Ok(installations);
            }
        };

        let mut total_keys = 0;
        let mut metatrader_keys = 0;
        for key_name in uninstall.enum_keys().filter_map(|k| k.ok()) {
            total_keys += 1;
            if let Ok(app_key) = uninstall.open_subkey(&key_name) {
                if let Some(installation) = self.parse_registry_entry(&app_key) {
                    metatrader_keys += 1;
                    tracing::debug!("Found MT installation in key: {}", key_name);
                    installations.push(installation);
                }
            }
        }

        tracing::info!(
            "Scanned registry key '{}': {} total keys, {} MT installations found",
            path,
            total_keys,
            metatrader_keys
        );

        Ok(installations)
    }

    /// レジストリエントリをパースしてMtInstallation情報を生成
    #[cfg(windows)]
    fn parse_registry_entry(&self, key: &RegKey) -> Option<MtInstallation> {
        // DisplayNameを取得
        let display_name: String = key.get_value("DisplayName").ok()?;

        // MetaTraderまたはMT4/MT5を含むもののみ処理
        if !display_name.to_lowercase().contains("metatrader")
            && !display_name.to_lowercase().contains("mt4")
            && !display_name.to_lowercase().contains("mt5") {
            return None;
        }

        tracing::debug!("Parsing MT registry entry: {}", display_name);

        // インストールパスを取得
        let install_location: String = match key.get_value("InstallLocation")
            .or_else(|_| key.get_value("InstallPath")) {
            Ok(path) => path,
            Err(e) => {
                tracing::warn!(
                    "Failed to get InstallLocation/InstallPath for '{}': {}",
                    display_name,
                    e
                );
                return None;
            }
        };

        tracing::debug!("Install location for '{}': {}", display_name, install_location);

        let install_path = PathBuf::from(&install_location);
        if !install_path.exists() {
            tracing::warn!("Install location does not exist: {:?}", install_path);
            return None;
        }

        // MT4/MT5のタイプとアーキテクチャを判定
        let (mt_type, platform, executable) = match self.detect_mt_type_and_platform(&install_path) {
            Some(result) => result,
            None => {
                tracing::warn!(
                    "Could not detect MT type/platform for '{}' at {:?}",
                    display_name,
                    install_path
                );
                return None;
            }
        };

        tracing::debug!(
            "Detected MT type: {:?}, platform: {:?} for '{}'",
            mt_type,
            platform,
            display_name
        );

        // データディレクトリを検出
        let data_path = match self.find_data_directory(&install_path, &mt_type) {
            Some(path) => path,
            None => {
                tracing::warn!(
                    "Could not find data directory for '{}' at {:?}",
                    display_name,
                    install_path
                );
                return None;
            }
        };
        let data_path_str = data_path.to_string_lossy().to_string();

        tracing::debug!(
            "Found data directory for '{}': {}",
            display_name,
            data_path_str
        );

        // IDを生成
        let id = MtInstallation::generate_id(&mt_type, &data_path_str);

        // バージョン情報
        let version: Option<String> = key.get_value("DisplayVersion").ok();
        let _publisher: Option<String> = key.get_value("Publisher").ok();

        // 名前を生成（DisplayNameから）
        let name = display_name.clone();

        // プロセスが実行中かチェック
        let (is_running, process_id) = self.check_if_running(&executable);

        // インストールされたコンポーネントをチェック
        let components = self.check_installed_components(&data_path, &mt_type)
            .unwrap_or_default();

        let is_installed = components.dll && components.master_ea && components.slave_ea;

        let installed_version = if is_installed {
            Some("1.0.0".to_string()) // TODO: 実際のバージョンファイルから読み取る
        } else {
            None
        };

        tracing::info!(
            "Detected {} installation: {} ({})",
            match mt_type {
                MtType::MT4 => "MT4",
                MtType::MT5 => "MT5",
            },
            name,
            data_path_str
        );

        Some(MtInstallation {
            id,
            name,
            mt_type,
            platform,
            path: data_path_str,
            executable: executable.to_string_lossy().to_string(),
            version,
            is_running,
            process_id,
            detection_method: DetectionMethod::Registry,
            is_installed,
            installed_version,
            available_version: env!("CARGO_PKG_VERSION").to_string(),
            components,
            last_updated: None,
        })
    }

    /// MT4/MT5のタイプとプラットフォームを検出
    fn detect_mt_type_and_platform(&self, install_path: &Path) -> Option<(MtType, Architecture, PathBuf)> {
        // 64-bit MT5
        let terminal64_path = install_path.join("terminal64.exe");
        if terminal64_path.exists() {
            return Some((MtType::MT5, Architecture::Bit64, terminal64_path));
        }

        // 32-bit MT4
        let terminal_path = install_path.join("terminal.exe");
        if terminal_path.exists() {
            // ディレクトリ内にMQL5があればMT5、なければMT4
            let mql5_path = install_path.join("MQL5");
            let mt_type = if mql5_path.exists() {
                MtType::MT5
            } else {
                MtType::MT4
            };
            return Some((mt_type, Architecture::Bit32, terminal_path));
        }

        None
    }

    /// データディレクトリを検出
    fn find_data_directory(&self, install_path: &Path, mt_type: &MtType) -> Option<PathBuf> {
        // ポータブルモード: インストールパスと同じ
        let mql_folder = match mt_type {
            MtType::MT4 => "MQL4",
            MtType::MT5 => "MQL5",
        };

        let portable_mql = install_path.join(mql_folder);
        tracing::debug!(
            "Checking for portable mode: {:?} (exists: {})",
            portable_mql,
            portable_mql.exists()
        );

        if portable_mql.exists() {
            tracing::debug!("Detected portable mode at {:?}", install_path);
            return Some(install_path.to_path_buf());
        }

        tracing::debug!("Not portable mode, searching in APPDATA");
        // 通常モード: %APPDATA%\MetaQuotes\Terminal\ から検索
        self.find_data_directory_from_appdata(install_path, mt_type)
    }

    /// %APPDATA%\MetaQuotes\Terminal\ からデータディレクトリを検索
    /// Windows Service (SYSTEM account) で実行される場合、全ユーザーのプロファイルを検索
    fn find_data_directory_from_appdata(&self, install_path: &Path, mt_type: &MtType) -> Option<PathBuf> {
        let appdata = match std::env::var("APPDATA") {
            Ok(path) => {
                tracing::debug!("APPDATA environment variable: {}", path);
                path
            }
            Err(e) => {
                tracing::warn!("Failed to get APPDATA environment variable: {}", e);
                return None;
            }
        };

        // Check if running as SYSTEM account (common for Windows Services)
        let is_system_account = appdata.contains("system32\\config\\systemprofile");

        if is_system_account {
            tracing::info!("Running as SYSTEM account, scanning all user profiles for MT data directories");
            return self.find_data_directory_all_users(install_path, mt_type);
        }

        let terminal_base = PathBuf::from(&appdata).join("MetaQuotes").join("Terminal");

        tracing::debug!(
            "Searching for data directory in: {:?} (exists: {})",
            terminal_base,
            terminal_base.exists()
        );

        if !terminal_base.exists() {
            tracing::warn!("Terminal base directory does not exist: {:?}", terminal_base);
            return None;
        }

        let mql_folder = match mt_type {
            MtType::MT4 => "MQL4",
            MtType::MT5 => "MQL5",
        };

        let mut checked_dirs = 0;
        // origin.txtを使って照合
        for entry in fs::read_dir(&terminal_base).ok()?.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            checked_dirs += 1;
            let origin_file = path.join("origin.txt");

            if origin_file.exists() {
                tracing::debug!("Checking origin.txt in: {:?}", path);

                if let Ok(content_bytes) = fs::read(&origin_file) {
                    if let Some(origin_path) = self.decode_origin_txt(&content_bytes) {
                        let origin_path_normalized = origin_path.to_lowercase();
                        let install_path_normalized = install_path.to_string_lossy().to_lowercase();

                        tracing::debug!(
                            "Comparing origin '{}' with install path '{}'",
                            origin_path_normalized,
                            install_path_normalized
                        );

                        if origin_path_normalized == install_path_normalized {
                            // MQLフォルダが存在することを確認
                            let mql_path = path.join(mql_folder);
                            if mql_path.exists() {
                                tracing::info!("Found data directory via origin.txt: {:?}", path);
                                return Some(path);
                            } else {
                                tracing::warn!(
                                    "Found matching origin.txt but {} folder does not exist: {:?}",
                                    mql_folder,
                                    mql_path
                                );
                            }
                        }
                    }
                }
            }
        }

        tracing::warn!(
            "No matching data directory found in {} (checked {} directories)",
            terminal_base.display(),
            checked_dirs
        );

        None
    }

    /// すべてのユーザープロファイルからデータディレクトリを検索（SYSTEM account用）
    fn find_data_directory_all_users(&self, install_path: &Path, mt_type: &MtType) -> Option<PathBuf> {
        // C:\Users\ 配下の全ユーザーを検索
        let users_dir = PathBuf::from("C:\\Users");
        if !users_dir.exists() {
            tracing::warn!("Users directory does not exist: {:?}", users_dir);
            return None;
        }

        let mql_folder = match mt_type {
            MtType::MT4 => "MQL4",
            MtType::MT5 => "MQL5",
        };

        tracing::debug!("Scanning all user profiles in: {:?}", users_dir);

        // Enumerate all user directories
        let user_entries = match fs::read_dir(&users_dir) {
            Ok(entries) => entries,
            Err(e) => {
                tracing::warn!("Failed to read users directory: {}", e);
                return None;
            }
        };

        for user_entry in user_entries.flatten() {
            let user_path = user_entry.path();
            if !user_path.is_dir() {
                continue;
            }

            let terminal_base = user_path
                .join("AppData")
                .join("Roaming")
                .join("MetaQuotes")
                .join("Terminal");

            if !terminal_base.exists() {
                continue;
            }

            tracing::debug!(
                "Checking user profile: {:?}, terminal base: {:?}",
                user_path.file_name(),
                terminal_base
            );

            // Search in this user's terminal directory
            if let Some(data_path) = self.search_terminal_directory(&terminal_base, install_path, mql_folder) {
                tracing::info!(
                    "Found data directory in user profile {:?}: {:?}",
                    user_path.file_name(),
                    data_path
                );
                return Some(data_path);
            }
        }

        tracing::warn!(
            "No matching data directory found in any user profile for: {:?}",
            install_path
        );
        None
    }

    /// Terminal ディレクトリ内で origin.txt を使ってデータディレクトリを検索
    fn search_terminal_directory(
        &self,
        terminal_base: &Path,
        install_path: &Path,
        mql_folder: &str,
    ) -> Option<PathBuf> {
        let entries = fs::read_dir(terminal_base).ok()?;

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let origin_file = path.join("origin.txt");
            if !origin_file.exists() {
                continue;
            }

            if let Ok(content_bytes) = fs::read(&origin_file) {
                if let Some(origin_path) = self.decode_origin_txt(&content_bytes) {
                    let origin_path_normalized = origin_path.to_lowercase();
                    let install_path_normalized = install_path.to_string_lossy().to_lowercase();

                    if origin_path_normalized == install_path_normalized {
                        let mql_path = path.join(mql_folder);
                        if mql_path.exists() {
                            return Some(path);
                        }
                    }
                }
            }
        }

        None
    }

    /// origin.txtをデコード（UTF-16LE）
    fn decode_origin_txt(&self, content: &[u8]) -> Option<String> {
        let content = if content.starts_with(&[0xFF, 0xFE]) {
            &content[2..]
        } else {
            content
        };

        let u16_vec: Vec<u16> = content
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();

        String::from_utf16(&u16_vec)
            .ok()
            .map(|s| s.trim_end_matches('\0').trim().to_string())
    }

    /// プロセスが実行中かチェック（簡易版 - ファイルロックでチェック）
    fn check_if_running(&self, _executable: &Path) -> (bool, Option<u32>) {
        // TODO: より正確なプロセスチェックを実装
        // 現在はファイルが存在するかのみチェック
        (false, None)
    }

    /// インストールされたコンポーネントをチェック
    fn check_installed_components(&self, data_path: &Path, mt_type: &MtType) -> Result<InstalledComponents> {
        let (mql_folder, ea_ext) = match mt_type {
            MtType::MT4 => ("MQL4", "ex4"),
            MtType::MT5 => ("MQL5", "ex5"),
        };

        let mql_path = data_path.join(mql_folder);

        // DLLチェック
        let dll_path = mql_path.join("Libraries").join("sankey_copier_zmq.dll");
        let dll = dll_path.exists();

        // Master EAチェック
        let master_ea_path = mql_path.join("Experts").join(format!("SankeyCopierMaster.{}", ea_ext));
        let master_ea = master_ea_path.exists();

        // Slave EAチェック
        let slave_ea_path = mql_path.join("Experts").join(format!("SankeyCopierSlave.{}", ea_ext));
        let slave_ea = slave_ea_path.exists();

        Ok(InstalledComponents {
            dll,
            master_ea,
            slave_ea,
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
    use tempfile::TempDir;

    #[test]
    fn test_detector_creation() {
        let _detector = MtDetector::new();
        // Just ensure it can be created
        assert!(true);
    }

    #[test]
    fn test_check_installed_components_none() {
        let detector = MtDetector::new();
        let temp_dir = TempDir::new().unwrap();
        let mt_path = temp_dir.path();

        let mql4_path = mt_path.join("MQL4");
        fs::create_dir_all(&mql4_path).unwrap();

        let result = detector.check_installed_components(mt_path, &MtType::MT4).unwrap();

        assert!(!result.dll);
        assert!(!result.master_ea);
        assert!(!result.slave_ea);
    }

    #[test]
    fn test_check_installed_components_all_mt4() {
        let detector = MtDetector::new();
        let temp_dir = TempDir::new().unwrap();
        let mt_path = temp_dir.path();

        let mql4_path = mt_path.join("MQL4");
        let libraries_path = mql4_path.join("Libraries");
        let experts_path = mql4_path.join("Experts");

        fs::create_dir_all(&libraries_path).unwrap();
        fs::create_dir_all(&experts_path).unwrap();

        fs::write(libraries_path.join("sankey_copier_zmq.dll"), b"dll").unwrap();
        fs::write(experts_path.join("SankeyCopierMaster.ex4"), b"master").unwrap();
        fs::write(experts_path.join("SankeyCopierSlave.ex4"), b"slave").unwrap();

        let result = detector.check_installed_components(mt_path, &MtType::MT4).unwrap();

        assert!(result.dll);
        assert!(result.master_ea);
        assert!(result.slave_ea);
    }

    #[test]
    fn test_check_installed_components_all_mt5() {
        let detector = MtDetector::new();
        let temp_dir = TempDir::new().unwrap();
        let mt_path = temp_dir.path();

        let mql5_path = mt_path.join("MQL5");
        let libraries_path = mql5_path.join("Libraries");
        let experts_path = mql5_path.join("Experts");

        fs::create_dir_all(&libraries_path).unwrap();
        fs::create_dir_all(&experts_path).unwrap();

        fs::write(libraries_path.join("sankey_copier_zmq.dll"), b"dll").unwrap();
        fs::write(experts_path.join("SankeyCopierMaster.ex5"), b"master").unwrap();
        fs::write(experts_path.join("SankeyCopierSlave.ex5"), b"slave").unwrap();

        let result = detector.check_installed_components(mt_path, &MtType::MT5).unwrap();

        assert!(result.dll);
        assert!(result.master_ea);
        assert!(result.slave_ea);
    }

    #[test]
    fn test_decode_origin_txt_utf16le() {
        let detector = MtDetector::new();

        // UTF-16LE BOM + "C:\Test"
        let content = vec![
            0xFF, 0xFE, // BOM
            0x43, 0x00, 0x3A, 0x00, 0x5C, 0x00, 0x54, 0x00, 0x65, 0x00, 0x73, 0x00, 0x74, 0x00
        ];

        let result = detector.decode_origin_txt(&content);
        assert_eq!(result, Some("C:\\Test".to_string()));
    }
}
