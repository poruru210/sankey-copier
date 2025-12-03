# Runtime Status 整合設計ノート

Web UI、EA パネル、relay-server の 3 者で Runtime Status が同じ意味を持つように、情報源・更新フロー・既知の乖離・対応策をゼロベースでまとめ直した。議論や改修の前提資料として利用する。

> **2025-12-04 更新**: `RuntimeStatusUpdater` サービスに Heartbeat/Timeout/Intent API/RequestConfig/Unregister を集約し、あらゆる経路で `trade_group_members.runtime_status` が即時更新されるようになった。本ドキュメントは新しい警告コードとメトリクス仕様を含めて再整理している。

---

## 1. 目的と非目的

- ユーザーが見るステータス（バッジ、Nord バー、EA Enabled/Disabled）が指し示す状態を明確にする。
- `runtime_status` 値と、その値を更新するバックエンドイベントを可視化し、仕様変更の影響範囲を把握する。
- 食い違いが発生するシナリオと、解決オプションを列挙する。

以下は対象外：MT 端末側 UI の変更、ZeroMQ メッセージ形式の改訂、Status 以外のメトリクス（スプレッド等）の扱い。

---

## 2. 表示に使われる信号一覧

| 信号 | 生成元 | Web UI での利用箇所 | 更新トリガー | 備考 |
| --- | --- | --- | --- | --- |
| EA Enabled/Disabled | MT4/MT5 ローカル設定 (AlgoTrading ボタン、EA Inputs) | EA パネルのみ | 端末操作即時 | Relay Server を通らない。 |
| `runtime_status` (0/1/2) | Relay Server `RuntimeStatusUpdater` が `trade_group_members.runtime_status` を単一ソースとして管理 | ノードバッジ・Nord ステータスバー | Master/Slave Heartbeat, Timeout, Intent API, RequestConfig, Unregister のたびに再計算 | Web UI は EventStream/ポーリングで DB 値を受け、そのまま描画。 |
| `hasWarning` | Relay で集計した `warning_codes` | Nord ステータスバー色 (黄)・ツールチップ | Algo OFF など警告検出時 | `runtime_status` に優先してバー色を黄色へ上書き。 |

---

## 3. runtime_status 定義と UI 反映

| 値 | 名称 | サーバー判定条件 (簡略) | Web UI 表示 | Nord バー色 | 代表シナリオ |
| --- | --- | --- | --- | --- | --- |
| 2 | Connected | Master/Slave: Intent ON ∧ 接続 Online ∧ `is_trade_allowed` true。Slave は全 Master Connected が必須。 | Master: `配信中`、Slave: `受信中` (エメラルド) | `bg-green-500` (警告なしの場合) | 両 EA が稼働し、コピー配信中。 |
| 1 | Standby | Slave: 自身は送受信可能だが関連 Master が未接続。Master 側では事実上未使用。 | `待機中` (琥珀) | `bg-amber-500` | Slave が接続済みだが Master を待っている。 |
| 0 | ManualOff | Intent OFF / 接続 Offline / `is_trade_allowed` false 等で evaluate が失敗。 | `手動OFF` (グレー) | `bg-gray-300` | ユーザーが停止、Algo OFF、未接続など。 |
| Warning override | — | `hasWarning=true` | バッジは runtime_status 表示 + 警告ツールチップ | `bg-yellow-500` へ強制 | Algo Trading OFF や証拠金警告。 |

> 補足: `StatusIndicatorBar` は `hasWarning` を最優先し、次に `runtime_status`→`isActive` の順で色を決定する。ノードヘッダーのバッジは `runtime_status` と `isActive` を基に Intlayer の文言を選択する。

---

## 4. runtime_status 更新フロー (2025-12-04)

すべてのイベントは `RuntimeStatusUpdater` を経由し、`trade_group_members.runtime_status` および `warning_codes`（ZMQ/REST レスポンスのみ）が一元的に決定される。Builder/Config 送信も同じスナップショットを共有するため、EA/UI/DB が同じ値を参照する。

### 4.1 Master

1. Heartbeat/Timeout/Unregister/API Toggle の各経路で `RuntimeStatusUpdater::evaluate_master_runtime_status` を呼び、Intent/Online/TradeAllowed を判定して `STATUS_CONNECTED (2)` か `STATUS_DISABLED (0)` を返す。
2. 結果は `ConfigBuilder::build_master_config` と VictoriaLogs ブロードキャストで共有され、`warning_codes` と `allow_new_orders` も同じ値になる。
3. `RuntimeStatusUpdater` が `trade_group_members` へ Master 集約結果を反映し、全 Slave のクラスター評価に再利用される。

### 4.2 Slave

1. `RuntimeStatusUpdater::evaluate_slave_runtime_status` が (Intent, 接続, TradeAllowed, MasterClusterSnapshot) を評価し、0/1/2 と `warning_codes` を返す。
2. 呼び出しトリガーは Slave/Master Heartbeat, Timeout, RequestConfig, Intent Toggle, Unregister の全イベントで、`message_handler` と `trade_group_members` API が共通のヘルパーを使用する。
3. `send_config_to_slave` は `RuntimeStatusUpdater::build_slave_bundle` を通じて Config + DB 更新 + メトリクス記録をまとめて実行するため、ZMQ に載る `status` と DB の `runtime_status` が常に一致する。
4. Slave Heartbeat も `RuntimeStatusUpdater` を通すようになったため、Algo ON へ戻した瞬間に Standby(1) または Connected(2) が DB に反映される。Master 不在時でも `RequestConfig`/Heartbeat どちらでも同じ結果を得られる。

---

## 5. 既知の乖離シナリオ

| シナリオ | EA パネル | `runtime_status` | Web UI 表示 | 原因 | 対処方針 |
| --- | --- | --- | --- | --- | --- |
| Algo OFF → ON を素早く切り替え | Enabled に戻る | 1 (Standby) に遷移するが `warning_codes` が 1 ティック残る | `待機中` + 黄色バー (警告優先) | Heartbeat→RuntimeStatusUpdater→Config の間に 1 ティック遅延がある | 監視ログで `warning_codes=[]` を確認後に UI 更新。今後は UI 側で `warning_codes` 解除イベントを待つ。 |
| Intent ON だが `is_trade_allowed=false` | Enabled | 0 | `手動OFF` | Status Engine が TradeAllowed を優先し 0 を返す仕様。 | ドキュメント/サポートで「Algo を許可 or AutoTrading ON が必要」と明記済み。 |
| Multi-Master で 1 台だけ Offline | Enabled | 1 (Standby) | `待機中` | MasterClusterSnapshot が完全接続になるまで 2 に上がらない。 | RuntimeStatus の仕様通り。UI に「Master 復帰待ち」ツールチップを表示。 |

---

## 6. 対応オプション

| # | アプローチ | 内容 | ステータス | 補足 |
| --- | --- | --- | --- | --- |
| A | サーバー主義 (RuntimeStatusUpdater) | Heartbeat/Timeout/API から必ず Status Engine を通し、DB/ZMQ/UI を単一ソース化。 | ✅ 本番反映済み (2025-12-04) | `runtime_status_updater.rs` + Config Builder + DB 更新の三位一体で運用。 |
| B | クライアント補正 | Web UI が意図的に `runtime_status` を書き換えて UX を補正。 | ❌ 廃止 | 新仕様ではデータ不整合になるため削除済み。 |
| C | モニタリング強化 | `RuntimeStatusMetrics` + VictoriaLogs で評価回数/失敗を可視化。 | 🚧 ダッシュボード整備中 | `GET /api/runtime-status-metrics` を Grafana へ取り込み予定。 |

---

## 7. 推奨アクション

1. **RuntimeStatusUpdater を前提にした説明資料へ差し替え**: `docs/architecture.md` / `docs/api-specification.md` / リリースノートで Standby/警告表示の新ルールと API レスポンス例を共有する。
2. **警告コードの運用メモ**: `warning_codes` が Master/Slave で別 enum になったため、CS/QA で参照できる一覧を `docs/runtime-status-alignment.md` と `docs/troubleshooting/*` に追記する。
3. **テレメトリ監視**: `/api/runtime-status-metrics` を Grafana へ流し、`slave_evaluations_failed` が増えた際に VictoriaLogs/DB/ZeroMQ を切り分けられるようにする。
4. **E2E カバレッジ**: Heartbeat → Standby → Connected までの遷移を Rust E2E テストで固定化し、回帰をブロックする。

---

## 8. 参考: Nord カードの視覚ルール

| 判定要素 | ロジック | 表示結果 |
| --- | --- | --- |
| `hasWarning=true` | どの `runtime_status` でも先に評価。 | Nord バー=黄 (`bg-yellow-500`)、バッジは通常表示 + 警告ツールチップ。 |
| `runtime_status=2` | `account.isActive` が true のときのみ緑。 | バッジ: `配信中/受信中 (bg-emerald-500)`、バー: `bg-green-500`。 |
| `runtime_status=1` | Master 待ち。 | バッジ: `待機中 (bg-amber-100)`、バー: `bg-amber-500`。 |
| `runtime_status=0` | 停止。 | バッジ: `手動OFF (bg-gray-200)`、バー: `bg-gray-300`。 |

これらは `web-ui/components/nodes/AccountNodeHeader.tsx` と `StatusIndicatorBar.tsx` に定義されており、カラークラスは Tailwind (`tailwind.config.ts`) のデフォルトスケールを使用している。

---

## 9. Runtime Status 監視エンドポイント (2025-12-04 更新)

- Relay Server の `GET /api/runtime-status-metrics` は `RuntimeStatusUpdater::RuntimeStatusMetricsSnapshot` を返し、Heartbeat/Timeout/API すべての評価回数を一目で確認できる。
- レスポンス項目: `master_evaluations_total/failed`, `slave_evaluations_total/failed`, `slave_bundles_built`, `last_cluster_size`。`last_cluster_size` は multi-master 環境の規模把握にも利用できる。
- 監視活用例:
  - `slave_evaluations_failed` が閾値を超えたら VictoriaLogs で同時刻のエラーを検索し、ZeroMQ 側の疎通を調査。
  - `last_cluster_size` を Web UI と突き合わせ、UI 側が Master 数を誤って表示していないか確認。
- Prometheus exporter/CloudWatch へ転送する場合も、このスナップショット構造体をそのまま scrape すれば良い。

## 10. Warning Codes の整理

- Master/Slave で別々の enum (`WarningCode::Master*`, `WarningCode::Slave*`) を導入し、未接続 Master 名や `is_trade_allowed=false` など原因をペイロード化した。
- Web UI は `warning_codes` が空でない場合に Nord バーを強制的に黄色化し、ツールチップに `code` と `detail` を翻訳して表示する。
- ZMQ Config / REST API / WebSocket が同じ配列を返すため、CS はログ ID だけで現象をトレースできる。