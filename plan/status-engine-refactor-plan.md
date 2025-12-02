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

## 実装ステップ
1. **準備**
   - 影響範囲調査・既存ログ取得。
   - `status_engine.rs` ひな形と単体テスト追加。→ ✅ 2025-12-02 完了

2. **Status Engine 適用**
   - `heartbeat.rs` → 新API。
   - ✅ `evaluate_*` への切替済み (2025-12-02)
   - ✅ `api/trade_group_members.rs`, `api/trade_groups.rs` を新 Engine 経由に更新 (2025-12-06)
   - ✅ `message_handler/unregister.rs` / timeout ハンドラの通知経路を status engine 化 (2025-12-06)
   - ✅ `message_handler/config_request.rs` を新 Engine 経由に更新 (2025-12-06)
   - 旧ロジックと新ロジックの差分ログを一時的に出力。

3. **DB マイグレーション**
   - SQLite: `ALTER TABLE trade_group_members ADD COLUMN enabled_flag INTEGER DEFAULT 0;`
   - ブート時に `enabled_flag` が NULL の行へ `status > 0` をコピー。
   - API/UI を `enabled_flag` ベースに更新。
   - `runtime_status` 専用カラムを追加し、Status Engine 出力を保存。

### Phase2 残タスク詳細 (API/UI)
1. **API 拡張**
   - `TradeGroupMember` / `SlaveConfigWithMaster` のシリアライズに `enabled_flag` と `runtime_status` を必ず含め、Web UI で追加フィールドにアクセスできるようにする。
   - `/api/trade-groups/{id}` レスポンスへ `master_runtime_status` を追加し、Master ステータスも Web UI が参照できるようにする (Status Engine の結果を `master_status_snapshot` キャッシュに保存)。
   - WebSocket 通知 (`member_*`, `settings_updated`, `trade_group_updated`) も同じ構造体を返すよう統一し、フロント側で追加ロジック不要にする。
   - 旧 `status` ベースのトグル API を正式に廃止して `enabled_flag` を唯一の入口とする (現状コードは対応済みだが API 契約書の更新とレスポンス整形が必要)。
2. **Web UI 更新**
   - `useMembers()` などの SWR hooks を `enabled_flag` + `runtime_status` を保持する shape に変更し、表示ロジックから `status` 参照をなくす。
   - Graph ノード / 詳細パネルの状態バッジを `enabled_flag` (ユーザー意図) と `runtime_status` (実際の稼働) の 2 層表示へ刷新。例: "手動OFF" / "準備完了" / "接続中"。
   - トグル UI は `enabled_flag` だけを切り替え、応答で返る `runtime_status` を optimistic 更新せずサーバ通知を待つようにする。
   - `allow_new_orders` 表示は Status Engine の結果から計算された値を使い、クライアント側計算を削除。
   - i18n 辞書 (`*.content.ts`) とヘルプツールチップを新しい用語に合わせて更新。
3. **テスト/ドキュメント**
   - API 契約書 (`docs/api-specification.md`) に `enabled_flag` / `runtime_status` を追加し、旧 `status` の説明を削除。
   - Web UI E2E (`web-ui/__tests__`) でスイッチ操作後に `enabled_flag` が保持され、`runtime_status` がハートビート後に更新されるケースを追加。
   - Playwright テストのモック API も新レスポンスに合わせて更新。

4. **Config Builder 導入**
   - 新 Builder を作成し、すべての config 送信経路を移行。
   - `allow_new_orders` を `runtime_status == CONNECTED` に一本化。

5. **クライアントとドキュメント更新**
   - MTアドバイザ: `ProcessConfigMessage` とパネルロジックを新仕様に変更。
   - Web UI: 表示/操作 UI と API クライアントを更新。
   - Docs/E2Eテストを最新仕様に改定。

6. **クリーンアップ**
   - 旧 `status` カラム削除、不要ログ/flag撤去。
   - 監視メトリクス整備、ステータス異常検知を追加。

## リスクと対策
- **移行期間中の不整合**: 新旧ロジック比較ログで検知、feature flag で切替。
- **SQLite マイグレーション失敗**: 事前バックアップとリハーサル。
- **MT EA アップデート漏れ**: バージョン番号更新とインストーラ連携で強制更新。

## 成功判定
- すべての config 送信パスが Status Engine/Builder 経由。
- `enabled_flag` が UI 操作のみで変化し、`runtime_status` がサーバ計算のみで更新されていること。
- Web UI/EA の表示とサーバログが一致。
- 新 E2E/ユニットテストがパスし、旧テスト資産を置き換え済み。
