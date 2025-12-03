# Status Engine リファクタリング進捗管理

| タスクID | フェーズ | 内容 | 担当 | 期限 | 状態 | メモ |
|----------|----------|------|------|------|------|------|
| P1 | 準備 | 影響範囲調査・ログ取得 | TBD |  | ☐ 未着手 | 
| P2 | 準備 | `status_engine.rs` ひな形 + 単体テスト | TBD |  | ☑ 完了 (2025-12-02) | 基本API+テスト追加済み |
| P3 | Phase1 | heartbeat を新 API に置換 | TBD |  | ☑ 完了 (2025-12-02) | evaluate_* に統一 |
| P4 | Phase1 | API (`trade_group_members`, `trade_groups`) を新 API に置換 | TBD |  | ☑ 完了 (2025-12-06) | ZMQ config 経路が status_engine 出力を使用 |
| P5 | Phase1 | unregister / timeout ハンドラ適用 | TBD |  | ☑ 完了 (2025-12-06) | master disconnect/timeout で status_engine ベース通知 |
| P4a | Phase1 | config_request を新 API に置換 | TBD |  | ☑ 完了 (2025-12-06) | Master/Slave CONFIG 応答が evaluate_* を使用 |
| P6 | Phase2 | DB へ `enabled_flag` / `runtime_status` 追加 (マイグレーション) | TBD |  | ☑ 完了 (2025-12-07) | カラム追加・バックフィル・CRUD更新まで実装済み |
| P7 | Phase2 | API/UI を `enabled_flag` ベースに更新 | TBD |  | ☑ 完了 (2025-12-03) | API + WebSocket + Web UI (badges/i18n) 対応完了 |
| P8 | Phase3 | Config Builder 実装・全経路切替 | TBD |  | ☑ 完了 (2025-12-03) | Builder + docs/tests (allow_new_orders) 全経路切替完了 |
| P9 | Phase4 | MT アドバイザ更新 (MT4/MT5) | TBD | 2025-12-03 | ☑ 完了 | MT EA 側で `allow_new_orders/runtime_status` を起点に制御するよう改修済み |
| P10 | Phase4 | Web UI 更新 | TBD |  | ☑ 完了 (2025-12-03) | Intlayer/バッジ/トグルが新 2 層ステータスに追従済み |
| P11 | Phase5 | Docs / Test 更新 | TBD |  | ☑ 完了 (2025-12-03) | API仕様ドキュメント追加 + Status Engine 追加テスト |
| P12 | Phase6 | 旧ロジック削除・監視整備 | TBD |  | ☑ 完了 (2025-12-03) | legacy status.rs 削除 + runtime telemetry ログ出力 |
| P13 | Phase7 | Heartbeat/Timeout で runtime_status を常時再計算 | TBD | 2025-12-10 | ☑ 完了 (2025-12-03) | RuntimeStatusUpdater で Master/Slave Heartbeat, timeout/unregister すべてが即DB反映 + ZMQ 通知 |
| P14 | Phase7 | MasterClusterSnapshot 共通化 | TBD | 2025-12-10 | ☑ 完了 (2025-12-03) | 新 `runtime_status_updater` モジュールで multi-master cluster を共通化 |
| P15 | Phase7 | Intent Toggle 直後の runtime 反映 | TBD | 2025-12-11 | ☑ 完了 (2025-12-03) | Web API (toggle/update) が Status Engine 再評価→即ZMQ送信するよう統一 |
| P16 | Phase7 | warning_codes 詳細化 | TBD | 2025-12-12 | ☑ 完了 (2025-12-03) | WarningCode enum + Config/ZMQ/API へ伝搬・テスト拡充 |
| P17 | Phase7 | RuntimeStatusUpdater サービス & 監視 | TBD | 2025-12-12 | ☑ 完了 (2025-12-03) | RuntimeStatusUpdater を単一サービス化し、tracing/metrics を追加。Heartbeat/ConfigRequest/Unregister/API から共通メトリクスを共有 |
| P18 | Phase7 | Docs/E2E リフレッシュ | TBD | 2025-12-13 | ☐ 未着手 | 新挙動を `docs/runtime-status-alignment.md` と E2E テストに反映 |

## 決定事項・メモ
- 後方互換性は考慮しない。既存カラム/コードは新設計に沿わなければ削除。
- 進捗は週次で更新し、完了タスクは ☑ に変更。

## P17 設計メモ (2025-12-03)
- RuntimeStatusUpdater を単一サービスとして扱うため、`runtime_status_updater.rs` に以下を追加予定:
	- `RuntimeStatusMetrics` 構造体（Arc共有）で `evaluations_total`, `evaluations_failed`, `cluster_size_histogram` を記録。
	- `tracing::instrument` を `evaluate_master_runtime_status` / `evaluate_slave_runtime_status` / `build_slave_bundle` に付与し、master/slave account, intent フラグ, allow_new_orders 等をイベント化。
- UT: `runtime_status_updater.rs` にメトリクス増分・スナップショット検証を追加、API では `runtime_metrics_tests` で `/api/runtime-status-metrics` の応答内容を検証。
- 監視出力の集約先:
	- Heartbeat / ConfigRequest / Unregister ハンドラ → `RuntimeStatusMonitor::record_slave_eval(&result, source)` を呼び出し、source (`heartbeat`, `config_request`, `api_toggle`) タグでメトリクス化。
	- API (`trade_group_members` list/toggle/update) では Web UI 返却 JSON に `warning_codes` を載せつつ、必要なトレースIDを `tracing::Span::current()` に記録。
- 収集バックエンド未定のため、当面は `metrics` crate の `describe_*` + Prometheus exporter（既存 `monitoring` submodule）へ統合する想定。導入後に `docs/runtime-status-alignment.md` へダッシュボード手順を追記予定。
