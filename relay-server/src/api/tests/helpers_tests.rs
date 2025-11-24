//! Tests for helper functions
//!
//! Tests for calculate_effective_status, build_config_message,
//! refresh_settings_cache, and related functionality.

use super::*;
use crate::api::helpers::*;

#[test]
fn test_calculate_effective_status_slave_disabled() {
    // Case 1: Slave is disabled (status 0)
    // Should be 0 regardless of master state
    assert_eq!(calculate_effective_status(0, None), 0);

    let conn = create_test_connection(true);
    assert_eq!(calculate_effective_status(0, Some(&conn)), 0);
}

#[test]
fn test_calculate_effective_status_slave_enabled_master_offline() {
    // Case 2: Slave enabled (1), Master offline (None)
    // Should be 1 (ENABLED)
    assert_eq!(calculate_effective_status(1, None), 1);
}

#[test]
fn test_calculate_effective_status_slave_enabled_master_trade_allowed() {
    // Case 3: Slave enabled (1), Master online & trade allowed
    // Should be 2 (CONNECTED)
    let conn = create_test_connection(true);
    assert_eq!(calculate_effective_status(1, Some(&conn)), 2);
}

#[test]
fn test_calculate_effective_status_slave_enabled_master_trade_not_allowed() {
    // Case 4: Slave enabled (1), Master online but trade NOT allowed
    // Should be 1 (ENABLED)
    let conn = create_test_connection(false);
    assert_eq!(calculate_effective_status(1, Some(&conn)), 1);
}

#[tokio::test]
async fn test_build_config_message_master_online_trade_allowed() {
    // Test ConfigMessage building when Master is online with trade allowed
    let state = create_test_app_state().await;
    let settings = create_test_copy_settings();

    // Register master connection with trade allowed
    let heartbeat = create_test_heartbeat("MASTER123", true);
    state.connection_manager.update_heartbeat(heartbeat).await;

    let config = build_config_message(&state, &settings).await;

    // Verify config fields
    assert_eq!(config.account_id, "SLAVE456");
    assert_eq!(config.master_account, "MASTER123");
    assert_eq!(config.status, 2); // CONNECTED (slave enabled + master trade allowed)
    assert_eq!(config.lot_multiplier, Some(2.0));
    assert!(!config.reverse_trade);
    assert_eq!(config.symbol_mappings.len(), 1);
    assert_eq!(config.symbol_prefix, Some("pro.".to_string()));
    assert_eq!(config.symbol_suffix, Some(".m".to_string()));
}

#[tokio::test]
async fn test_build_config_message_master_offline() {
    // Test ConfigMessage building when Master is offline
    let state = create_test_app_state().await;
    let settings = create_test_copy_settings();

    // No master connection registered
    let config = build_config_message(&state, &settings).await;

    // Verify status is ENABLED (1) when master is offline
    assert_eq!(config.status, 1);
    assert_eq!(config.account_id, "SLAVE456");
    assert_eq!(config.master_account, "MASTER123");
}

#[tokio::test]
async fn test_build_config_message_master_trade_not_allowed() {
    // Test ConfigMessage building when Master is online but trade not allowed
    let state = create_test_app_state().await;
    let settings = create_test_copy_settings();

    // Register master connection with trade NOT allowed
    let heartbeat = create_test_heartbeat("MASTER123", false);
    state.connection_manager.update_heartbeat(heartbeat).await;

    let config = build_config_message(&state, &settings).await;

    // Verify status is ENABLED (1) when master trade is not allowed
    assert_eq!(config.status, 1);
}

#[tokio::test]
async fn test_build_config_message_slave_disabled() {
    // Test ConfigMessage building when Slave is disabled
    let state = create_test_app_state().await;
    let mut settings = create_test_copy_settings();
    settings.status = 0; // Disabled

    // Register master connection with trade allowed
    let heartbeat = create_test_heartbeat("MASTER123", true);
    state.connection_manager.update_heartbeat(heartbeat).await;

    let config = build_config_message(&state, &settings).await;

    // Verify status is DISABLED (0) regardless of master state
    assert_eq!(config.status, 0);
}

#[tokio::test]
async fn test_refresh_settings_cache_success() {
    // Test successful cache refresh
    let state = create_test_app_state().await;
    let mut settings = create_test_copy_settings();
    settings.id = 0; // Use 0 for new record creation

    // Save settings to DB
    state.db.save_copy_settings(&settings).await.unwrap();

    // Initial cache should be empty
    assert_eq!(state.settings_cache.read().await.len(), 0);

    // Refresh cache
    refresh_settings_cache(&state).await;

    // Cache should now contain the setting
    let cache = state.settings_cache.read().await;
    assert_eq!(cache.len(), 1);
    assert_eq!(cache[0].master_account, "MASTER123");
    assert_eq!(cache[0].slave_account, "SLAVE456");
}

#[tokio::test]
async fn test_refresh_settings_cache_empty_db() {
    // Test cache refresh with empty database
    let state = create_test_app_state().await;

    // Refresh cache with empty DB
    refresh_settings_cache(&state).await;

    // Cache should remain empty
    assert_eq!(state.settings_cache.read().await.len(), 0);
}

#[tokio::test]
async fn test_build_config_message_with_null_values() {
    // Test ConfigMessage building with null optional fields
    let state = create_test_app_state().await;
    let mut settings = create_test_copy_settings();
    settings.lot_multiplier = None;
    settings.symbol_prefix = None;
    settings.symbol_suffix = None;
    settings.symbol_mappings = vec![];

    let config = build_config_message(&state, &settings).await;

    // Verify null fields are correctly passed through
    assert_eq!(config.lot_multiplier, None);
    assert_eq!(config.symbol_prefix, None);
    assert_eq!(config.symbol_suffix, None);
    assert_eq!(config.symbol_mappings.len(), 0);
}
