// e2e-tests/src/platform/runner.rs
//
// Platform Runner (Event Loop)
//
// MQL5の実行環境（プラットフォーム）をシミュレートするランナー。
// 単一のスレッドでイベントループを回し、EAのイベントハンドラを順次呼び出します。
// OnTickやOnTimerは、前のイベント処理が完了していない場合、スキップされる挙動（またはキューイング）を制御します。
// ※MQL5では一般的にイベントはキューイングされますが、スタックオーバーフローを防ぐために特定のイベントは間引かれることがあります。
//   ここではシンプルにチャネルを使用したキューイングモデルを採用し、単一スレッドで消費します。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use crate::platform::traits::ExpertAdvisor;
use crate::platform::types::{
    ENUM_DEINIT_REASON, ENUM_INIT_RETCODE, MqlTradeRequest, MqlTradeResult, MqlTradeTransaction,
};

/// プラットフォームで発生するイベントの種類
pub enum PlatformEvent {
    Init,
    Deinit(ENUM_DEINIT_REASON),
    Timer,
    Tick, // 引数としてMqlTickを渡すことも考えられるが、通常はSymbolInfoTickで取得する
    Trade,
    TradeTransaction(MqlTradeTransaction, MqlTradeRequest, MqlTradeResult),
    Shutdown,
}

/// EAを実行するプラットフォームシミュレータ
pub struct PlatformRunner {
    /// イベント送信用のチャネル送信端
    sender: std::sync::mpsc::Sender<PlatformEvent>,
    /// イベントループスレッドのハンドル
    thread_handle: Option<JoinHandle<()>>,
    /// 実行中フラグ
    running: Arc<AtomicBool>,
}

impl PlatformRunner {
    /// 新しいプラットフォームランナーを作成し、EAの実行を開始します。
    pub fn new<E>(mut ea: E) -> Self
    where
        E: ExpertAdvisor + 'static,
    {
        let (sender, receiver) = std::sync::mpsc::channel();
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();

        let thread_handle = thread::spawn(move || {
            // イベントループ
            while let Ok(event) = receiver.recv() {
                if !running_clone.load(Ordering::SeqCst) {
                    break;
                }

                match event {
                    PlatformEvent::Init => {
                        let ret = ea.on_init();
                        if ret != ENUM_INIT_RETCODE::INIT_SUCCEEDED {
                            // 初期化失敗時の処理（Deinitを呼んで終了など）
                            // 簡易実装としてログ出力してループを継続（または終了）
                            eprintln!("EA Initialization failed: {:?}", ret);
                            ea.on_deinit(ENUM_DEINIT_REASON::REASON_INITFAILED);
                            break;
                        }
                    }
                    PlatformEvent::Deinit(reason) => {
                        ea.on_deinit(reason);
                        // Deinit後はループを抜けるのが一般的だが、Shutdownイベントで制御する
                    }
                    PlatformEvent::Timer => {
                        ea.on_timer();
                    }
                    PlatformEvent::Tick => {
                        ea.on_tick();
                    }
                    PlatformEvent::Trade => {
                        ea.on_trade();
                    }
                    PlatformEvent::TradeTransaction(trans, req, res) => {
                        ea.on_trade_transaction(&trans, &req, &res);
                    }
                    PlatformEvent::Shutdown => {
                        break;
                    }
                }
            }
        });

        // 初期化イベントを送信
        let _ = sender.send(PlatformEvent::Init);

        Self {
            sender,
            thread_handle: Some(thread_handle),
            running,
        }
    }

    /// OnTimerイベントをスケジュールします（外部からのトリガー）
    pub fn send_timer(&self) {
        let _ = self.sender.send(PlatformEvent::Timer);
    }

    /// OnTickイベントをスケジュールします
    pub fn send_tick(&self) {
        let _ = self.sender.send(PlatformEvent::Tick);
    }

    /// OnTradeイベントをスケジュールします
    pub fn send_trade(&self) {
        let _ = self.sender.send(PlatformEvent::Trade);
    }

    /// OnTradeTransactionイベントをスケジュールします
    pub fn send_trade_transaction(
        &self,
        trans: MqlTradeTransaction,
        req: MqlTradeRequest,
        res: MqlTradeResult,
    ) {
        let _ = self
            .sender
            .send(PlatformEvent::TradeTransaction(trans, req, res));
    }

    /// プラットフォームを停止します（OnDeinitを呼び出してスレッドを終了）
    pub fn stop(&mut self, reason: ENUM_DEINIT_REASON) {
        if self.running.load(Ordering::SeqCst) {
            let _ = self.sender.send(PlatformEvent::Deinit(reason));
            let _ = self.sender.send(PlatformEvent::Shutdown);
            if let Some(handle) = self.thread_handle.take() {
                let _ = handle.join();
            }
            self.running.store(false, Ordering::SeqCst);
        }
    }
}

impl Drop for PlatformRunner {
    fn drop(&mut self) {
        // デフォルトの終了処理（まだ実行中の場合）
        if self.running.load(Ordering::SeqCst) {
            self.stop(ENUM_DEINIT_REASON::REASON_REMOVE);
        }
    }
}
