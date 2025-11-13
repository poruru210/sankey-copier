use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

/// RFC 9457準拠のProblem Details構造体
/// https://www.rfc-editor.org/rfc/rfc9457.html
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemDetails {
    /// 問題タイプのURI参照
    /// 問題の種類を識別するURI。アプリケーション固有のエラータイプを表す
    #[serde(rename = "type")]
    pub type_uri: String,

    /// 人間が読める短い要約
    /// 問題タイプの簡潔な説明（ローカライズ可能）
    pub title: String,

    /// HTTPステータスコード
    /// このProblem Detailsが含まれるHTTPレスポンスのステータスコード
    pub status: u16,

    /// 問題の詳細説明
    /// この特定の問題発生についての人間が読める説明（ローカライズ可能）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,

    /// 問題が発生した特定のインスタンスを識別するURI参照
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,
}

impl ProblemDetails {
    /// 新しいProblemDetailsを作成
    pub fn new(type_uri: impl Into<String>, title: impl Into<String>, status: StatusCode) -> Self {
        Self {
            type_uri: type_uri.into(),
            title: title.into(),
            status: status.as_u16(),
            detail: None,
            instance: None,
        }
    }

    /// 詳細説明を設定
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// インスタンスURIを設定
    pub fn with_instance(mut self, instance: impl Into<String>) -> Self {
        self.instance = Some(instance.into());
        self
    }

    /// リソースが見つからない（404 Not Found）
    pub fn not_found(resource: impl Into<String>) -> Self {
        let status = StatusCode::NOT_FOUND;
        Self::new(
            "https://sankey-copier.example.com/errors/not-found",
            status.canonical_reason().unwrap_or("Not Found"),
            status,
        )
        .with_detail(format!("{}が見つかりません", resource.into()))
    }

    /// リソースが既に存在する（409 Conflict）
    pub fn conflict(detail: impl Into<String>) -> Self {
        let status = StatusCode::CONFLICT;
        Self::new(
            "https://sankey-copier.example.com/errors/conflict",
            status.canonical_reason().unwrap_or("Conflict"),
            status,
        )
        .with_detail(detail)
    }

    /// バリデーションエラー（400 Bad Request）
    #[allow(dead_code)]
    pub fn validation_error(detail: impl Into<String>) -> Self {
        let status = StatusCode::BAD_REQUEST;
        Self::new(
            "https://sankey-copier.example.com/errors/validation",
            status.canonical_reason().unwrap_or("Bad Request"),
            status,
        )
        .with_detail(detail)
    }

    /// 内部サーバーエラー（500 Internal Server Error）
    pub fn internal_error(detail: impl Into<String>) -> Self {
        let status = StatusCode::INTERNAL_SERVER_ERROR;
        Self::new(
            "https://sankey-copier.example.com/errors/internal",
            status.canonical_reason().unwrap_or("Internal Server Error"),
            status,
        )
        .with_detail(detail)
    }
}

impl IntoResponse for ProblemDetails {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.status)
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        // RFC 9457で規定されているContent-Type
        let mut response = (status, Json(self)).into_response();

        response.headers_mut().insert(
            axum::http::header::CONTENT_TYPE,
            axum::http::HeaderValue::from_static("application/problem+json"),
        );

        response
    }
}

/// APIResult型エイリアス
/// 成功時はT型の値を返し、エラー時はProblemDetailsを返す
#[allow(dead_code)]
pub type ApiResult<T> = Result<T, ProblemDetails>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_found() {
        let problem = ProblemDetails::not_found("Settings");
        assert_eq!(problem.status, 404);
        assert_eq!(problem.title, "Not Found");
        assert!(problem.detail.is_some());
    }

    #[test]
    fn test_conflict() {
        let problem = ProblemDetails::conflict("Duplicate entry");
        assert_eq!(problem.status, 409);
        assert_eq!(problem.title, "Conflict");
    }

    #[test]
    fn test_validation_error() {
        let problem = ProblemDetails::validation_error("Invalid input");
        assert_eq!(problem.status, 400);
        assert_eq!(problem.title, "Bad Request");
    }

    #[test]
    fn test_internal_error() {
        let problem = ProblemDetails::internal_error("Database error");
        assert_eq!(problem.status, 500);
        assert_eq!(problem.title, "Internal Server Error");
    }

    #[test]
    fn test_with_instance() {
        let problem = ProblemDetails::not_found("Settings")
            .with_instance("/api/settings/123");
        assert_eq!(problem.instance, Some("/api/settings/123".to_string()));
    }

    #[test]
    fn test_serialization() {
        let problem = ProblemDetails::not_found("Settings")
            .with_instance("/api/settings/123");
        let json = serde_json::to_value(&problem).unwrap();

        assert_eq!(json["type"], "https://sankey-copier.example.com/errors/not-found");
        assert_eq!(json["title"], "Not Found");
        assert_eq!(json["status"], 404);
        assert!(json["detail"].is_string());
        assert_eq!(json["instance"], "/api/settings/123");
    }
}
