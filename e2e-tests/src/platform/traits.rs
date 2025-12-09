// e2e-tests/src/platform/traits.rs
//
// ExpertAdvisor Trait Definition
//
// すべてのExpert Advisor (EA) シミュレータが実装すべきトレイト。
// MQL5のイベントハンドラ関数をモデル化しています。

use crate::platform::types::{
    ENUM_DEINIT_REASON, ENUM_INIT_RETCODE, MqlTradeRequest, MqlTradeResult, MqlTradeTransaction,
};

/// MQL5のExpert Advisorを表すトレイト
pub trait ExpertAdvisor: Send + Sync {
    /// OnInitイベントハンドラ
    /// EAの初期化時に呼び出されます。
    fn on_init(&mut self) -> ENUM_INIT_RETCODE {
        ENUM_INIT_RETCODE::INIT_SUCCEEDED
    }

    /// OnDeinitイベントハンドラ
    /// EAの終了時に呼び出されます。
    fn on_deinit(&mut self, _reason: ENUM_DEINIT_REASON) {}

    /// OnTickイベントハンドラ
    /// 新しいティック（価格更新）が発生したときに呼び出されます。
    fn on_tick(&mut self) {}

    /// OnTimerイベントハンドラ
    /// タイマーイベントが発生したときに呼び出されます。
    /// 事前にEventSetTimerでタイマーを設定する必要があります。
    fn on_timer(&mut self) {}

    /// OnTradeイベントハンドラ
    /// 取引操作が完了したときに呼び出されます。
    fn on_trade(&mut self) {}

    /// OnTradeTransactionイベントハンドラ
    /// 取引トランザクションの結果を処理するために呼び出されます。
    fn on_trade_transaction(
        &mut self,
        _trans: &MqlTradeTransaction,
        _request: &MqlTradeRequest,
        _result: &MqlTradeResult,
    ) {
    }
}
