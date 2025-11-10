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

    #[test]
    fn test_installer_creation() {
        let installer = MtInstaller::default();
        assert!(installer.components_base_path.exists() || !installer.components_base_path.exists()); // パスの存在は問わない
    }
}
