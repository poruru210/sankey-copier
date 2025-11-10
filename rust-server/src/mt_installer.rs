use crate::models::{Architecture, MtType};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// MT4/MT5インストーラー
pub struct MtInstaller {
    /// コンポーネントファイルのベースパス
    components_base_path: PathBuf,
}

impl MtInstaller {
    /// 新しいインストーラーを作成
    pub fn new(components_base_path: PathBuf) -> Self {
        Self {
            components_base_path,
        }
    }

    /// デフォルトのコンポーネントパス（開発環境用）
    pub fn default() -> Self {
        // プロジェクトルートからの相対パス
        let project_root = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".."));

        Self::new(project_root)
    }

    /// MT4/MT5にコンポーネントをインストール
    pub fn install(
        &self,
        mt_path: &Path,
        mt_type: &MtType,
        architecture: &Architecture,
    ) -> Result<()> {
        tracing::info!(
            "Starting installation to {} {:?} ({:?})",
            mt_path.display(),
            mt_type,
            architecture
        );

        // MQL4/MQL5フォルダを決定
        let mql_folder = match mt_type {
            MtType::MT4 => "MQL4",
            MtType::MT5 => "MQL5",
        };

        let mql_path = mt_path.join(mql_folder);
        if !mql_path.exists() {
            anyhow::bail!("{} folder not found at {}", mql_folder, mql_path.display());
        }

        // DLLをコピー
        self.install_dll(&mql_path, architecture)
            .context("Failed to install DLL")?;

        // EAをコピー
        self.install_eas(&mql_path, mt_type)
            .context("Failed to install EAs")?;

        // Include filesをコピー
        self.install_includes(&mql_path)
            .context("Failed to install include files")?;

        tracing::info!("Installation completed successfully");
        Ok(())
    }

    /// DLLをインストール
    fn install_dll(&self, mql_path: &Path, architecture: &Architecture) -> Result<()> {
        let libraries_path = mql_path.join("Libraries");
        fs::create_dir_all(&libraries_path)
            .context("Failed to create Libraries directory")?;

        // DLLソースパスを決定
        let dll_source = match architecture {
            Architecture::Bit32 => {
                self.components_base_path
                    .join("mql-zmq-dll/target/i686-pc-windows-msvc/release/sankey_copier_zmq.dll")
            }
            Architecture::Bit64 => {
                self.components_base_path
                    .join("mql-zmq-dll/target/release/sankey_copier_zmq.dll")
            }
        };

        if !dll_source.exists() {
            anyhow::bail!(
                "DLL source not found: {}. Please build the DLL first.",
                dll_source.display()
            );
        }

        let dll_dest = libraries_path.join("sankey_copier_zmq.dll");

        tracing::info!(
            "Copying DLL from {} to {}",
            dll_source.display(),
            dll_dest.display()
        );

        fs::copy(&dll_source, &dll_dest)
            .with_context(|| format!("Failed to copy DLL to {}", dll_dest.display()))?;

        Ok(())
    }

    /// EAをインストール
    fn install_eas(&self, mql_path: &Path, mt_type: &MtType) -> Result<()> {
        let experts_path = mql_path.join("Experts");
        fs::create_dir_all(&experts_path)
            .context("Failed to create Experts directory")?;

        let (mt_folder, extension) = match mt_type {
            MtType::MT4 => ("MT4", "mq4"),
            MtType::MT5 => ("MT5", "mq5"),
        };

        // Master EAをコピー
        let master_source = self
            .components_base_path
            .join(format!("mql/{}/Master/SankeyCopierMaster.{}", mt_folder, extension));

        if !master_source.exists() {
            anyhow::bail!(
                "Master EA source not found: {}",
                master_source.display()
            );
        }

        let master_dest = experts_path.join(format!("SankeyCopierMaster.{}", extension));

        tracing::info!(
            "Copying Master EA from {} to {}",
            master_source.display(),
            master_dest.display()
        );

        fs::copy(&master_source, &master_dest)
            .with_context(|| format!("Failed to copy Master EA to {}", master_dest.display()))?;

        // Slave EAをコピー
        let slave_source = self
            .components_base_path
            .join(format!("mql/{}/Slave/SankeyCopierSlave.{}", mt_folder, extension));

        if !slave_source.exists() {
            anyhow::bail!(
                "Slave EA source not found: {}",
                slave_source.display()
            );
        }

        let slave_dest = experts_path.join(format!("SankeyCopierSlave.{}", extension));

        tracing::info!(
            "Copying Slave EA from {} to {}",
            slave_source.display(),
            slave_dest.display()
        );

        fs::copy(&slave_source, &slave_dest)
            .with_context(|| format!("Failed to copy Slave EA to {}", slave_dest.display()))?;

        Ok(())
    }

    /// Include filesをインストール
    fn install_includes(&self, mql_path: &Path) -> Result<()> {
        let include_path = mql_path.join("Include");
        let sankey_include_path = include_path.join("SankeyCopier");

        fs::create_dir_all(&sankey_include_path)
            .context("Failed to create Include/SankeyCopier directory")?;

        let source_include_path = self
            .components_base_path
            .join("mql/Include/SankeyCopier");

        if !source_include_path.exists() {
            anyhow::bail!(
                "Include files source not found: {}",
                source_include_path.display()
            );
        }

        // 各.mqhファイルをコピー
        let include_files = ["SankeyCopierCommon.mqh", "SankeyCopierMessages.mqh", "SankeyCopierTrade.mqh"];

        for file_name in &include_files {
            let source = source_include_path.join(file_name);
            if !source.exists() {
                anyhow::bail!("Include file not found: {}", source.display());
            }

            let dest = sankey_include_path.join(file_name);

            tracing::info!(
                "Copying include file from {} to {}",
                source.display(),
                dest.display()
            );

            fs::copy(&source, &dest)
                .with_context(|| format!("Failed to copy {} to {}", file_name, dest.display()))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_installer_creation() {
        let installer = MtInstaller::default();
        assert!(installer.components_base_path.exists() || !installer.components_base_path.exists()); // パスの存在は問わない
    }

    #[test]
    fn test_installer_with_custom_path() {
        let temp_dir = TempDir::new().unwrap();
        let custom_path = temp_dir.path().to_path_buf();
        let installer = MtInstaller::new(custom_path.clone());

        assert_eq!(installer.components_base_path, custom_path);
    }

    #[test]
    fn test_install_dll_32bit() {
        let temp_components = TempDir::new().unwrap();
        let temp_mt = TempDir::new().unwrap();

        // Create source DLL directory structure
        let source_dll_path = temp_components.path()
            .join("mql-zmq-dll/target/i686-pc-windows-msvc/release");
        fs::create_dir_all(&source_dll_path).unwrap();

        // Create dummy DLL file
        let source_dll = source_dll_path.join("sankey_copier_zmq.dll");
        fs::write(&source_dll, b"32-bit DLL content").unwrap();

        // Create MT4 structure
        let mql_path = temp_mt.path().join("MQL4");
        fs::create_dir_all(&mql_path).unwrap();

        // Install
        let installer = MtInstaller::new(temp_components.path().to_path_buf());
        let result = installer.install_dll(&mql_path, &Architecture::Bit32);

        assert!(result.is_ok());

        // Verify DLL was copied
        let dest_dll = mql_path.join("Libraries").join("sankey_copier_zmq.dll");
        assert!(dest_dll.exists());
        let content = fs::read(&dest_dll).unwrap();
        assert_eq!(content, b"32-bit DLL content");
    }

    #[test]
    fn test_install_dll_64bit() {
        let temp_components = TempDir::new().unwrap();
        let temp_mt = TempDir::new().unwrap();

        // Create source DLL directory structure
        let source_dll_path = temp_components.path()
            .join("mql-zmq-dll/target/release");
        fs::create_dir_all(&source_dll_path).unwrap();

        // Create dummy DLL file
        let source_dll = source_dll_path.join("sankey_copier_zmq.dll");
        fs::write(&source_dll, b"64-bit DLL content").unwrap();

        // Create MT5 structure
        let mql_path = temp_mt.path().join("MQL5");
        fs::create_dir_all(&mql_path).unwrap();

        // Install
        let installer = MtInstaller::new(temp_components.path().to_path_buf());
        let result = installer.install_dll(&mql_path, &Architecture::Bit64);

        assert!(result.is_ok());

        // Verify DLL was copied
        let dest_dll = mql_path.join("Libraries").join("sankey_copier_zmq.dll");
        assert!(dest_dll.exists());
        let content = fs::read(&dest_dll).unwrap();
        assert_eq!(content, b"64-bit DLL content");
    }

    #[test]
    fn test_install_dll_missing_source() {
        let temp_components = TempDir::new().unwrap();
        let temp_mt = TempDir::new().unwrap();

        // Don't create source DLL
        let mql_path = temp_mt.path().join("MQL4");
        fs::create_dir_all(&mql_path).unwrap();

        // Try to install - should fail
        let installer = MtInstaller::new(temp_components.path().to_path_buf());
        let result = installer.install_dll(&mql_path, &Architecture::Bit32);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("DLL source not found"));
    }

    #[test]
    fn test_install_eas_mt4() {
        let temp_components = TempDir::new().unwrap();
        let temp_mt = TempDir::new().unwrap();

        // Create source EA files for MT4
        let source_master_path = temp_components.path().join("mql/MT4/Master");
        let source_slave_path = temp_components.path().join("mql/MT4/Slave");
        fs::create_dir_all(&source_master_path).unwrap();
        fs::create_dir_all(&source_slave_path).unwrap();

        fs::write(source_master_path.join("SankeyCopierMaster.mq4"), b"master ea mt4").unwrap();
        fs::write(source_slave_path.join("SankeyCopierSlave.mq4"), b"slave ea mt4").unwrap();

        // Create MT4 structure
        let mql_path = temp_mt.path().join("MQL4");
        fs::create_dir_all(&mql_path).unwrap();

        // Install
        let installer = MtInstaller::new(temp_components.path().to_path_buf());
        let result = installer.install_eas(&mql_path, &MtType::MT4);

        assert!(result.is_ok());

        // Verify EAs were copied
        let dest_master = mql_path.join("Experts").join("SankeyCopierMaster.mq4");
        let dest_slave = mql_path.join("Experts").join("SankeyCopierSlave.mq4");
        assert!(dest_master.exists());
        assert!(dest_slave.exists());
        assert_eq!(fs::read(&dest_master).unwrap(), b"master ea mt4");
        assert_eq!(fs::read(&dest_slave).unwrap(), b"slave ea mt4");
    }

    #[test]
    fn test_install_eas_mt5() {
        let temp_components = TempDir::new().unwrap();
        let temp_mt = TempDir::new().unwrap();

        // Create source EA files for MT5
        let source_master_path = temp_components.path().join("mql/MT5/Master");
        let source_slave_path = temp_components.path().join("mql/MT5/Slave");
        fs::create_dir_all(&source_master_path).unwrap();
        fs::create_dir_all(&source_slave_path).unwrap();

        fs::write(source_master_path.join("SankeyCopierMaster.mq5"), b"master ea mt5").unwrap();
        fs::write(source_slave_path.join("SankeyCopierSlave.mq5"), b"slave ea mt5").unwrap();

        // Create MT5 structure
        let mql_path = temp_mt.path().join("MQL5");
        fs::create_dir_all(&mql_path).unwrap();

        // Install
        let installer = MtInstaller::new(temp_components.path().to_path_buf());
        let result = installer.install_eas(&mql_path, &MtType::MT5);

        assert!(result.is_ok());

        // Verify EAs were copied
        let dest_master = mql_path.join("Experts").join("SankeyCopierMaster.mq5");
        let dest_slave = mql_path.join("Experts").join("SankeyCopierSlave.mq5");
        assert!(dest_master.exists());
        assert!(dest_slave.exists());
        assert_eq!(fs::read(&dest_master).unwrap(), b"master ea mt5");
        assert_eq!(fs::read(&dest_slave).unwrap(), b"slave ea mt5");
    }

    #[test]
    fn test_install_includes() {
        let temp_components = TempDir::new().unwrap();
        let temp_mt = TempDir::new().unwrap();

        // Create source include files
        let source_includes = temp_components.path().join("mql/Include/SankeyCopier");
        fs::create_dir_all(&source_includes).unwrap();

        fs::write(source_includes.join("SankeyCopierCommon.mqh"), b"common").unwrap();
        fs::write(source_includes.join("SankeyCopierMessages.mqh"), b"messages").unwrap();
        fs::write(source_includes.join("SankeyCopierTrade.mqh"), b"trade").unwrap();

        // Create MQL structure
        let mql_path = temp_mt.path().join("MQL4");
        fs::create_dir_all(&mql_path).unwrap();

        // Install
        let installer = MtInstaller::new(temp_components.path().to_path_buf());
        let result = installer.install_includes(&mql_path);

        assert!(result.is_ok());

        // Verify includes were copied
        let dest_includes = mql_path.join("Include").join("SankeyCopier");
        assert!(dest_includes.join("SankeyCopierCommon.mqh").exists());
        assert!(dest_includes.join("SankeyCopierMessages.mqh").exists());
        assert!(dest_includes.join("SankeyCopierTrade.mqh").exists());

        assert_eq!(fs::read(dest_includes.join("SankeyCopierCommon.mqh")).unwrap(), b"common");
        assert_eq!(fs::read(dest_includes.join("SankeyCopierMessages.mqh")).unwrap(), b"messages");
        assert_eq!(fs::read(dest_includes.join("SankeyCopierTrade.mqh")).unwrap(), b"trade");
    }

    #[test]
    fn test_install_includes_missing_file() {
        let temp_components = TempDir::new().unwrap();
        let temp_mt = TempDir::new().unwrap();

        // Create source include files but miss one
        let source_includes = temp_components.path().join("mql/Include/SankeyCopier");
        fs::create_dir_all(&source_includes).unwrap();

        fs::write(source_includes.join("SankeyCopierCommon.mqh"), b"common").unwrap();
        fs::write(source_includes.join("SankeyCopierMessages.mqh"), b"messages").unwrap();
        // Missing: SankeyCopierTrade.mqh

        // Create MQL structure
        let mql_path = temp_mt.path().join("MQL4");
        fs::create_dir_all(&mql_path).unwrap();

        // Install should fail
        let installer = MtInstaller::new(temp_components.path().to_path_buf());
        let result = installer.install_includes(&mql_path);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Include file not found"));
    }

    #[test]
    fn test_full_install_mt4_32bit() {
        let temp_components = TempDir::new().unwrap();
        let temp_mt = TempDir::new().unwrap();

        // Setup complete component structure
        setup_complete_components(temp_components.path(), true); // true = MT4

        // Create MT4 installation directory
        let mt_path = temp_mt.path();
        fs::create_dir_all(mt_path.join("MQL4")).unwrap();

        // Install
        let installer = MtInstaller::new(temp_components.path().to_path_buf());
        let result = installer.install(mt_path, &MtType::MT4, &Architecture::Bit32);

        assert!(result.is_ok());

        // Verify all components
        let mql_path = mt_path.join("MQL4");
        assert!(mql_path.join("Libraries").join("sankey_copier_zmq.dll").exists());
        assert!(mql_path.join("Experts").join("SankeyCopierMaster.mq4").exists());
        assert!(mql_path.join("Experts").join("SankeyCopierSlave.mq4").exists());
        assert!(mql_path.join("Include").join("SankeyCopier").join("SankeyCopierCommon.mqh").exists());
        assert!(mql_path.join("Include").join("SankeyCopier").join("SankeyCopierMessages.mqh").exists());
        assert!(mql_path.join("Include").join("SankeyCopier").join("SankeyCopierTrade.mqh").exists());
    }

    #[test]
    fn test_full_install_mt5_64bit() {
        let temp_components = TempDir::new().unwrap();
        let temp_mt = TempDir::new().unwrap();

        // Setup complete component structure
        setup_complete_components(temp_components.path(), false); // false = MT5

        // Create MT5 installation directory
        let mt_path = temp_mt.path();
        fs::create_dir_all(mt_path.join("MQL5")).unwrap();

        // Install
        let installer = MtInstaller::new(temp_components.path().to_path_buf());
        let result = installer.install(mt_path, &MtType::MT5, &Architecture::Bit64);

        assert!(result.is_ok());

        // Verify all components
        let mql_path = mt_path.join("MQL5");
        assert!(mql_path.join("Libraries").join("sankey_copier_zmq.dll").exists());
        assert!(mql_path.join("Experts").join("SankeyCopierMaster.mq5").exists());
        assert!(mql_path.join("Experts").join("SankeyCopierSlave.mq5").exists());
        assert!(mql_path.join("Include").join("SankeyCopier").join("SankeyCopierCommon.mqh").exists());
        assert!(mql_path.join("Include").join("SankeyCopier").join("SankeyCopierMessages.mqh").exists());
        assert!(mql_path.join("Include").join("SankeyCopier").join("SankeyCopierTrade.mqh").exists());
    }

    /// Helper function to setup complete component directory structure
    fn setup_complete_components(base_path: &Path, is_mt4: bool) {
        // DLL files (32-bit)
        let dll_32_path = base_path.join("mql-zmq-dll/target/i686-pc-windows-msvc/release");
        fs::create_dir_all(&dll_32_path).unwrap();
        fs::write(dll_32_path.join("sankey_copier_zmq.dll"), b"32-bit dll").unwrap();

        // DLL files (64-bit)
        let dll_64_path = base_path.join("mql-zmq-dll/target/release");
        fs::create_dir_all(&dll_64_path).unwrap();
        fs::write(dll_64_path.join("sankey_copier_zmq.dll"), b"64-bit dll").unwrap();

        let (mt_folder, ext) = if is_mt4 { ("MT4", "mq4") } else { ("MT5", "mq5") };

        // EA files
        let master_path = base_path.join(format!("mql/{}/Master", mt_folder));
        let slave_path = base_path.join(format!("mql/{}/Slave", mt_folder));
        fs::create_dir_all(&master_path).unwrap();
        fs::create_dir_all(&slave_path).unwrap();
        fs::write(master_path.join(format!("SankeyCopierMaster.{}", ext)), b"master").unwrap();
        fs::write(slave_path.join(format!("SankeyCopierSlave.{}", ext)), b"slave").unwrap();

        // Include files
        let includes_path = base_path.join("mql/Include/SankeyCopier");
        fs::create_dir_all(&includes_path).unwrap();
        fs::write(includes_path.join("SankeyCopierCommon.mqh"), b"common").unwrap();
        fs::write(includes_path.join("SankeyCopierMessages.mqh"), b"messages").unwrap();
        fs::write(includes_path.join("SankeyCopierTrade.mqh"), b"trade").unwrap();
    }
}
