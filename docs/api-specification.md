- `warning_codes` は REST / WebSocket / ZMQ Config すべてで同じリストが返るため、UI・EA・サポートが統一的に原因を把握できる。

## Warning Codes リファレンス

| コード | 発生条件 | 推奨対応 |
|--------|----------|-----------|
| `slave_web_ui_disabled` | Web UI で Slave Intent が OFF | UI でトグルを ON に戻す。 |
| `slave_offline` | Slave Heartbeat を受信できていない | 端末/ネットワークを確認し、Heartbeat を再送する。 |
| `slave_auto_trading_disabled` | MT4/MT5 の AlgoTrading が OFF | 「Algo Trading」ボタンを有効にする。 |
| `master_web_ui_disabled` | Master Intent が OFF | Master ノードを ON に戻す。 |
| `master_offline` | Master Heartbeat が失われた | Master EA を起動する。 |
| `master_auto_trading_disabled` | Master 側の自動売買が OFF | Master の Algo 設定を修正。 |
| `master_cluster_degraded` | マルチ Master の一部が未接続 | すべての Master を接続するまで Slave は Standby。 |
| `no_master_assigned` | Slave に紐付く Master が 0 件 | Web UI で TradeGroup に Slave を追加。 |

すべて `snake_case` 文字列でシリアライズされ、将来的にコードが増える場合も後方互換を維持する。
# API仕様 (Status Engine 版)

Status Engine リファクタリング後の REST / WebSocket API で公開されるステータス関連フィールドをまとめる。`enabled_flag` (ユーザー意図) と `runtime_status` (Status Engine の計算結果) を統一的に扱うためのリファレンスとして利用できる。

## ステータスフィールドの定義

| フィールド | 型 | 由来 | 説明 |
|------------|----|------|------|
| `enabled_flag` | boolean | Web UI / API | ユーザーがトグルで指定した意図。true のとき「コピーしたい」。Status Engine の入力値になる。 |
| `runtime_status` | number (0/1/2) | Status Engine | Slave 実効ステータス。0=DISABLED, 1=ENABLED (Slave準備完了だが Master 未接続), 2=CONNECTED。 |
| `master_runtime_status` | number (0/2) | Status Engine | Master の実効ステータス。Master は ENABLED を取らないため 0 (DISABLED) or 2 (CONNECTED)。 |
| `allow_new_orders` | boolean | Status Engine | Slave runtime_status が 2 の場合のみ true。EA へ設定を送る際に参照される。 |
| `warning_codes` | WarningCode[] | Status Engine | RuntimeStatusUpdater が付与する警告配列。`snake_case` 文字列 (`slave_offline` など) を返し、原因を UI/EA/CS で共有する。 |

> **重要:** `status` カラムは後方互換目的で DB に残っているが値は常に `runtime_status` と一致する。API / WebSocket では `runtime_status` を参照すること。

## オブジェクトスキーマ

### TradeGroup (Master)

```jsonc
{
  "master_account": "MASTER_001",
  "display_name": "My Master",
  "enabled": true,
  "enabled_flag": true,
  "master_runtime_status": 2,
  "warning_codes": [],
  "members": [ ... ]
}
```

| フィールド | 説明 |
|------------|------|
| `enabled` | 旧プロパティ。常に `enabled_flag` と同期。段階的に削除予定。 |
| `enabled_flag` | Master の意図。`POST /api/trade-groups/{master}/toggle` で更新。 |
| `master_runtime_status` | Status Engine の結果。Heartbeat/Unregister 経由で再計算される。 |

### TradeGroupMember (Slave)

```jsonc
{
  "trade_group_id": "MASTER_001",
  "slave_account": "SLAVE_TYO_01",
  "enabled_flag": true,
  "runtime_status": 1,
  "status": 1,
  "slave_settings": { ... }
}
```

| フィールド | 説明 |
|------------|------|
| `enabled_flag` | Slave の意図。`POST /api/trade-groups/{master}/members/{slave}/toggle` で更新。 |
| `runtime_status` | Status Engine の実効ステータス。`member_updated` WebSocket で配信。 |
| `status` | 互換用ミラー。クライアントは `runtime_status` を使う。 |
| `allow_new_orders` | Slave 設定 (`send_config_to_slave`) 内に含まれる。`runtime_status === 2` のときのみ true。 |
| `warning_codes` | Slave 用の警告配列。`slave_offline` や `master_cluster_degraded` など原因を示す。 |

## REST エンドポイント

### `GET /api/trade-groups`

- 返却値: `TradeGroup[]`
- `master_runtime_status` および `enabled_flag` を含む。
- `members` 配列の各要素には `runtime_status` / `enabled_flag` が付与されている。

### `GET /api/trade-groups/{master_account}`

- 単一マスター詳細。
- `master_runtime_status`、`members[].runtime_status` が最新値で返る。
- マルチ Master/Slave の「全 Master 接続判定」は Status Engine が計算済み。
- `warning_codes` が付与されている場合は UI で黄色バナーを表示し、CS も同じ配列を確認できる。

### `POST /api/trade-groups/{master_account}/toggle`

```json
{
  "enabled": true
}
```

- リクエスト body: `{ "enabled": boolean }`
- `enabled_flag` を即時更新し、その後 Status Engine が Heartbeat/接続状況から `master_runtime_status` を再計算。
- レスポンスは更新後の `TradeGroup` レコード。

### `POST /api/trade-groups/{master_account}/members/{slave_account}/toggle`

```json
{
  "enabled": false
}
```

- `enabled_flag` を切り替える唯一の手段。
- レスポンスは更新後の `TradeGroupMember`。`runtime_status` は WebSocket で配信される値を待つ（即時同期しない）。
- `warning_codes` は Heartbeat/RuntimeStatusUpdater の再評価後に WebSocket/Config 経由で更新される点に注意。

### `GET /api/trade-groups/{master_account}/members`

- 指定 Master に紐づく全 Slave を一覧取得。
- 監視 UI では `enabled_flag` を意図表示に、`runtime_status` を実行状態バッジに利用する。

## WebSocket イベント

`/ws` で配信される主要イベント:

| イベント | ペイロード | 説明 |
|----------|------------|------|
| `member_updated` | `TradeGroupMember` | Slave の `enabled_flag`/`runtime_status`/`warning_codes` 更新を通知。トグル操作後の状態反映に必須。 |
| `trade_group_updated` | `TradeGroup` | Master の `enabled_flag`/`master_runtime_status`/`warning_codes` 更新。複数 Slave に影響する。 |
| `settings_updated` | `SlaveConfigWithMaster` | `allow_new_orders` と `warning_codes` を含む config 再配信。Status Engine 結果を EA に同期。 |

## 状態遷移タイミング

1. ユーザーが Web UI でトグル操作 → REST API が `enabled_flag` を即時更新し 200 を返す。
2. 接続情報 (Heartbeat/Unregister) を受けた Status Engine が `runtime_status`/`master_runtime_status` を再計算。
3. DB 更新後、WebSocket で `member_updated` / `trade_group_updated` を配信。UI はこの通知で実効ステータスを更新する。
4. Config Builder が `allow_new_orders` を含む設定を再生成し、対象 EA へ配信する。

## Runtime Status Metrics API

- `GET /api/runtime-status-metrics` は `RuntimeStatusUpdater` のメトリクススナップショットを返す。
- レスポンス例:

```json
{
  "master_evaluations_total": 420,
  "master_evaluations_failed": 0,
  "slave_evaluations_total": 1380,
  "slave_evaluations_failed": 2,
  "slave_bundles_built": 512,
  "last_cluster_size": 2
}
```

- 監視用途: `slave_evaluations_failed` の急増で ZeroMQ/DB 輻輳を検知、`last_cluster_size` でマルチ Master の接続数を可視化できる。

## テストポリシー

- REST ハンドラの単体テスト (`relay-server/src/api/tests`) では `enabled_flag` と `runtime_status` が JSON に含まれることを検証する。
- Web UI の Playwright テスト (`web-ui/__tests__/runtime-intent-toggle.spec.ts`) は「トグル操作 → runtime_status は WS 通知後に変わる」流れを再現する。
- Python プロトコルテスト (`tests/test_zmq_communication.py`) は、`allow_new_orders` が `runtime_status == 2` のときのみ true になることを確認する予定。

必要に応じて本ドキュメントを拡張し、API 変更時は最初に更新する。