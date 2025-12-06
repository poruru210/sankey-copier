// mt-bridge/src/ea_state.rs
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

/// EA状態管理マネージャー
#[derive(Debug, Clone)]
pub struct EaState {
    /// Configリクエスト済みフラグ
    is_config_requested: bool,
    /// 最後のauto-trading状態 (状態追跡用)
    last_trade_allowed: bool,
}

impl EaState {
    /// 新規作成 (初期状態: 両フラグfalse)
    pub fn new() -> Self {
        Self {
            is_config_requested: false,
            last_trade_allowed: false,
        }
    }

    /// RequestConfig送信判定
    ///
    /// # Arguments
    /// * `current_trade_allowed` - 現在のauto-trading状態 (MQL側で取得)
    ///
    /// # Returns
    /// true: RequestConfigを送信すべき
    /// false: 送信不要
    ///
    /// # Logic
    /// 1. 既にリクエスト済み → false
    /// 2. 未リクエスト → true (Auto-trading状態に関わらず)
    ///
    /// # Note
    /// - 状態変化の検出もRust側で行う(last_trade_allowedとの比較)
    /// - Auto-trading OFFで起動してもRequestConfigを送信し、サーバーからDISABLEDステータスを受信する
    ///   (現在のMasterはAuto-trading ON時のみ送信するバグがあるが、このリファクタリングで修正)
    pub fn should_request_config(&mut self, current_trade_allowed: bool) -> bool {
        // 内部状態追跡を更新
        self.last_trade_allowed = current_trade_allowed;

        // 既にリクエスト済み
        if self.is_config_requested {
            return false;
        }

        // 未リクエストの場合は常にtrue (Auto-trading状態に関わらず送信)
        true
    }

    /// Config受信時にフラグをセット
    pub fn mark_config_requested(&mut self) {
        self.is_config_requested = true;
    }

    /// 再接続時等のリセット (将来拡張用)
    pub fn reset(&mut self) {
        self.is_config_requested = false;
    }
}

impl Default for EaState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let state = EaState::new();
        assert!(!state.is_config_requested);
        assert!(!state.last_trade_allowed);
    }

    #[test]
    fn test_should_request_when_trade_allowed_first_time() {
        let mut state = EaState::new();
        assert!(state.should_request_config(true));
    }

    #[test]
    fn test_should_request_even_when_trade_not_allowed() {
        // 重要: Auto-trading OFFでもRequestConfigを送信すべき
        let mut state = EaState::new();
        assert!(
            state.should_request_config(false),
            "Should request config even when trade is disabled"
        );
    }

    #[test]
    fn test_should_not_request_after_marked() {
        let mut state = EaState::new();
        assert!(state.should_request_config(true));
        state.mark_config_requested();
        assert!(!state.should_request_config(true));
    }

    #[test]
    fn test_state_change_tracking() {
        let mut state = EaState::new();
        state.should_request_config(false);
        assert!(!state.last_trade_allowed);
        state.should_request_config(true);
        assert!(state.last_trade_allowed);
    }

    #[test]
    fn test_reset_clears_config_requested() {
        let mut state = EaState::new();
        state.should_request_config(true);
        state.mark_config_requested();
        assert!(!state.should_request_config(true));

        state.reset();
        assert!(state.should_request_config(true));
    }

    #[test]
    fn test_multiple_calls_when_not_marked() {
        let mut state = EaState::new();
        assert!(state.should_request_config(true));
        assert!(state.should_request_config(true));
        assert!(state.should_request_config(false));

        state.mark_config_requested();
        assert!(!state.should_request_config(true));
        assert!(!state.should_request_config(false));
    }

    #[test]
    fn test_default_trait() {
        let state = EaState::default();
        assert!(!state.is_config_requested);
        assert!(!state.last_trade_allowed);
    }
}
