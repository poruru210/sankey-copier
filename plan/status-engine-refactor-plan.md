# Status Engine リファクタリング計画

## 背景
- Master/Slave ステータス判定が `heartbeat.rs` や API 層など複数箇所に分散し、修正漏れによる不整合が頻発。
- `trade_group_members.status` が「ユーザー意図 (ON/OFF)」と「サーバ計算済みステータス」の双方を兼ねており、状態が上書きされる。
- `allow_new_orders` など付帯フラグの算出が経路ごとに異なる。
- ドキュメント/テストが古い仕様のままで、設計との乖離が大きい。

## ゴール
1. ステータス計算・通知ロジックを単一モジュールに集約し、すべての呼び出し元を統一する。
2. DB モデルを "ユーザー意図" と "実効ステータス" に分離し、競合をなくす。
3. Config 生成/配信を共通ビルダー経由にして `allow_new_orders` などの付帯情報を揃える。
4. Web UI / MT アドバイザ / ドキュメント / テストを新仕様に同期。

## 非互換方針
- 本変更は未リリースバージョン前提のため後方互換性は考慮しない。
- 旧フィールドや暫定仕様は破棄し、クリーン実装を優先。

## スコープ
- `relay-server`: status engine、DB層、config送信経路、テスト。
- `mt-advisors`: Master/Slave EAのパネル表示とシグナル処理。
- `web-ui`: APIレスポンスへの追従、UI表示更新。
- `docs`: relay-server / mt-advisors / 開発ガイドライン。

## アーキテクチャ指針
### 1. Status Engine
- `relay-server/src/models/status_engine.rs` (新規) に以下をまとめる。
  - `MasterIntent` / `SlaveIntent` / `ConnectionSnapshot` 構造体。
  - ステータス計算 `evaluate_master_status(...)`, `evaluate_slave_status(...)`。
  - `allow_new_orders` 判定ロジック。
  - Master 複数接続時の "all masters connected" 判定。
- 既存 `status.rs` のテストを移管し、追加ケース (多Master、auto-trade OFF) を網羅。

### 2. DB モデル再設計
- `trade_group_members` に `enabled_flag` (ユーザー意図) と `runtime_status` (0/1/2) を分離。
- API/UI は `enabled_flag` を更新、Status Engine が `runtime_status` を書き込む。
- 既存 `status` カラムは移行後に削除 (マイグレーション step 参照)。

### 3. Config Builder
- `relay-server/src/config/builder.rs` (仮) を追加。
- `SlaveConfigMessage` / `MasterConfigMessage` 生成を共通化し、`allow_new_orders` や `timestamp` を一箇所で定義。
- `send_config_to_slave/slaves`, heartbeat, unregister, timeout 復旧すべてで Builder を使用。

### 4. クライアント同期
- MT4/MT5 EA: 受信した `status` を唯一のソースとし、ローカル判定で色変更しない。
- Web UI: API レスポンス (enabled_flag, runtime_status) をそのまま表示し、状態バッジを新ルールで実装。
- Docs+テスト: 新しい状態遷移表を共有し、古い記述を削除。

## 実装ステップ（最新版）
### フェーズA: 既存対応（完了）
1. Status Engine モジュール化とユニットテスト ✅
2. Heartbeat/API/ConfigRequest/Unregister の evaluate_* 置換 ✅
3. DB マイグレーション（`enabled_flag`／`runtime_status` 分離） ✅
4. Config Builder 適用・MT EA/Web UI 旧仕様削除 ✅

### フェーズB: 最終仕上げ（今回着手）
1. **イベント全網羅で runtime_status を更新**
   - Master/Slave Heartbeat を受信したら必ず最新の `ConnectionSnapshot` で Status Engine を実行し、`trade_group_members.runtime_status` と `trade_groups.master_settings.enabled` に反映。
   - Timeout / Unregister / VictoriaLogs 起動時も同じユーティリティを用いて即時反映。
2. **MasterCluster 再評価の共通化**
   - `get_masters_for_slave`→`MasterClusterSnapshot` 作成処理を `runtime_status_updater` (新規モジュール) に切り出し、Heartbeat/RequestConfig/API Toggle の全経路から再利用。
   - 多 Master 接続時でも一貫した `runtime_status=1` (Standby) 判定が得られるようにする。
3. **Intent Toggle/API 応答の即時反映**
   - `/api/trade-group-members/{id}/toggle` 等で `enabled_flag` 更新後、同じリクエスト内で Status Engine を再評価し DB へ保存→ZMQ 送信。Web UI はサーバ通知を待つだけで良くなる。
4. **warning_codes の精緻化**
   - `warning_codes` を Master/Slave 別の enum として管理し、未接続 Master 名や `is_trade_allowed=false` など原因を付与。Web UI への通知構造体を拡張し、Colour override の理由をロギング。
5. **ユーティリティ/監視整備**
   - `RuntimeStatusUpdater` (仮) を追加し、status-engine 呼び出し + DB 更新 + ログ出力を統合。
   - `tracing` target `status_engine` でメトリクス化し、Prometheus/VictoriaLogs へ送る Hooks を準備。
6. **テスト/ドキュメント刷新**
   - Heartbeat だけで Slave runtime が変化するケース、Master 復帰待ちケース等の自動テストを `message_handler/tests` に追加。
   - `e2e_*` テストに Standby 表示/警告の回帰シナリオを増強。
   - `docs/runtime-status-alignment.md`／`docs/api-specification.md`／`docs/architecture.md` を今回の設計で再記述し、UI 側補正 (B) が不要になった旨を明記。

### フェーズC: リリース準備
1. 旧 `status` カラムと冗長ログ削除。
2. モニタリングダッシュボード更新（Master/Slave runtime_status 分布、警告件数）。
3. インストーラ/リリースノート更新、QA チェックリスト反映。

## リスクと対策
- **移行期間中の不整合**: 新旧ロジック比較ログで検知、feature flag で切替。
- **SQLite マイグレーション失敗**: 事前バックアップとリハーサル。
- **MT EA アップデート漏れ**: バージョン番号更新とインストーラ連携で強制更新。

## 成功判定
- すべての config 送信パスが Status Engine/Builder 経由。
- `enabled_flag` が UI 操作のみで変化し、`runtime_status` がサーバ計算のみで更新されていること。
- Web UI/EA の表示とサーバログが一致。
- 新 E2E/ユニットテストがパスし、旧テスト資産を置き換え済み。
