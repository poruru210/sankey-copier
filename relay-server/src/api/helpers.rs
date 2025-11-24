//! Helper functions for the Relay Server API
//!
//! Contains utility functions for settings cache management,
//! status calculation, and configuration message building.

use crate::{
    api::AppState,
    models::{CopySettings, EaConnection, SlaveConfigMessage},
};

/// Refresh the settings cache from the database
pub async fn refresh_settings_cache(state: &AppState) {
    if let Ok(all_settings) = state.db.list_copy_settings().await {
        *state.settings_cache.write().await = all_settings;
    }
}

/// Calculate effective status based on settings status and master connection state
///
/// Logic:
/// - If slave disabled (status == 0) -> DISABLED (0)
/// - If slave enabled (status == 1 or 2):
///   - If master connected and trade allowed -> CONNECTED (2)
///   - Otherwise -> ENABLED (1)
pub fn calculate_effective_status(settings_status: i32, master_conn: Option<&EaConnection>) -> i32 {
    if settings_status == 0 {
        // Slave disabled -> DISABLED
        0
    } else {
        // Slave enabled (status == 1 or 2)
        match master_conn {
            Some(conn) if conn.is_trade_allowed => {
                // Both master trade-allowed and slave enabled -> CONNECTED
                2
            }
            _ => {
                // Master not ready or trade not allowed -> ENABLED (but not connected)
                1
            }
        }
    }
}

/// Build SlaveConfigMessage with calculated effective status based on Master connection state
///
/// CopySettingsから完全な設定情報を含むSlaveConfigMessageを生成します。
/// マスターEAの接続状態を確認し、実効ステータスを計算します。
pub async fn build_config_message(state: &AppState, settings: &CopySettings) -> SlaveConfigMessage {
    // Check if Master is connected and has trading allowed
    let master_conn = state
        .connection_manager
        .get_ea(&settings.master_account)
        .await;

    let effective_status = calculate_effective_status(settings.status, master_conn.as_ref());

    SlaveConfigMessage {
        account_id: settings.slave_account.clone(),
        master_account: settings.master_account.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        status: effective_status,
        lot_multiplier: settings.lot_multiplier,
        reverse_trade: settings.reverse_trade,
        symbol_mappings: settings.symbol_mappings.clone(),
        filters: settings.filters.clone(),
        config_version: 1,
        symbol_prefix: settings.symbol_prefix.clone(),
        symbol_suffix: settings.symbol_suffix.clone(),
    }
}

/// Send configuration message to slave EA via ZeroMQ
///
/// CopySettingsから完全な設定情報を含むSlaveConfigMessageを生成し、
/// ZeroMQ経由でSlaveEAに送信します。
pub async fn send_config_to_ea(state: &AppState, settings: &CopySettings) {
    // Build SlaveConfigMessage with calculated effective status
    let config = build_config_message(state, settings).await;

    if let Err(e) = state.config_sender.send(&config).await {
        tracing::error!(
            "Failed to send config message to {}: {}",
            settings.slave_account,
            e
        );
    } else {
        tracing::info!(
            "Sent full config to EA: {} (master: {}, db_status: {}, effective_status: {}, lot_mult: {:?})",
            settings.slave_account,
            settings.master_account,
            settings.status,
            config.status,
            settings.lot_multiplier
        );
    }
}
