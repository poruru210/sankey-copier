# Master EA設定配信 + DB再構成リファクタリング実装計画

**作成日**: 2025-11-24
**ステータス**: 実装中
**担当**: Development Team

---

## 目次
1. [プロジェクト概要](#プロジェクト概要)
2. [背景と目的](#背景と目的)
3. [設計変更の詳細](#設計変更の詳細)
4. [実装フェーズ](#実装フェーズ)
5. [テスト戦略](#テスト戦略)
6. [リスクと対策](#リスクと対策)

---

## プロジェクト概要

### 目標
- Master EAへの設定配信機能実装（SymbolPrefix/Suffix）
- TradeGroup中心のDB構造への再構成
- クリーンコード原則に基づく大規模リファクタリング

### スコープ
- ✅ DB再構成（trade_groups中心）
- ✅ Master EA設定配信機能
- ✅ Relay Serverリファクタリング
- ✅ Web UI更新
- ✅ 冗長フィールド削除（trade_group_id, MagicFilter）
- ❌ 既存データ移行（不要）
- 🔜 Symbol Mappingバグ修正（Phase 2で実施）

### 原則
- TDD（Test-Driven Development）
- Small, safe, reversible changes
- Documentation-first

---

## 背景と目的

### 現状の問題点

1. **Master EAに設定配信機能が存在しない**
   - SymbolPrefix/SuffixがInput parameterのみ
   - Web UIからの動的設定変更が不可能

2. **DB構造の問題**
   - `connections`テーブルにすべての設定が混在
   - Master設定とSlave設定が分離されていない
   - `trade_group_id`が冗長（常に`master_account`と同一）

3. **Symbol Mappingバグ**
   - Web UIで設定されたSymbol MappingがSlave EAに届かない
   - DLL配列取得APIが未実装（TODO line 226-232）

### 新設計の方針

**TradeGroupの再定義**:
- 現在: TradeGroup = Master（冗長な別名）
- 新設計: TradeGroup = 1 Master + N Slaves のグループ
- 1つのMasterは1つの設定を持ち、すべてのSlaveに共通適用

**設定の分離**:
- Master設定: TradeGroupレベル（symbol_prefix, symbol_suffix）
- Slave設定: Memberレベル（lot_multiplier, reverse_trade, symbol_mappings, etc.）

---

## 設計変更の詳細

### 現在のDB構造

```sql
connections (
  id INTEGER PRIMARY KEY,
  master_account TEXT,
  slave_account TEXT,
  status INTEGER,
  settings TEXT (JSON),  -- すべての設定が混在
  created_at DATETIME,
  updated_at DATETIME,
  UNIQUE(master_account, slave_account)
)
```

**問題点**:
- Master設定がSlave接続ごとに重複
- 設定の論理的分離ができていない

### 新しいDB構造

```sql
-- TradeGroup: Master中心のグループ
trade_groups (
  id TEXT PRIMARY KEY,              -- master_account
  master_settings TEXT (JSON),      -- Master固有の設定
  created_at DATETIME,
  updated_at DATETIME
)

-- TradeGroupMember: Slave接続
trade_group_members (
  trade_group_id TEXT,              -- FK → trade_groups.id
  slave_account TEXT,
  slave_settings TEXT (JSON),       -- Slave固有の設定
  status INTEGER,                   -- 0=DISABLED, 1=ENABLED, 2=CONNECTED
  created_at DATETIME,
  updated_at DATETIME,
  PRIMARY KEY (trade_group_id, slave_account),
  FOREIGN KEY (trade_group_id) REFERENCES trade_groups(id) ON DELETE CASCADE
)
```

### JSON構造

**master_settings**:
```json
{
  "symbol_prefix": "pro.",
  "symbol_suffix": ".m",
  "config_version": 1
}
```

**slave_settings**:
```json
{
  "lot_multiplier": 1.0,
  "reverse_trade": false,
  "symbol_mappings": [
    {"source_symbol": "EURUSD", "target_symbol": "EURUSDm"}
  ],
  "filters": {
    "allowed_symbols": [],
    "blocked_symbols": []
  },
  "config_version": 1
}
```

### データフロー図

#### 現在のフロー（Slaveのみ）

```
┌─────────┐  Heartbeat    ┌──────────────┐  query       ┌──────────┐
│ Slave   │─────(PUSH)────>│ Relay Server │─connections─>│ Database │
│ EA      │               │              │              └──────────┘
└─────────┘               └──────────────┘
     │                           │
     │ RequestConfig (PUSH)      │
     │─────────────────────────> │
     │                           │
     │ ConfigMessage (SUB)       │ send_config()
     │ <──────────────────────── │ (PUB socket, port 5557)
```

#### 新しいフロー（Master + Slave）

```
┌─────────┐  Heartbeat    ┌──────────────┐  query       ┌──────────────┐
│ Master  │─────(PUSH)────>│ Relay Server │─trade_groups─>│ Database     │
│ EA      │               │              │              │              │
└─────────┘               │              │  query       │ - trade_groups
     │                    │              │─members────> │ - members    │
     │ RequestConfig      │              │              └──────────────┘
     │─────────────────> │              │
     │                    │              │
     │ MasterConfig (SUB) │ send_config()│
     │ <──────────────────│ (PUB 5557)   │
     │                    │              │
     │                    │              │
┌─────────┐  Heartbeat    │              │
│ Slave   │─────(PUSH)────>│              │
│ EA      │               │              │
└─────────┘               │              │
     │                    │              │
     │ RequestConfig      │              │
     │─────────────────> │              │
     │                    │              │
     │ SlaveConfig (SUB)  │ send_config()│
     │ <──────────────────│ (PUB 5557)   │
```

---

## 実装フェーズ

### Phase 1: DB再構成 + マイグレーション

#### 1.1 新スキーマ設計・SQL定義

**タスク**:
- [x] 新スキーマの詳細設計
- [ ] SQL migration fileの作成
- [ ] Schema validation

**成果物**:
- `relay-server/migrations/YYYYMMDDHHMMSS_refactor_to_trade_groups.sql`

**SQL内容**:
```sql
-- Drop old table
DROP TABLE IF EXISTS connections;

-- Create trade_groups table
CREATE TABLE trade_groups (
  id TEXT PRIMARY KEY,
  master_settings TEXT NOT NULL DEFAULT '{}',
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Create trade_group_members table
CREATE TABLE trade_group_members (
  trade_group_id TEXT NOT NULL,
  slave_account TEXT NOT NULL,
  slave_settings TEXT NOT NULL DEFAULT '{}',
  status INTEGER NOT NULL DEFAULT 0,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (trade_group_id, slave_account),
  FOREIGN KEY (trade_group_id) REFERENCES trade_groups(id) ON DELETE CASCADE
);

-- Indexes
CREATE INDEX idx_trade_group_members_slave ON trade_group_members(slave_account);
CREATE INDEX idx_trade_group_members_status ON trade_group_members(status);
```

#### 1.2 TradeGroup/Member構造体とテスト実装

**タスク**:
- [ ] `TradeGroup` struct定義
- [ ] `TradeGroupMember` struct定義
- [ ] `MasterSettings` struct定義（JSON deserialize用）
- [ ] `SlaveSettings` struct定義（JSON deserialize用）
- [ ] DB CRUD操作のテスト作成
- [ ] DB CRUD操作の実装

**ファイル**:
- `relay-server/src/models/trade_group.rs` (新規)
- `relay-server/src/models/trade_group_member.rs` (新規)
- `relay-server/tests/db/trade_group_test.rs` (新規)

**テストケース**:
1. TradeGroup作成・取得
2. Member追加・削除
3. Master設定更新
4. Slave設定更新
5. TradeGroup削除時のカスケード

#### 1.3 マイグレーション実行

**タスク**:
- [ ] マイグレーションツール確認
- [ ] ローカル環境でマイグレーション実行
- [ ] マイグレーション後のDB状態検証

---

### Phase 2: Relay Server リファクタリング

#### 2.1 DB層メソッド（テスト + 実装）

**新規メソッド**:

```rust
// relay-server/src/db/mod.rs

// TradeGroup操作
pub async fn create_trade_group(master_account: &str) -> Result<TradeGroup>;
pub async fn get_trade_group(master_account: &str) -> Result<TradeGroup>;
pub async fn update_master_settings(master_account: &str, settings: MasterSettings) -> Result<()>;
pub async fn delete_trade_group(master_account: &str) -> Result<()>;

// Member操作
pub async fn add_member(trade_group_id: &str, slave_account: &str, settings: SlaveSettings) -> Result<()>;
pub async fn get_members(trade_group_id: &str) -> Result<Vec<TradeGroupMember>>;
pub async fn get_member(trade_group_id: &str, slave_account: &str) -> Result<TradeGroupMember>;
pub async fn update_member_settings(trade_group_id: &str, slave_account: &str, settings: SlaveSettings) -> Result<()>;
pub async fn delete_member(trade_group_id: &str, slave_account: &str) -> Result<()>;

// 設定取得
pub async fn get_settings_for_master(master_account: &str) -> Result<MasterSettings>;
pub async fn get_settings_for_slave(slave_account: &str) -> Result<Vec<SlaveSettingsWithGroup>>;
```

**テストファイル**:
- `relay-server/tests/db/trade_group_operations_test.rs`

#### 2.2 Message Handler更新

**タスク**:
- [ ] `handle_request_config()`を拡張
  - Master向けロジック追加
  - Slave向けロジックを新構造対応
- [ ] `MasterConfigMessage` struct作成
- [ ] `SlaveConfigMessage` struct更新
- [ ] テスト更新

**ファイル**:
- `relay-server/src/message_handler.rs`
- `relay-server/tests/config_distribution_test.rs`

**変更内容**:
```rust
async fn handle_request_config(&mut self, message: RequestConfigMessage) {
    match message.ea_type.as_str() {
        "Master" => {
            // 新規: Master向けロジック
            let settings = self.db.get_settings_for_master(&message.account_id).await?;
            let config = MasterConfigMessage {
                account_id: message.account_id.clone(),
                symbol_prefix: settings.symbol_prefix,
                symbol_suffix: settings.symbol_suffix,
                config_version: settings.config_version,
            };
            self.config_publisher.send_config(&config).await?;
        }
        "Slave" => {
            // 既存: 新構造対応
            let settings_list = self.db.get_settings_for_slave(&message.account_id).await?;
            for settings in settings_list {
                let config = SlaveConfigMessage { /* ... */ };
                self.config_publisher.send_config(&config).await?;
            }
        }
        _ => { /* error */ }
    }
}
```

#### 2.3 REST API更新

**タスク**:
- [ ] GET `/api/trade-groups` - 全TradeGroup取得
- [ ] GET `/api/trade-groups/:id` - 特定TradeGroup取得
- [ ] POST `/api/trade-groups` - TradeGroup作成
- [ ] PUT `/api/trade-groups/:id/master-settings` - Master設定更新
- [ ] POST `/api/trade-groups/:id/members` - Member追加
- [ ] PUT `/api/trade-groups/:id/members/:slave` - Member設定更新
- [ ] DELETE `/api/trade-groups/:id/members/:slave` - Member削除
- [ ] API統合テスト

**ファイル**:
- `relay-server/src/api/trade_groups.rs` (新規)
- `relay-server/tests/api/trade_groups_test.rs` (新規)

---

### Phase 3: Master EA設定配信実装

#### 3.1 Master EA Config Socket実装

**MT5版タスク** (`mt-advisors/MT5/SankeyCopierMaster.mq5`):
- [ ] Config SUB socket変数追加
- [ ] OnInit: Config socket初期化（port 5557）
- [ ] OnInit: AccountIDでサブスクライブ
- [ ] OnTimer: ハートビート成功後にRequestConfig送信
- [ ] OnTimer: Config socketからメッセージ受信
- [ ] `ProcessMasterConfigMessage()` 関数実装
- [ ] MagicFilter関連コード削除

**MT4版タスク** (`mt-advisors/MT4/SankeyCopierMaster.mq4`):
- [ ] MT5版と同様の変更

**コード例**:
```mql5
// Global variables
int g_config_socket = INVALID_HANDLE;
string g_symbol_prefix = "";
string g_symbol_suffix = "";

int OnInit() {
    // ... existing code ...

    // Create config socket
    g_config_socket = zmq_socket_create(ZMQ_SUB);
    if (g_config_socket == INVALID_HANDLE) {
        Print("Failed to create config socket");
        return INIT_FAILED;
    }

    if (!zmq_socket_connect(g_config_socket, "tcp://localhost:5557")) {
        Print("Failed to connect config socket");
        return INIT_FAILED;
    }

    // Subscribe to own account ID
    if (!zmq_socket_subscribe(g_config_socket, AccountID)) {
        Print("Failed to subscribe to config topic");
        return INIT_FAILED;
    }

    Print("Config socket initialized and subscribed to topic: ", AccountID);
    return INIT_SUCCEEDED;
}

void OnTimer() {
    // ... existing heartbeat logic ...

    // Request config after first successful heartbeat
    if (g_heartbeat_success && !g_config_requested) {
        SendRequestConfig(g_zmq_socket, AccountID, "Master");
        g_config_requested = true;
    }

    // Receive config messages
    uchar config_buffer[];
    while (zmq_socket_recv_nonblocking(g_config_socket, config_buffer)) {
        ProcessConfigMessage(config_buffer);
    }
}

void ProcessConfigMessage(uchar &buffer[]) {
    // Parse topic and payload
    int space_pos = ArraySearchLinear(buffer, ' ');
    if (space_pos < 0) return;

    string topic = CharArrayToString(buffer, 0, space_pos);

    // Extract MessagePack payload
    uchar msgpack_data[];
    int msgpack_size = ArraySize(buffer) - space_pos - 1;
    ArrayResize(msgpack_data, msgpack_size);
    ArrayCopy(msgpack_data, buffer, 0, space_pos + 1, msgpack_size);

    // Deserialize using DLL
    string prefix = config_get_string(msgpack_data, "symbol_prefix");
    string suffix = config_get_string(msgpack_data, "symbol_suffix");

    // Update settings
    if (prefix != g_symbol_prefix || suffix != g_symbol_suffix) {
        g_symbol_prefix = prefix;
        g_symbol_suffix = suffix;
        Print("Master config updated: prefix=", prefix, ", suffix=", suffix);
    }
}
```

#### 3.2 MasterConfigMessage構造定義

**Rust側** (`mt-bridge/src/msgpack.rs`):
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterConfigMessage {
    pub account_id: String,
    pub symbol_prefix: Option<String>,
    pub symbol_suffix: Option<String>,
    pub config_version: u32,
    pub timestamp: String,
}
```

**MQL側** (`mt-advisors/Include/SankeyCopier/Common.mqh`):
```mql5
struct MasterConfig {
    string account_id;
    string symbol_prefix;
    string symbol_suffix;
    int    config_version;
};
```

#### 3.3 Master-Server統合テスト

**テストシナリオ**:
1. Master EA起動 → Config socket接続確認
2. Heartbeat送信 → RequestConfig送信確認
3. Relay Server → MasterConfigMessage送信確認
4. Master EA → 設定受信・適用確認
5. 設定変更（Web UI） → 動的更新確認

**テストファイル**:
- `relay-server/tests/master_config_integration_test.rs`

---

### Phase 4: Web UI更新

#### 4.1 API Client更新

**タスク**:
- [ ] TradeGroups API client作成
- [ ] Master設定取得/更新API追加
- [ ] 型定義更新

**ファイル**:
- `web-ui/lib/api/tradeGroups.ts` (新規)
- `web-ui/types/tradeGroup.ts` (新規)

#### 4.2 UI実装

**タスク**:
- [ ] TradeGroup一覧ページ
- [ ] TradeGroup詳細ページ
- [ ] Master設定編集フォーム（Prefix/Suffix）
- [ ] Member管理UI

**ファイル**:
- `web-ui/app/trade-groups/page.tsx` (新規)
- `web-ui/app/trade-groups/[id]/page.tsx` (新規)
- `web-ui/components/tradeGroups/MasterSettingsForm.tsx` (新規)

---

### Phase 5: クリーンアップ & 最終検証

#### 5.1 冗長フィールド削除

**タスク**:
- [ ] `trade_group_id`フィールドを全削除
  - `ConfigMessage`構造から削除
  - Slave EA受信処理から削除
  - 直接`master_account`を使用
- [ ] `MagicFilter`関連コード削除
  - Master EA Input parameterから削除
  - フィルタリングロジック削除
  - 関連テスト削除

#### 5.2 エンドツーエンドテスト

**テストシナリオ**:
1. TradeGroup作成（Web UI）
2. Master EA接続・設定受信
3. Member追加（Web UI）
4. Slave EA接続・設定受信
5. Master設定変更 → Master EA動的更新
6. Slave設定変更 → Slave EA動的更新
7. トレードシグナル送信・受信
8. TradeGroup削除

#### 5.3 ドキュメント更新

**タスク**:
- [ ] アーキテクチャ図更新
- [ ] API仕様書更新
- [ ] README更新
- [ ] 設定ガイド作成

**ファイル**:
- `docs/architecture/database-schema.md`
- `docs/api/trade-groups-api.md`
- `docs/guides/master-configuration.md`

---

## テスト戦略

### TDD原則

1. **Red**: テストを先に書き、失敗を確認
2. **Green**: 最小限のコードで成功させる
3. **Refactor**: コードを改善

### テストレベル

#### Unit Tests
- DB操作メソッド単体
- 設定パース・シリアライズ
- フィルタリングロジック

#### Integration Tests
- Relay Server ↔ Database
- Message Handler ↔ ConfigPublisher
- API ↔ Database

#### End-to-End Tests
- Master EA ↔ Relay Server ↔ Database
- Slave EA ↔ Relay Server ↔ Database
- Web UI ↔ API ↔ Database

### テストカバレッジ目標

- DB層: 90%以上
- Relay Server: 85%以上
- EA: 手動テスト（自動化困難）

---

## リスクと対策

### リスク1: 既存機能の破壊

**対策**:
- 包括的な回帰テスト
- Slave EA設定配信の動作検証
- 段階的なリリース

### リスク2: マイグレーションの失敗

**対策**:
- DBバックアップ必須
- ロールバック手順の準備
- ステージング環境での事前検証

### リスク3: Master/Slave EA更新のタイミング

**対策**:
- 後方互換性の考慮（旧EA対応）
- 段階的なEAアップデート
- エラーハンドリング強化

### リスク4: Web UIとバックエンドの不整合

**対策**:
- API仕様の明確化
- 型定義の共有（TypeScript + Rust）
- API統合テストの徹底

---

## 進捗管理

### マイルストーン

- [ ] **M1**: Phase 1完了（DB再構成）- 2025-XX-XX
- [ ] **M2**: Phase 2完了（Relay Server）- 2025-XX-XX
- [ ] **M3**: Phase 3完了（Master EA）- 2025-XX-XX
- [ ] **M4**: Phase 4完了（Web UI）- 2025-XX-XX
- [ ] **M5**: Phase 5完了（最終検証）- 2025-XX-XX

### 進捗レポート

週次で以下を更新:
- 完了タスク
- 進行中タスク
- ブロッカー
- リスク状況

---

## 次のステップ

Phase 2（次回実装予定）:
- Symbol Mappingバグ修正
- DLL配列取得API実装
- Slave EA側のTODO解消

---

## 参考資料

- [調査レポート](./investigation-report.md)
- [現状のDB構造](../architecture/database-schema.md)
- [MessagePack仕様](../architecture/messagepack-protocol.md)

---

**最終更新**: 2025-11-24
**次回レビュー**: Phase 1完了時
