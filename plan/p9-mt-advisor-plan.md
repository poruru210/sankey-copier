# P9 Phase4: MTアドバイザ更新計画

## 背景
- Status Engine + Config Builder により `runtime_status`/`allow_new_orders` がサーバ側で一元化され、MT EA は `CopyConfig.status` やログのローカル判定に頼らずこの情報を素直に表示・実行する必要がある。
- Relay Server の新設計では `runtime_status == CONNECTED` のときのみ `allow_new_orders` が `true` になるため、Slave 側ではこのフラグを起点に "新規オーダーを発注してよいか" を制御するのが正しい運用になる。
- Web UI・DB 側では `enabled_flag` と `runtime_status` の二層表示を前提としており、MT EA は `runtime_status` を表示/色分けするだけに留めることで三者の整合性を保つ。

## 目的
1. MT4/MT5 の Slave EA がサーバから渡される `status`/`allow_new_orders` を唯一の実行条件とし、ローカル判定や派生フラグを廃止する。
2. `ProcessConfigMessage`/`CopyConfig`/パネル表示の構造を調整して `runtime_status` の意味（CONNECTED/ENABLED/DISABLED）と `allow_new_orders` を正確に伝える。
3. ドキュメント/テストを更新し、MT EA 側の振る舞いが新ステータス仕様に沿っていることを明記・検証する。

## 対象コード
- `mt-advisors/Include/SankeyCopier/Trade.mqh` (`CopyConfig` へのフィールド流用、`ShouldProcessTrade` の判定)
- `mt-advisors/Include/SankeyCopier/GridPanel.mqh` (サマリ/ステータス表示、`UpdatePanelStatusFromConfigs` の判定)  
- `mt-advisors/MT4/SankeyCopierSlave.mq4` & `mt-advisors/MT5/SankeyCopierSlave.mq5` (トレードシグナルの `Open` 判定、ログ/オーダー実行パス)
- `tests/`~ (必要に応じて `test_zmq_communication.py` などで `allow_new_orders` の値が期待通りかを確認)

## 実装ステップ
1. `CopyConfig` に含まれる `status` をそのまま `runtime_status` と見なし、`allow_new_orders` を `ProcessConfigMessage` で `slave_config_get_bool("allow_new_orders")` から確実に反映
   - 既存の default のままで問題ないが、ログメッセージや `LogInfo` に `runtime_status` を含めて、Relay Server 側で計算された値であることを明示。
2. `ShouldProcessTrade` と各 EA の `ProcessTradeSignal` で `Open` だけ `allow_new_orders` をチェックし、ステータス表示用の `status` はログやパネル用に保持するだけにする。
   - `status` に `CONNECTED` 以外が入っていても `allow_new_orders` が `true` なら開く（現状は一致するが将来のメンテ性向上）。
   - `allow_new_orders` が `false` のときは `LogWarn` で理由を出力し、`Close`/`Modify` は常に通す。
3. `GridPanel::UpdateConfigList` と `UpdatePanelStatusFromConfigs` を `runtime_status` を元に色分けし、「WAIT (ENABLED)」「ON (CONNECTED)」「OFF (DISABLED)」を正しく表示。
   - `allow_new_orders` が `true` ならパネルのヘッダやステータス行に `trade ready` のような注釈を追加して、Web UI と同じ実効ステータスを示す。
4. MT4/MT5 の `ProcessTradeSignal` 内のログを `runtime_status` 独自表示に更新し、Trading decision log との齟齬を防ぐ。
5. 変更内容を `docs/relay-server.md` か `mt-advisors` 用の技術資料に追記し、`runtime_status`/`allow_new_orders` の意図とMT EA側の対応ポイントを明記する。

## テスト・検証
1. `tests/test_zmq_communication.py` などの Python レベルの MessagePack テストで `SlaveConfigMessage.allow_new_orders` が `runtime_status == STATUS_CONNECTED` のときだけ `True` になることを再確認（既存のテストがあれば値を固める）。
2. MT EA のログ出力を利用して、`allow_new_orders` ❤️`status`/`runtime_status` の組み合わせがサーバの `ConfigBuilder` から受信されていることを手動確認（必要なら `tests/e2e_trade_signal_test.rs` など Relay 側との統合で検証）。
3. `Web UI` の `runtime_status`/`enabled_flag` 表示との整合性確認（Phase4 P10 との連携）。
4. ビルドした MT4/MT5 EA を少なくとも1つのMTターミナルで起動し、`allow_new_orders=false` 時に `Open` シグナルが拒否されること、`allow_new_orders=true` 時に `Connected` を表示し `Open` が通ることを確認。

## 残作業予告
- MT EA 側のソースに具体的なコード変更を加えたら、`plan/status-engine-progress.md` の P9 行に完了日と備考を追記する。
- `docs/relay-server.md` のステータスセクションにも MT EA 連携ステップを追加して、開発者が Phase4 の変更を一望できるようにする。
- Phase4 のテスト/ドキュメント（P11）と Web UI 更新（P10）との調整を忘れず、MT EA の変更は後続フェーズと整合が取れていることを確認する。
