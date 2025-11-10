use axum::{
    extract::{Path, State},
    Json,
};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::models::{DetectionMethod, DetectionSummary, MtInstallationsResponse};
use crate::mt_detector::MtDetector;
use crate::mt_installer::MtInstaller;

use super::{ApiResponse, AppState};

/// MT4/MT5インストール一覧を取得（プロセス検出のみ）
pub async fn list_mt_installations(
    State(_state): State<AppState>,
) -> Result<Json<ApiResponse<MtInstallationsResponse>>, String> {
    // プロセスから起動中のMT4/MT5を検出
    let mut detector = MtDetector::new();
    let installations = match detector.detect_running_installations() {
        Ok(installs) => installs,
        Err(e) => {
            tracing::error!("Failed to detect running MT installations: {}", e);
            Vec::new()
        }
    };

    // 検出サマリーを作成
    let total_found = installations.len();
    let running = installations.iter().filter(|i| i.is_running).count();
    let stopped = total_found - running;

    let mut by_method: HashMap<String, usize> = HashMap::new();
    for installation in &installations {
        let method = match installation.detection_method {
            DetectionMethod::Process => "process",
        };
        *by_method.entry(method.to_string()).or_insert(0) += 1;
    }

    let response = MtInstallationsResponse {
        success: true,
        data: installations,
        detection_summary: DetectionSummary {
            total_found,
            by_method,
            running,
            stopped,
        },
    };

    Ok(Json(ApiResponse::success(response)))
}

/// MT4/MT5にコンポーネントをインストール
pub async fn install_to_mt(
    State(_state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<String>>, String> {
    tracing::info!("Installation request for MT installation ID: {}", id);

    // まず、起動中のMT4/MT5を検出して該当のものを探す
    let mut detector = MtDetector::new();
    let installations = match detector.detect_running_installations() {
        Ok(installs) => installs,
        Err(e) => {
            tracing::error!("Failed to detect running MT installations: {}", e);
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
            return Ok(Json(ApiResponse::error(format!(
                "指定されたID ({}) のMT4/MT5が見つかりません。MT4/MT5が起動しているか確認してください。",
                id
            ))));
        }
    };

    // 起動中の場合は警告
    if installation.is_running {
        tracing::warn!("MT4/MT5 is running. Installation may fail due to file locks.");
    }

    // インストーラーを作成
    let installer = MtInstaller::default();

    // インストール実行
    let mt_path = PathBuf::from(&installation.path);
    match installer.install(&mt_path, &installation.mt_type, &installation.platform) {
        Ok(_) => {
            tracing::info!("Installation completed successfully for {}", id);
            Ok(Json(ApiResponse::success(format!(
                "インストールが完了しました: {}",
                installation.name
            ))))
        }
        Err(e) => {
            tracing::error!("Installation failed for {}: {}", id, e);
            Ok(Json(ApiResponse::error(format!(
                "インストールに失敗しました: {}",
                e
            ))))
        }
    }
}
