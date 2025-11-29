// relay-server/src/api/zeromq_settings.rs
//
// REST API endpoints for ZeroMQ port configuration
// - GET /api/zeromq-config: Returns current port configuration

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use super::{AppState, ProblemDetails};

/// Response for GET /api/zeromq-config
/// Contains ZeroMQ port configuration (read-only, managed by runtime.toml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZeromqConfigResponse {
    /// PULL socket port (EA → Server)
    pub receiver_port: u16,
    /// PUB socket port (Trade signals)
    pub sender_port: u16,
    /// PUB socket port (Config messages)
    pub config_sender_port: u16,
    /// Whether ports are dynamically assigned (from runtime.toml) or fixed (from config.toml)
    pub is_dynamic: bool,
    /// When dynamic ports were generated (ISO 8601 format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_at: Option<String>,
}

/// GET /api/zeromq-config
/// Returns current ZeroMQ port configuration
pub async fn get_zeromq_config(
    State(state): State<AppState>,
) -> Result<Json<ZeromqConfigResponse>, ProblemDetails> {
    let span = tracing::info_span!("get_zeromq_config");
    let _enter = span.enter();

    let ports = &state.resolved_ports;

    let response = ZeromqConfigResponse {
        receiver_port: ports.receiver_port,
        sender_port: ports.sender_port,
        config_sender_port: ports.config_sender_port,
        is_dynamic: ports.is_dynamic,
        generated_at: ports.generated_at.map(|dt| dt.to_rfc3339()),
    };

    tracing::info!(
        receiver_port = ports.receiver_port,
        sender_port = ports.sender_port,
        config_sender_port = ports.config_sender_port,
        is_dynamic = ports.is_dynamic,
        "Retrieved ZeroMQ config"
    );

    Ok(Json(response))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zeromq_config_response_serialize() {
        let response = ZeromqConfigResponse {
            receiver_port: 5555,
            sender_port: 5556,
            config_sender_port: 5557,
            is_dynamic: false,
            generated_at: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("5555"));
        assert!(json.contains("5556"));
        assert!(json.contains("5557"));
        // generated_at should be skipped when None
        assert!(!json.contains("generated_at"));
    }

    #[test]
    fn test_zeromq_config_response_serialize_dynamic() {
        let response = ZeromqConfigResponse {
            receiver_port: 15555,
            sender_port: 15556,
            config_sender_port: 15557,
            is_dynamic: true,
            generated_at: Some("2024-01-15T10:30:00Z".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("15555"));
        assert!(json.contains("\"is_dynamic\":true"));
        assert!(json.contains("generated_at"));
    }
}
