// Location: mt-bridge/src/constants.rs
// Purpose: Shared constants between relay-server and MQL EAs
// Why: Single source of truth for protocol constants to ensure consistency
//
// NOTE: MQL side uses #define in Common.mqh. When updating these values,
// ensure mt-advisors/Include/SankeyCopier/Common.mqh is also updated.

use serde::{Deserialize, Serialize};

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
// Order Type Enum
// =============================================================================

/// Order type enumeration matching MQL's ORDER_TYPE_* / OP_* constants
/// Serializes to PascalCase strings (e.g., "Buy", "SellLimit")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderType {
    Buy,
    Sell,
    BuyLimit,
    SellLimit,
    BuyStop,
    SellStop,
}

impl OrderType {
    /// Convert from MQL numeric order type
    /// MT5: ORDER_TYPE_BUY=0, ORDER_TYPE_SELL=1, etc.
    /// MT4: OP_BUY=0, OP_SELL=1, etc.
    pub fn from_mql(value: i32) -> Option<Self> {
        match value {
            0 => Some(OrderType::Buy),
            1 => Some(OrderType::Sell),
            2 => Some(OrderType::BuyLimit),
            3 => Some(OrderType::SellLimit),
            4 => Some(OrderType::BuyStop),
            5 => Some(OrderType::SellStop),
            _ => None,
        }
    }

    /// Convert to MQL numeric order type
    pub fn to_mql(&self) -> i32 {
        match self {
            OrderType::Buy => 0,
            OrderType::Sell => 1,
            OrderType::BuyLimit => 2,
            OrderType::SellLimit => 3,
            OrderType::BuyStop => 4,
            OrderType::SellStop => 5,
        }
    }

    /// Check if this is a market order (Buy/Sell)
    pub fn is_market(&self) -> bool {
        matches!(self, OrderType::Buy | OrderType::Sell)
    }

    /// Check if this is a pending order (Limit/Stop)
    pub fn is_pending(&self) -> bool {
        !self.is_market()
    }
}

// =============================================================================
// Trade Action Enum
// =============================================================================

/// Trade action enumeration for trade signals
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeAction {
    Open,
    Close,
    Modify,
}

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

    #[test]
    fn test_order_type_from_mql() {
        assert_eq!(OrderType::from_mql(0), Some(OrderType::Buy));
        assert_eq!(OrderType::from_mql(1), Some(OrderType::Sell));
        assert_eq!(OrderType::from_mql(2), Some(OrderType::BuyLimit));
        assert_eq!(OrderType::from_mql(3), Some(OrderType::SellLimit));
        assert_eq!(OrderType::from_mql(4), Some(OrderType::BuyStop));
        assert_eq!(OrderType::from_mql(5), Some(OrderType::SellStop));
        assert_eq!(OrderType::from_mql(99), None);
    }

    #[test]
    fn test_order_type_to_mql() {
        assert_eq!(OrderType::Buy.to_mql(), 0);
        assert_eq!(OrderType::Sell.to_mql(), 1);
        assert_eq!(OrderType::BuyLimit.to_mql(), 2);
        assert_eq!(OrderType::SellLimit.to_mql(), 3);
        assert_eq!(OrderType::BuyStop.to_mql(), 4);
        assert_eq!(OrderType::SellStop.to_mql(), 5);
    }

    #[test]
    fn test_order_type_classification() {
        assert!(OrderType::Buy.is_market());
        assert!(OrderType::Sell.is_market());
        assert!(!OrderType::BuyLimit.is_market());
        assert!(OrderType::BuyLimit.is_pending());
        assert!(OrderType::SellStop.is_pending());
    }
}
