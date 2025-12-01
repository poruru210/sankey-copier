# ログ改善の変化点・影響分析

## 概要

本ドキュメントでは、ダッシュボードの全機能を有効にするために必要なログ構造改善について、変更対象ファイル、影響範囲、後方互換性を分析します。

---

## 変更対象ファイル

### 1. `relay-server/src/victoria_logs.rs`

**変更内容**: LogEntry構造体の拡張

```rust
// 現在の構造
pub struct LogEntry {
    pub time: String,      // _time
    pub msg: String,       // _msg
    pub level: String,
    pub source: String,
    pub target: String,
    pub file: Option<String>,
    pub line: Option<u32>,
}

// 改善後の構造
pub struct LogEntry {
    pub time: String,
    pub msg: String,
    pub level: String,
    pub source: String,
    pub target: String,
    pub file: Option<String>,
    pub line: Option<u32>,
    // 新規フィールド
    pub event_type: Option<String>,      // "trade_signal_received" etc.
    pub ticket: Option<i64>,
    pub symbol: Option<String>,
    pub action: Option<String>,
    pub lots: Option<f64>,
    pub master_account: Option<String>,
    pub slave_account: Option<String>,
    pub duration_ms: Option<u64>,
    pub error_type: Option<String>,
}
```

**MessageVisitor の変更**:
- 構造化フィールド (ticket, symbol, action, lots, etc.) を個別フィールドとして抽出
- tracing マクロの named parameters を LogEntry の対応フィールドにマッピング

**影響**:
- VictoriaLogs への JSON Lines フォーマット変更
- 既存ログはそのまま動作（新フィールドは Optional）
- ダッシュボードで新しいフィルタリング・集計が可能に

---

### 2. `relay-server/src/log_buffer.rs`

**現状**: 既に `fields: HashMap<String, JsonValue>` をサポート

**変更内容**: 変更不要（tracing の named parameters は既に fields に格納される）

**理由**: LogBufferLayer の FieldVisitor は既に以下をサポート:
- `record_i64`, `record_u64`, `record_f64` - 数値フィールド
- `record_str` - 文字列フィールド
- `record_bool` - ブールフィールド
- `record_debug` - その他のフィールド

---

### 3. `relay-server/src/message_handler/trade_signal.rs`

**変更内容**: tracing マクロの呼び出しを構造化フィールド付きに変更

#### 変更箇所 1: シグナル受信 (L12)

```rust
// Before
tracing::info!("Processing trade signal: {:?}", signal);

// After
tracing::info!(
    event_type = "trade_signal_received",
    ticket = signal.ticket,
    symbol = signal.symbol.as_deref().unwrap_or(""),
    action = ?signal.action,
    lots = signal.lots.unwrap_or(0.0),
    master_account = %signal.source_account,
    "Processing trade signal"
);
```

#### 変更箇所 2: TradeGroup 未検出 (L26-29)

```rust
// Before
tracing::warn!(
    "TradeGroup not found for master {}, using defaults",
    signal.source_account
);

// After
tracing::warn!(
    event_type = "trade_group_not_found",
    master_account = %signal.source_account,
    "TradeGroup not found, using defaults"
);
```

#### 変更箇所 3: TradeGroup 取得エラー (L33-36)

```rust
// Before
tracing::error!(
    "Failed to get TradeGroup for master {}: {}",
    signal.source_account,
    e
);

// After
tracing::error!(
    event_type = "trade_group_error",
    master_account = %signal.source_account,
    error_type = "db_error",
    error = %e,
    "Failed to get TradeGroup"
);
```

#### 変更箇所 4: メンバー取得エラー (L45-49)

```rust
// Before
tracing::error!(
    "Failed to get members for master {}: {}",
    signal.source_account,
    e
);

// After
tracing::error!(
    event_type = "members_error",
    master_account = %signal.source_account,
    error_type = "db_error",
    error = %e,
    "Failed to get members"
);
```

#### 変更箇所 5: トレードフィルタリング (L58-61)

```rust
// Before
tracing::debug!(
    "Trade filtered out for slave account: {}",
    member.slave_account
);

// After
tracing::debug!(
    event_type = "trade_copy_filtered",
    ticket = signal.ticket,
    master_account = %signal.source_account,
    slave_account = %member.slave_account,
    symbol = signal.symbol.as_deref().unwrap_or(""),
    "Trade filtered out"
);
```

#### 変更箇所 6: コピー実行 (L92-97)

```rust
// Before
tracing::info!(
    "Copying trade to {}: {} {} lots",
    member.slave_account,
    transformed.symbol.as_deref().unwrap_or("?"),
    transformed.lots.unwrap_or(0.0)
);

// After
tracing::info!(
    event_type = "trade_copy_started",
    ticket = signal.ticket,
    master_account = %signal.source_account,
    slave_account = %member.slave_account,
    symbol = transformed.symbol.as_deref().unwrap_or(""),
    lots = transformed.lots.unwrap_or(0.0),
    action = ?signal.action,
    "Copying trade"
);
```

#### 変更箇所 7: 送信成功 (L109-113)

```rust
// Before
tracing::debug!(
    "Sent signal to trade group '{}' for slave '{}'",
    member.trade_group_id,
    member.slave_account
);

// After
tracing::debug!(
    event_type = "trade_copy_completed",
    ticket = signal.ticket,
    slave_account = %member.slave_account,
    trade_group_id = %member.trade_group_id,
    "Signal sent successfully"
);
```

#### 変更箇所 8: 送信エラー (L107)

```rust
// Before
tracing::error!("Failed to send signal to trade group: {}", e);

// After
tracing::error!(
    event_type = "trade_copy_failed",
    ticket = signal.ticket,
    slave_account = %member.slave_account,
    error_type = "send_error",
    error = %e,
    "Failed to send signal"
);
```

#### 変更箇所 9: 変換エラー (L126)

```rust
// Before
tracing::error!("Failed to transform signal: {}", e);

// After
tracing::error!(
    event_type = "trade_copy_failed",
    ticket = signal.ticket,
    master_account = %signal.source_account,
    slave_account = %member.slave_account,
    error_type = "transform_error",
    error = %e,
    "Failed to transform signal"
);
```

---

### 4. レイテンシ計測の追加

**変更箇所**: `process_trade_copy` 関数

```rust
// relay-server/src/message_handler/trade_signal.rs

use std::time::Instant;

async fn process_trade_copy(
    &self,
    signal: &TradeSignal,
    member: &TradeGroupMember,
    master_settings: &MasterSettings,
) {
    let start = Instant::now();

    // ... existing logic ...

    // 成功時のログにduration_msを追加
    let duration_ms = start.elapsed().as_millis() as u64;
    tracing::info!(
        event_type = "trade_copy_completed",
        ticket = signal.ticket,
        slave_account = %member.slave_account,
        duration_ms = duration_ms,
        "Trade copy completed"
    );
}
```

---

## VictoriaLogs JSON出力例

### Before (現在)

```json
{"_time":"2025-01-15T10:30:45.123Z","_msg":"Processing trade signal: TradeSignal { action: Open, ticket: 12345, symbol: Some(\"EURUSD\"), ... }","level":"INFO","source":"relay-server","target":"relay_server::message_handler::trade_signal"}
```

### After (改善後)

```json
{"_time":"2025-01-15T10:30:45.123Z","_msg":"Processing trade signal","level":"INFO","source":"relay-server","target":"relay_server::message_handler::trade_signal","event_type":"trade_signal_received","ticket":12345,"symbol":"EURUSD","action":"Open","lots":1.0,"master_account":"MASTER_001"}
```

---

## 影響範囲

### 1. VictoriaLogs クエリ

| 機能 | Before | After |
|------|--------|-------|
| シンボル別フィルタ | `_msg:~"EURUSD"` (不正確) | `symbol:"EURUSD"` (正確) |
| チケット追跡 | 不可能 | `ticket:12345` |
| マスター別集計 | `_msg:~"master"` (不正確) | `master_account:"MASTER_001"` |
| レイテンシ集計 | 不可能 | `duration_ms:>100` |
| イベントタイプ別 | メッセージ全文検索 | `event_type:"trade_copy_completed"` |

### 2. ダッシュボード機能

| パネル | Before | After |
|--------|--------|-------|
| Latency Metrics | "Not Available" 表示 | P50/P95/P99 表示可能 |
| Symbol Performance | 不可 | シンボル別レイテンシ表示 |
| Ticket Tracking | 不可 | End-to-end トレース可能 |
| Success Rate | メッセージ検索ベース | イベントタイプベース（正確） |

### 3. パフォーマンス

| 項目 | 影響 |
|------|------|
| ログサイズ | +20-50% （追加フィールド分） |
| CPU | 微増 （フィールド抽出処理） |
| メモリ | 変更なし（バッファサイズ固定） |

---

## 後方互換性

### ✅ 互換性あり

1. **既存ログ**: 新フィールドは全て `Option<T>` のため、既存ログとの互換性維持
2. **VictoriaLogs**: 既存クエリは引き続き動作（`_msg:` 検索は継続可能）
3. **ダッシュボード**: 既存パネルは影響なし、新機能は段階的に有効化

### ⚠️ 注意事項

1. **混在期間**: デプロイ中は新旧フォーマットが混在
2. **クエリ変更**: 新フィールドを使用するクエリは新ログのみマッチ
3. **ディスク使用量**: ログサイズ増加に伴い、retention 期間の見直しが必要な場合あり

---

## 実装手順

### Step 1: LogEntry 構造体の拡張
```
対象: relay-server/src/victoria_logs.rs
作業: 新フィールドの追加（Option<T>で追加、既存との互換性維持）
テスト: 単体テストでシリアライズ確認
```

### Step 2: MessageVisitor の拡張
```
対象: relay-server/src/victoria_logs.rs
作業: 特定フィールド名を検出して構造化フィールドに格納
テスト: 単体テストでフィールド抽出確認
```

### Step 3: トレードシグナルハンドラの修正
```
対象: relay-server/src/message_handler/trade_signal.rs
作業: 各tracing呼び出しに構造化フィールドを追加
テスト: E2Eテストでログ出力確認
```

### Step 4: レイテンシ計測の追加
```
対象: relay-server/src/message_handler/trade_signal.rs
作業: process_trade_copy関数に時間計測を追加
テスト: E2Eテストでduration_msフィールド確認
```

### Step 5: ダッシュボードの更新
```
対象: grafana/dashboards/*.json
作業: Performance dashboardのLatencyセクションを有効化
テスト: Grafanaで表示確認
```

---

## テスト計画

### 単体テスト

```rust
#[test]
fn test_log_entry_with_structured_fields() {
    let entry = LogEntry {
        time: "2025-01-15T10:30:45.123Z".to_string(),
        msg: "Processing trade signal".to_string(),
        level: "INFO".to_string(),
        source: "relay-server".to_string(),
        target: "relay_server::message_handler".to_string(),
        file: None,
        line: None,
        event_type: Some("trade_signal_received".to_string()),
        ticket: Some(12345),
        symbol: Some("EURUSD".to_string()),
        action: Some("Open".to_string()),
        lots: Some(1.0),
        master_account: Some("MASTER_001".to_string()),
        slave_account: None,
        duration_ms: None,
        error_type: None,
    };

    let json = serde_json::to_string(&entry).unwrap();
    assert!(json.contains("\"ticket\":12345"));
    assert!(json.contains("\"symbol\":\"EURUSD\""));
    assert!(json.contains("\"event_type\":\"trade_signal_received\""));
}
```

### E2E テスト

```rust
#[tokio::test]
async fn test_structured_logging_e2e() {
    // 1. テスト用VictoriaLogsモックサーバー起動
    // 2. トレードシグナル送信
    // 3. VictoriaLogsに送信されたJSONを検証
    // 4. 構造化フィールドの存在確認
}
```

---

## ロールバック計画

問題発生時のロールバック手順:

1. **即時対応**: 新フィールドは Optional のため、旧バージョンへのロールバック可能
2. **データ**: VictoriaLogs内の既存データに影響なし
3. **ダッシュボード**: 旧クエリ（`_msg:` ベース）に戻すだけで復旧可能

---

## 完了条件

- [ ] LogEntry 構造体の拡張完了
- [ ] MessageVisitor のフィールド抽出実装完了
- [ ] trade_signal.rs の全 tracing 呼び出し更新完了
- [ ] レイテンシ計測（duration_ms）実装完了
- [ ] 単体テスト追加・パス
- [ ] E2E テスト追加・パス
- [ ] Performance ダッシュボードの Latency セクション有効化
- [ ] 本番環境デプロイ・動作確認
