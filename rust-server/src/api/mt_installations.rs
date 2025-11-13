use axum::{
    extract::{Path, State},
    Json,
};
use std::path::PathBuf;

use crate::models::{DetectionSummary, MtInstallationsResponse};
use crate::mt_detector::MtDetector;
use crate::mt_installer::MtInstaller;

use super::{ApiResponse, AppState};

/// MT4/MT5インストール一覧を取得（レジストリ検出）
pub async fn list_mt_installations(
    State(_state): State<AppState>,
) -> Result<Json<ApiResponse<MtInstallationsResponse>>, String> {
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
        detection_summary: DetectionSummary {
            total_found,
        },
    };

    Ok(Json(ApiResponse::success(response)))
}

/// MT4/MT5にコンポーネントをインストール
pub async fn install_to_mt(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<String>>, String> {
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
            return Ok(Json(ApiResponse::error(format!(
                "MT4/MT5の検出に失敗しました: {}",
                e
            ))));
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
            return Ok(Json(ApiResponse::error(format!(
                "指定されたID ({}) のMT4/MT5が見つかりません。",
                id
            ))));
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
            Ok(Json(ApiResponse::success(format!(
                "インストールが完了しました: {}",
                installation.name
            ))))
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
            Ok(Json(ApiResponse::error(format!(
                "インストールに失敗しました: {}",
                e
            ))))
        }
    }
}
