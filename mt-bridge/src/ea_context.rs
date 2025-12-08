// mt-bridge/src/ea_context.rs
//
// EA状態管理ロジック
//
// このモジュールは、MetaTrader EAの状態管理(特にRequestConfig送信判定)を
// Rust側で一元管理するためのロジックを提供します。
//
// ## 設計方針
// - Auto-trading状態に関わらず、未リクエスト時は常にRequestConfigを送信
// - 状態追跡はRust側で完結し、MQL側の計算に依存しない
// - FFI経由でMQL4/MT5、E2E Simulatorから共通利用

/// EA Context Manager
/// Holds both static account configuration and dynamic runtime state.
#[derive(Debug, Clone)]
pub struct EaContext {
    // --- Static Identity (Set via ea_init) ---
    pub account_id: String,
    pub ea_type: String,  // "Master" or "Slave"
    pub platform: String, // "MT4" or "MT5"
    pub account_number: i64,
    pub broker: String,
    pub account_name: String,
    pub server: String,
    pub currency: String,
    pub leverage: i64,

    // --- Runtime State ---
    /// Config request sent flag
    pub is_config_requested: bool,
    /// Last auto-trading state (for tracking changes)
    pub last_trade_allowed: bool,
}

impl EaContext {
    /// Create a new Context with static identity information
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        account_id: String,
        ea_type: String,
        platform: String,
        account_number: i64,
        broker: String,
        account_name: String,
        server: String,
        currency: String,
        leverage: i64,
    ) -> Self {
        Self {
            account_id,
            ea_type,
            platform,
            account_number,
            broker,
            account_name,
            server,
            currency,
            leverage,
            is_config_requested: false,
            last_trade_allowed: false,
        }
    }

    /// Determine if RequestConfig should be sent
    pub fn should_request_config(&mut self, current_trade_allowed: bool) -> bool {
        self.last_trade_allowed = current_trade_allowed;
        if self.is_config_requested {
            return false;
        }
        true
    }

    /// Mark that specific config has been requested
    pub fn mark_config_requested(&mut self) {
        self.is_config_requested = true;
    }

    /// Reset state (e.g. on reconnection)
    pub fn reset(&mut self) {
        self.is_config_requested = false;
    }
}

// Removing Default implementation as EaContext requires explicit initialization.

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_context() -> EaContext {
        EaContext::new(
            "test_acc".to_string(),
            "Master".to_string(),
            "MT5".to_string(),
            123456,
            "TestBroker".to_string(),
            "Test Account".to_string(),
            "TestServer".to_string(),
            "USD".to_string(),
            100,
        )
    }

    #[test]
    fn test_initial_state() {
        let ctx = create_test_context();
        assert!(!ctx.is_config_requested);
        assert!(!ctx.last_trade_allowed);
        assert_eq!(ctx.account_id, "test_acc");
        assert_eq!(ctx.broker, "TestBroker");
    }

    #[test]
    fn test_should_request_logic() {
        let mut ctx = create_test_context();
        // Should request initially
        assert!(ctx.should_request_config(true));

        // Mark as requested
        ctx.mark_config_requested();
        assert!(!ctx.should_request_config(true));

        // Reset
        ctx.reset();
        assert!(ctx.should_request_config(true));
    }
}
