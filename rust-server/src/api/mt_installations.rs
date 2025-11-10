use axum::{
    extract::State,
    Json,
};
use std::collections::HashMap;

use crate::models::{DetectionMethod, DetectionSummary, MtInstallationsResponse};
use crate::mt_detector::MtDetector;

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
