// Location: mt-bridge/src/constants.rs
// Purpose: Shared constants between relay-server and MQL EAs
// Why: Single source of truth for protocol constants to ensure consistency
//
// NOTE: MQL side uses #define in Common.mqh. When updating these values,
// ensure mt-advisors/Include/SankeyCopier/Common.mqh is also updated.

// =============================================================================
// Connection Status Constants
// =============================================================================

/// Status indicating the connection is disabled by user
pub const STATUS_DISABLED: i32 = 0;

/// Status indicating the connection is enabled but not yet connected
pub const STATUS_ENABLED: i32 = 1;

/// Status indicating the connection is active and communicating
pub const STATUS_CONNECTED: i32 = 2;

/// Status indicating the EA has no configuration assigned
pub const STATUS_NO_CONFIG: i32 = -1;

// =============================================================================
// Message Type Constants
// =============================================================================

/// Periodic heartbeat from EA to relay-server
pub const MSG_TYPE_HEARTBEAT: &str = "heartbeat";

/// Trade signal from master to slaves
pub const MSG_TYPE_TRADE_SIGNAL: &str = "trade_signal";

/// Configuration request from EA to relay-server
pub const MSG_TYPE_REQUEST_CONFIG: &str = "request_config";

/// Position snapshot from slave EA
pub const MSG_TYPE_POSITION_SNAPSHOT: &str = "position_snapshot";

/// Sync request from slave EA (for position synchronization)
pub const MSG_TYPE_SYNC_REQUEST: &str = "sync_request";

/// Unregister message when EA is removed
pub const MSG_TYPE_UNREGISTER: &str = "unregister";

// =============================================================================
// Topic Constants
// =============================================================================

/// Global configuration topic for all EAs
pub const TOPIC_GLOBAL_CONFIG: &str = "config/global";

/// Prefix for account-specific config topics (format: "config/{account_id}")
pub const TOPIC_CONFIG_PREFIX: &str = "config/";

/// Prefix for trade topics (format: "trade/{account_id}")
pub const TOPIC_TRADE_PREFIX: &str = "trade/";

// =============================================================================
// Helper Functions for Topic Generation
// =============================================================================

/// Build a config topic for a specific account
/// Returns format: "config/{account_id}"
#[inline]
pub fn build_config_topic(account_id: &str) -> String {
    format!("{}{}", TOPIC_CONFIG_PREFIX, account_id)
}

/// Build a trade topic for a specific account
/// Returns format: "trade/{account_id}"
#[inline]
pub fn build_trade_topic(account_id: &str) -> String {
    format!("{}{}", TOPIC_TRADE_PREFIX, account_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_constants() {
        assert_eq!(STATUS_DISABLED, 0);
        assert_eq!(STATUS_ENABLED, 1);
        assert_eq!(STATUS_CONNECTED, 2);
        assert_eq!(STATUS_NO_CONFIG, -1);
    }

    #[test]
    fn test_topic_generation() {
        assert_eq!(build_config_topic("12345"), "config/12345");
        assert_eq!(build_trade_topic("12345"), "trade/12345");
        assert_eq!(TOPIC_GLOBAL_CONFIG, "config/global");
    }
}
