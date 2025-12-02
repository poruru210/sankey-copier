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
| P8 | Phase3 | Config Builder 実装・全経路切替 | TBD |  | ☐ 未着手 | 
| P9 | Phase4 | MT アドバイザ更新 (MT4/MT5) | TBD |  | ☐ 未着手 | 
| P10 | Phase4 | Web UI 更新 | TBD |  | ☐ 未着手 | 
| P11 | Phase5 | Docs / Test 更新 | TBD |  | ☐ 未着手 | 
| P12 | Phase6 | 旧ロジック削除・監視整備 | TBD |  | ☐ 未着手 | 

## 決定事項・メモ
- 後方互換性は考慮しない。既存カラム/コードは新設計に沿わなければ削除。
- 進捗は週次で更新し、完了タスクは ☑ に変更。
