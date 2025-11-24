//! Tests for trade signal message handling

use super::*;

#[tokio::test]
async fn test_handle_trade_signal_with_matching_setting() {
    let handler = create_test_handler().await;
    let signal = create_test_trade_signal();
    let settings = create_test_copy_settings();

    // Add settings to cache
    {
        let mut cache = handler.settings_cache.write().await;
        cache.push(settings);
    }

    // Process trade signal (should not panic)
    handler.handle_trade_signal(signal).await;
}

#[tokio::test]
async fn test_handle_trade_signal_no_matching_master() {
    let handler = create_test_handler().await;
    let mut signal = create_test_trade_signal();
    signal.source_account = "OTHER_MASTER".to_string();
    let settings = create_test_copy_settings();

    // Add settings to cache
    {
        let mut cache = handler.settings_cache.write().await;
        cache.push(settings);
    }

    // Process trade signal (should be filtered out, no panic)
    handler.handle_trade_signal(signal).await;
}

#[tokio::test]
async fn test_handle_trade_signal_disabled_setting() {
    let handler = create_test_handler().await;
    let signal = create_test_trade_signal();
    let mut settings = create_test_copy_settings();
    settings.status = 0; // STATUS_DISABLED

    // Add settings to cache
    {
        let mut cache = handler.settings_cache.write().await;
        cache.push(settings);
    }

    // Process trade signal (should be filtered out, no panic)
    handler.handle_trade_signal(signal).await;
}
