use axum::{
    extract::{Path, State},
    Json,
};
use std::collections::HashMap;

use crate::models::{Architecture, DetectionMethod, DetectionSummary, MtInstallation, MtInstallationsResponse, MtType};
use crate::mt_detector::MtDetector;

use super::{ApiResponse, AppState};

/// MT4/MT5インストール一覧を取得
pub async fn list_mt_installations(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<MtInstallationsResponse>>, String> {
    let db = &state.db;
    // プロセスから起動中のMT4/MT5を検出
    let mut detector = MtDetector::new();
    let mut installations = match detector.detect_running_installations() {
        Ok(installs) => installs,
        Err(e) => {
            tracing::error!("Failed to detect running MT installations: {}", e);
            Vec::new()
        }
    };

    // データベースから手動で追加されたインストールを取得
    match db.get_manual_installations().await {
        Ok(manual_installs) => {
            for (id, path, executable, mt_type_str, platform_str) in manual_installs {
                // すでにプロセスとして検出されている場合はスキップ
                if installations.iter().any(|i| i.id == id) {
                    continue;
                }

                // データベースのパスからMT installation情報を復元
                let _mt_type = match mt_type_str.as_str() {
                    "MT4" => MtType::MT4,
                    "MT5" => MtType::MT5,
                    _ => continue,
                };

                let _platform = match platform_str.as_str() {
                    "32-bit" => Architecture::Bit32,
                    "64-bit" => Architecture::Bit64,
                    _ => continue,
                };

                // パスが存在するか確認
                let path_buf = std::path::PathBuf::from(&path);
                if !path_buf.exists() {
                    tracing::warn!("Manual installation path no longer exists: {}", path);
                    continue;
                }

                // 詳細情報を取得
                match detector.analyze_installation(&path_buf, None) {
                    Ok(mut installation) => {
                        // 検出方法をManualにオーバーライド
                        installation.detection_method = DetectionMethod::Manual;
                        installations.push(installation);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to analyze manual installation at {}: {}", path, e);
                    }
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to get manual installations: {}", e);
        }
    }

    // 検出サマリーを作成
    let total_found = installations.len();
    let running = installations.iter().filter(|i| i.is_running).count();
    let stopped = total_found - running;

    let mut by_method: HashMap<String, usize> = HashMap::new();
    for installation in &installations {
        let method = match installation.detection_method {
            DetectionMethod::Process => "process",
            DetectionMethod::Manual => "manual",
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

/// 手動でMT4/MT5インストールを追加
#[derive(serde::Deserialize)]
pub struct AddManualInstallationRequest {
    pub path: String,
}

pub async fn add_manual_installation(
    State(state): State<AppState>,
    Json(req): Json<AddManualInstallationRequest>,
) -> Result<Json<ApiResponse<MtInstallation>>, String> {
    let db = &state.db;
    let path_buf = std::path::PathBuf::from(&req.path);

    // パスが存在するか確認
    if !path_buf.exists() {
        return Ok(Json(ApiResponse::error(
            "指定されたパスが存在しません".to_string(),
        )));
    }

    // terminal.exe または terminal64.exe かチェック
    let file_name = path_buf
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    if file_name != "terminal.exe" && file_name != "terminal64.exe" {
        return Ok(Json(ApiResponse::error(
            "選択されたファイルは有効なMT4/MT5実行ファイルではありません。terminal.exe または terminal64.exe を選択してください".to_string(),
        )));
    }

    let base_path = path_buf.parent().ok_or_else(|| {
        "親ディレクトリを取得できませんでした".to_string()
    })?;

    // インストール情報を分析
    let detector = MtDetector::new();
    let mut installation = match detector.analyze_installation(base_path, None) {
        Ok(inst) => inst,
        Err(e) => {
            return Ok(Json(ApiResponse::error(format!(
                "MT4/MT5インストールの分析に失敗しました: {}",
                e
            ))));
        }
    };

    // 検出方法をManualにオーバーライド
    installation.detection_method = DetectionMethod::Manual;

    // データベースに保存
    let mt_type_str = match installation.mt_type {
        MtType::MT4 => "MT4",
        MtType::MT5 => "MT5",
    };

    let platform_str = match installation.platform {
        Architecture::Bit32 => "32-bit",
        Architecture::Bit64 => "64-bit",
    };

    if let Err(e) = db
        .save_manual_installation(
            &installation.id,
            &installation.path,
            &installation.executable,
            mt_type_str,
            platform_str,
        )
        .await
    {
        return Ok(Json(ApiResponse::error(format!(
            "データベースへの保存に失敗しました: {}",
            e
        ))));
    }

    Ok(Json(ApiResponse::success(installation)))
}

/// 手動で追加したMT4/MT5インストールを削除
pub async fn remove_manual_installation(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, String> {
    let db = &state.db;
    match db.delete_manual_installation(&id).await {
        Ok(_) => Ok(Json(ApiResponse::success(()))),
        Err(e) => Ok(Json(ApiResponse::error(format!(
            "削除に失敗しました: {}",
            e
        )))),
    }
}
