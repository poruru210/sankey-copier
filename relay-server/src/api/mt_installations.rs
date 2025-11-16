use axum::{
    extract::{Path, State},
    Json,
};
use std::path::PathBuf;

use crate::models::{DetectionSummary, MtInstallationsResponse};
use crate::mt_detector::MtDetector;
use crate::mt_installer::MtInstaller;

use super::{AppState, ProblemDetails};

/// MT4/MT5インストール一覧を取得（レジストリ検出）
pub async fn list_mt_installations(
    State(_state): State<AppState>,
) -> Result<Json<MtInstallationsResponse>, ProblemDetails> {
    let span = tracing::info_span!("list_mt_installations");
    let _enter = span.enter();

    // Windowsレジストリから MT4/MT5 を検出
    let detector = MtDetector::new();
    let installations = match detector.detect() {
        Ok(installs) => {
            tracing::info!(
                count = installs.len(),
                "Successfully detected MT installations"
            );
            installs
        }
        Err(e) => {
            tracing::error!(
                error = %e,
                error_type = std::any::type_name_of_val(&e),
                backtrace = ?std::backtrace::Backtrace::capture(),
                "Failed to detect MT installations from registry"
            );
            Vec::new()
        }
    };

    // 検出サマリーを作成
    let total_found = installations.len();

    tracing::info!(
        total_found = total_found,
        "MT installations detection summary"
    );

    let response = MtInstallationsResponse {
        success: true,
        data: installations,
        detection_summary: DetectionSummary { total_found },
    };

    Ok(Json(response))
}

/// MT4/MT5にコンポーネントをインストール
pub async fn install_to_mt(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<String>, ProblemDetails> {
    let span = tracing::info_span!("install_to_mt", installation_id = %id);
    let _enter = span.enter();

    tracing::info!(
        installation_id = %id,
        "Installation request received"
    );

    // レジストリから MT4/MT5 を検出して該当のものを探す
    let detector = MtDetector::new();
    let installations = match detector.detect() {
        Ok(installs) => installs,
        Err(e) => {
            tracing::error!(
                installation_id = %id,
                error = %e,
                error_type = std::any::type_name_of_val(&e),
                backtrace = ?std::backtrace::Backtrace::capture(),
                "Failed to detect MT installations for install operation"
            );
            return Err(ProblemDetails::internal_error(format!(
                "Failed to detect MT4/MT5 installations: {}",
                e
            ))
            .with_instance(format!("/api/mt-installations/{}/install", id)));
        }
    };

    // IDに一致するインストールを探す
    let installation = installations.iter().find(|i| i.id == id);

    let installation = match installation {
        Some(inst) => inst,
        None => {
            tracing::warn!(
                installation_id = %id,
                available_ids = ?installations.iter().map(|i| &i.id).collect::<Vec<_>>(),
                "MT installation not found"
            );
            return Err(ProblemDetails::not_found("MT4/MT5 installation")
                .with_detail(format!(
                    "The specified MT4/MT5 installation (ID: {}) was not found",
                    id
                ))
                .with_instance(format!("/api/mt-installations/{}/install", id)));
        }
    };

    tracing::info!(
        installation_id = %id,
        installation_name = %installation.name,
        installation_path = %installation.path,
        mt_type = ?installation.mt_type,
        platform = ?installation.platform,
        "Starting installation process"
    );

    // インストーラーを作成
    let installer = MtInstaller::from_config(&state.config);

    // インストール実行
    let mt_path = PathBuf::from(&installation.path);
    match installer.install(&mt_path, &installation.mt_type, &installation.platform) {
        Ok(_) => {
            tracing::info!(
                installation_id = %id,
                installation_name = %installation.name,
                installation_path = %installation.path,
                "Installation completed successfully"
            );
            Ok(Json(format!(
                "Installation completed: {}",
                installation.name
            )))
        }
        Err(e) => {
            tracing::error!(
                installation_id = %id,
                installation_name = %installation.name,
                installation_path = %installation.path,
                error = %e,
                error_type = std::any::type_name_of_val(&e),
                backtrace = ?std::backtrace::Backtrace::capture(),
                "Installation failed"
            );
            Err(
                ProblemDetails::internal_error(format!("Installation failed: {}", e))
                    .with_instance(format!("/api/mt-installations/{}/install", id)),
            )
        }
    }
}
