# Per-Connection Status 評価への修正プラン

## 概要

現在の Status Engine は Slave ごとに「すべての接続 Master のステータスを集約」して単一の runtime_status を算出している。これにより：

- XM MT5 Slave が XM MT4 Master (CONNECTED) と OTHER Master (OFFLINE) に接続
- → Slave の status は ENABLED（全 Master が CONNECTED ではないため）
- → Web UI の Edge も ENABLED として表示
- → EA Panel も同様に ENABLED を表示

この設計では「どの Master との接続が健全か」がユーザーに伝わらない。

## 修正の方針

**各 Member（connection）ごとに独立した runtime_status を評価し、DB・API・EA・Web UI すべてで表示する。**

## 現状のアーキテクチャ

```
+------------------+       +---------------------+       +-------------------+
| evaluate_slave   |  <--  | MasterClusterSnap-  |  <--  | All Masters for   |
| _status()        |       | shot (全 Master)    |       | this Slave        |
+------------------+       +---------------------+       +-------------------+
         |
         v
  SlaveStatusResult (単一の status)
         |
         +---> SlaveConfigMessage.status
         +---> TradeGroupMember.runtime_status
         +---> Web UI Edge / Badge
         +---> EA Panel
```

## 新アーキテクチャ

```
+------------------------+       +-----------------------+
| evaluate_member_status |  <--  | SingleMasterSnapshot  |
| ()                     |       | (対象 Master のみ)     |
+------------------------+       +-----------------------+
         |
         v
  MemberStatusResult (per-connection status)
         |
         +---> SlaveConfigMessage.status (この connection の status)
         +---> TradeGroupMember.runtime_status (この connection の status)
         +---> Web UI Edge (この connection の status)
         +---> EA Panel per-config status
         
+------------------------+       +-----------------------+
| evaluate_slave_account |  <--  | All MemberStatusRes-  |
| _status()              |       | ults for this Slave   |
+------------------------+       +-----------------------+
         |
         v
  SlaveAccountStatusResult (集約 status, for account-level badge)
         |
         +---> Web UI Slave Node Badge (最小 status)
```

## 変更箇所

### 1. Status Engine (`status_engine.rs`)

#### 1.1 新関数: `evaluate_member_status()`

```rust
/// 単一の Master-Slave 接続（Member）のステータスを評価
pub fn evaluate_member_status(
    intent: SlaveIntent,
    slave_conn: ConnectionSnapshot,
    master_status_result: MasterStatusResult,
) -> MemberStatusResult {
    // Slave 側の条件
    let slave_web_ui_enabled = intent.web_ui_enabled;
    let slave_online = is_connection_online(slave_conn.connection_status);
    
    // Master 側の条件
    let master_connected = master_status_result.status == STATUS_CONNECTED;
    
    let status = if !slave_web_ui_enabled || !slave_online {
        STATUS_DISABLED
    } else if master_connected {
        STATUS_CONNECTED
    } else {
        STATUS_ENABLED
    };
    
    // Warning codes の収集...
    
    MemberStatusResult {
        status,
        allow_new_orders: slave_web_ui_enabled && slave_online,
        warning_codes,
    }
}
```

#### 1.2 既存関数の変更

`evaluate_slave_status()` は残すが、用途を明確化：
- **アカウントレベルの集約ステータス** 用（Web UI のノードバッジ用）
- Member ごとの評価は `evaluate_member_status()` を使用

### 2. Config Builder (`config_builder.rs`)

#### 2.1 `SlaveConfigContext` の変更

```rust
pub struct SlaveConfigContext<'a> {
    // 既存フィールド...
    
    // 削除: master_cluster: MasterClusterSnapshot
    // 追加: 
    pub master_status_result: MasterStatusResult, // 対象 Master のみ
}
```

#### 2.2 `build_slave_config()` の変更

```rust
pub fn build_slave_config(context: SlaveConfigContext) -> SlaveConfigBundle {
    let status_result = evaluate_member_status(
        context.intent,
        context.slave_connection_snapshot,
        context.master_status_result, // ← 単一の Master
    );
    // ...
}
```

### 3. Runtime Status Updater (`runtime_status_updater.rs`)

#### 3.1 新メソッド: `evaluate_member_runtime_status()`

```rust
pub async fn evaluate_member_runtime_status(
    &self,
    target: SlaveRuntimeTarget<'_>,
) -> MemberStatusResult {
    let master_result = self
        .evaluate_master_runtime_status(target.master_account)
        .await
        .unwrap_or_default();
    
    let slave_snapshot = self.slave_connection_snapshot(target.slave_account).await;
    
    evaluate_member_status(
        SlaveIntent { web_ui_enabled: target.enabled_flag },
        slave_snapshot,
        master_result,
    )
}
```

#### 3.2 `build_slave_bundle()` の変更

```rust
pub async fn build_slave_bundle(
    &self,
    target: SlaveRuntimeTarget<'_>,
) -> SlaveConfigBundle {
    let master_result = self
        .evaluate_master_runtime_status(target.master_account)
        .await
        .unwrap_or_default();
    
    // cluster ではなく単一の master_result を渡す
    ConfigBuilder::build_slave_config(SlaveConfigContext {
        // ...
        master_status_result: master_result,
        // ...
    })
}
```

### 4. DB 層 (`db/trade_group_members.rs`)

変更不要。既に Member ごとに `runtime_status` を保持している。

### 5. API 層

変更不要。既に `TradeGroupMember` に `runtime_status` と `warning_codes` を返している。

### 6. EA (MQL)

#### 6.1 `GridPanel.mqh`

既に `UpdateConfigList()` で per-config status を表示している。**変更不要**。

#### 6.2 Slave EA

`SlaveConfigMessage` の `status` フィールドは per-connection になるため、**変更不要**。

### 7. Web UI

#### 7.1 `useAccountData.ts`

現在の Slave `runtimeStatus` 算出ロジック:
```typescript
const statuses = receiverRuntimeStatuses.get(receiver.id) ?? [];
const runtimeStatus = statuses.length > 0 ? Math.min(...statuses) : 0;
```

これは **アカウントノードバッジ用の集約** として適切。**変更不要**。

#### 7.2 Edge (`SettingsEdge.tsx`)

```typescript
const isActive = setting?.runtime_status !== 0;
```

既に per-connection の `runtime_status` を参照。**変更不要**（サーバー側の修正で正しい値が来る）。

## WarningCode の Priority 追加

Status Engine が `warning_codes` をソートして表示優先度を制御できるよう、`WarningCode` に priority を追加。

### `mt-bridge/src/types.rs`

```rust
impl WarningCode {
    pub fn priority(&self) -> u8 {
        match self {
            WarningCode::SlaveWebUiDisabled => 10,
            WarningCode::SlaveOffline => 20,
            WarningCode::SlaveAutoTradingDisabled => 30,
            WarningCode::MasterWebUiDisabled => 40,
            WarningCode::MasterOffline => 50,
            WarningCode::MasterAutoTradingDisabled => 60,
            WarningCode::NoMasterAssigned => 70,
            WarningCode::MasterClusterDegraded => 80,
        }
    }
}
```

## テスト計画

### ユニットテスト

1. `evaluate_member_status()`:
   - Master CONNECTED + Slave OK → Member CONNECTED
   - Master OFFLINE + Slave OK → Member ENABLED
   - Slave OFFLINE → Member DISABLED
   - Slave Web UI OFF → Member DISABLED

2. `build_slave_config()`:
   - 単一 Master の status_result を正しく使用

### 統合テスト

1. 複数 Master に接続した Slave:
   - 各 Edge が個別の status を表示
   - Slave ノードバッジは最小 status を表示

2. EA Panel:
   - 各 config が個別の status を表示
   - 全体ステータスは最小 status を表示

## 仕様変更のまとめ

| 項目 | Before | After |
|------|--------|-------|
| Slave status 評価 | 全 Master cluster を集約 | 各 Member (connection) ごと |
| `SlaveConfigMessage.status` | 全 Master の集約結果 | 対象 Master との接続 status |
| `TradeGroupMember.runtime_status` | 全 Master の集約結果 | 対象 Master との接続 status |
| Web UI Edge | 集約 status | per-connection status |
| Web UI Slave Badge | 集約 status | `Math.min()` で集約（現行維持） |
| EA Panel per-config | 集約 status | per-connection status |
| `allow_new_orders` | Slave 側のみで決定 | 変更なし（現行維持） |

## 実装順序

1. **Phase 1: Status Engine**
   - `evaluate_member_status()` 追加
   - `WarningCode::priority()` 追加
   - ユニットテスト

2. **Phase 2: Config Builder**
   - `SlaveConfigContext` 変更
   - `build_slave_config()` 変更
   - ユニットテスト

3. **Phase 3: Runtime Status Updater**
   - `evaluate_member_runtime_status()` 追加
   - `build_slave_bundle()` 変更
   - 統合テスト

4. **Phase 4: E2E 検証**
   - 複数 Master シナリオ
   - EA Panel 表示確認
   - Web UI Edge 表示確認
