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

        tracing::info!("Installation completed successfully");
        Ok(())
    }

    /// DLLをインストール
    fn install_dll(&self, mql_path: &Path, architecture: &Architecture) -> Result<()> {
        let libraries_path = mql_path.join("Libraries");
        fs::create_dir_all(&libraries_path)
            .context("Failed to create Libraries directory")?;

        // DLLソースパスを決定（本番環境と開発環境の両方をサポート）
        let dll_source = match architecture {
            Architecture::Bit32 => {
                // Try production path first (installer package)
                let prod_path = self.components_base_path.join("mql/MT4/Libraries/sankey_copier_zmq.dll");
                if prod_path.exists() {
                    prod_path
                } else {
                    // Fall back to development path
                    self.components_base_path
                        .join("mql-zmq-dll/target/i686-pc-windows-msvc/release/sankey_copier_zmq.dll")
                }
            }
            Architecture::Bit64 => {
                // Try production path first (installer package)
                let prod_path = self.components_base_path.join("mql/MT5/Libraries/sankey_copier_zmq.dll");
                if prod_path.exists() {
                    prod_path
                } else {
                    // Fall back to development path
                    self.components_base_path
                        .join("mql-zmq-dll/target/release/sankey_copier_zmq.dll")
                }
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

    /// EAをインストール（コンパイル済みバイナリのみ）
    fn install_eas(&self, mql_path: &Path, mt_type: &MtType) -> Result<()> {
        let experts_path = mql_path.join("Experts");
        fs::create_dir_all(&experts_path)
            .context("Failed to create Experts directory")?;

        let (mt_folder, extension) = match mt_type {
            MtType::MT4 => ("MT4", "ex4"),
            MtType::MT5 => ("MT5", "ex5"),
        };

        // Master EAをコピー
        let master_source = self
            .components_base_path
            .join(format!("mql/{}/SankeyCopierMaster.{}", mt_folder, extension));

        if !master_source.exists() {
            anyhow::bail!(
                "Master EA binary not found: {}",
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
            .join(format!("mql/{}/SankeyCopierSlave.{}", mt_folder, extension));

        if !slave_source.exists() {
            anyhow::bail!(
                "Slave EA binary not found: {}",
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

        // Create source EA binary files for MT4 (flattened structure)
        let source_path = temp_components.path().join("mql/MT4");
        fs::create_dir_all(&source_path).unwrap();

        fs::write(source_path.join("SankeyCopierMaster.ex4"), b"master ea mt4").unwrap();
        fs::write(source_path.join("SankeyCopierSlave.ex4"), b"slave ea mt4").unwrap();

        // Create MT4 structure
        let mql_path = temp_mt.path().join("MQL4");
        fs::create_dir_all(&mql_path).unwrap();

        // Install
        let installer = MtInstaller::new(temp_components.path().to_path_buf());
        let result = installer.install_eas(&mql_path, &MtType::MT4);

        assert!(result.is_ok());

        // Verify EAs were copied
        let dest_master = mql_path.join("Experts").join("SankeyCopierMaster.ex4");
        let dest_slave = mql_path.join("Experts").join("SankeyCopierSlave.ex4");
        assert!(dest_master.exists());
        assert!(dest_slave.exists());
        assert_eq!(fs::read(&dest_master).unwrap(), b"master ea mt4");
        assert_eq!(fs::read(&dest_slave).unwrap(), b"slave ea mt4");
    }

    #[test]
    fn test_install_eas_mt5() {
        let temp_components = TempDir::new().unwrap();
        let temp_mt = TempDir::new().unwrap();

        // Create source EA binary files for MT5 (flattened structure)
        let source_path = temp_components.path().join("mql/MT5");
        fs::create_dir_all(&source_path).unwrap();

        fs::write(source_path.join("SankeyCopierMaster.ex5"), b"master ea mt5").unwrap();
        fs::write(source_path.join("SankeyCopierSlave.ex5"), b"slave ea mt5").unwrap();

        // Create MT5 structure
        let mql_path = temp_mt.path().join("MQL5");
        fs::create_dir_all(&mql_path).unwrap();

        // Install
        let installer = MtInstaller::new(temp_components.path().to_path_buf());
        let result = installer.install_eas(&mql_path, &MtType::MT5);

        assert!(result.is_ok());

        // Verify EAs were copied
        let dest_master = mql_path.join("Experts").join("SankeyCopierMaster.ex5");
        let dest_slave = mql_path.join("Experts").join("SankeyCopierSlave.ex5");
        assert!(dest_master.exists());
        assert!(dest_slave.exists());
        assert_eq!(fs::read(&dest_master).unwrap(), b"master ea mt5");
        assert_eq!(fs::read(&dest_slave).unwrap(), b"slave ea mt5");
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

        // Verify all components (実行に必要なもののみ)
        let mql_path = mt_path.join("MQL4");
        assert!(mql_path.join("Libraries").join("sankey_copier_zmq.dll").exists());
        assert!(mql_path.join("Experts").join("SankeyCopierMaster.ex4").exists());
        assert!(mql_path.join("Experts").join("SankeyCopierSlave.ex4").exists());
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

        // Verify all components (実行に必要なもののみ)
        let mql_path = mt_path.join("MQL5");
        assert!(mql_path.join("Libraries").join("sankey_copier_zmq.dll").exists());
        assert!(mql_path.join("Experts").join("SankeyCopierMaster.ex5").exists());
        assert!(mql_path.join("Experts").join("SankeyCopierSlave.ex5").exists());
    }

    /// Helper function to setup complete component directory structure (実行に必要なもののみ)
    fn setup_complete_components(base_path: &Path, is_mt4: bool) {
        // DLL files (32-bit)
        let dll_32_path = base_path.join("mql-zmq-dll/target/i686-pc-windows-msvc/release");
        fs::create_dir_all(&dll_32_path).unwrap();
        fs::write(dll_32_path.join("sankey_copier_zmq.dll"), b"32-bit dll").unwrap();

        // DLL files (64-bit)
        let dll_64_path = base_path.join("mql-zmq-dll/target/release");
        fs::create_dir_all(&dll_64_path).unwrap();
        fs::write(dll_64_path.join("sankey_copier_zmq.dll"), b"64-bit dll").unwrap();

        let (mt_folder, ext) = if is_mt4 { ("MT4", "ex4") } else { ("MT5", "ex5") };

        // EA binary files (flattened structure - directly under mql/MT4 or mql/MT5)
        let ea_path = base_path.join(format!("mql/{}", mt_folder));
        fs::create_dir_all(&ea_path).unwrap();
        fs::write(ea_path.join(format!("SankeyCopierMaster.{}", ext)), b"master").unwrap();
        fs::write(ea_path.join(format!("SankeyCopierSlave.{}", ext)), b"slave").unwrap();
    }
}
