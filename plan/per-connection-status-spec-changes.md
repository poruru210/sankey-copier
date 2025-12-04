# Per-Connection Status 仕様変更サマリー

## 変更の背景

### 現在の問題

XM MT5 Slave が以下の 2 つの Master に接続している場合：
- XM MT4 Master: **CONNECTED** (オンライン・自動売買ON・WebUI有効)
- OTHER Master: **OFFLINE** (オフライン)

→ Slave の `runtime_status = 1` (ENABLED) となり、「待機」と表示される

**実際には XM MT4 Master からのトレードは正常にコピーされている** にも関わらず、ユーザーには「待機中」と見える。

### 根本原因

`evaluate_slave_status()` が `MasterClusterSnapshot`（全 Master の集約）を受け取り、「全 Master が CONNECTED なら CONNECTED、そうでなければ ENABLED」と判定していた。

## 仕様変更

### Before: Slave-level 評価

```
Slave Account
  └── runtime_status = evaluate(ALL connected Masters)
       └── Member 1: uses Slave's runtime_status
       └── Member 2: uses Slave's runtime_status
       └── Member 3: uses Slave's runtime_status
```

### After: Member-level 評価

```
Slave Account
  └── Member 1 (→ Master A): runtime_status = evaluate(Master A only)
  └── Member 2 (→ Master B): runtime_status = evaluate(Master B only)
  └── Member 3 (→ Master C): runtime_status = evaluate(Master C only)
  └── Account Badge: Math.min(Member 1, Member 2, Member 3)
```

## 変更される API レスポンス

### `GET /api/trade-group-members`

**Before**:
```json
[
  {
    "trade_group_id": "XM_MT4_MASTER",
    "slave_account": "XM_MT5_SLAVE",
    "runtime_status": 1,  // 全 Master の集約結果
    "warning_codes": ["MasterClusterDegraded"]
  },
  {
    "trade_group_id": "OTHER_MASTER",
    "slave_account": "XM_MT5_SLAVE",
    "runtime_status": 1,  // 同じ値（Slave 全体の評価）
    "warning_codes": ["MasterClusterDegraded"]
  }
]
```

**After**:
```json
[
  {
    "trade_group_id": "XM_MT4_MASTER",
    "slave_account": "XM_MT5_SLAVE",
    "runtime_status": 2,  // XM MT4 Master との接続: CONNECTED
    "warning_codes": []
  },
  {
    "trade_group_id": "OTHER_MASTER",
    "slave_account": "XM_MT5_SLAVE",
    "runtime_status": 1,  // OTHER Master との接続: ENABLED (Master offline)
    "warning_codes": ["MasterOffline"]
  }
]
```

## 変更される ZeroMQ メッセージ

### `SlaveConfigMessage`

**Before**:
```
{
  "account_id": "XM_MT5_SLAVE",
  "master_account": "XM_MT4_MASTER",
  "status": 1,  // 全 Master の集約結果
  "warning_codes": ["MasterClusterDegraded"],
  ...
}
```

**After**:
```
{
  "account_id": "XM_MT5_SLAVE",
  "master_account": "XM_MT4_MASTER",
  "status": 2,  // この Master との接続: CONNECTED
  "warning_codes": [],
  ...
}
```

## 変更される UI 表示

### Web UI Edge (SettingsEdge.tsx)

| 条件 | Before | After |
|------|--------|-------|
| XM MT4 Master (CONNECTED) → XM MT5 Slave | グレー表示 (status=1) | アニメーション表示 (status=2) |
| OTHER Master (OFFLINE) → XM MT5 Slave | グレー表示 (status=1) | グレー表示 (status=1) |

### Web UI Slave Node Badge

| 条件 | Before | After |
|------|--------|-------|
| XM MT5 Slave (複合接続) | 「待機」(status=1) | 「待機」(status=min(2,1)=1) |

**Note**: Badge は `Math.min()` で集約するため、1 つでも ENABLED/DISABLED な接続があれば「待機」または「手動OFF」と表示される。これは設計通り。

### EA Panel (GridPanel.mqh)

| 条件 | Before | After |
|------|--------|-------|
| XM MT4 Master config | 「待機」(status=1) | 「配信中」(status=2) |
| OTHER Master config | 「待機」(status=1) | 「待機」(status=1) |

## 変更されないもの

1. **`allow_new_orders`**: 引き続き Slave 側の条件（Web UI ON + Online）のみで決定
2. **DB スキーマ**: `trade_group_members.runtime_status` は既に Member ごとに存在
3. **Web UI Types**: `TradeGroupMember.runtime_status` の型は変更なし
4. **Master の評価ロジック**: 変更なし

## テスト観点

### 機能テスト

1. **正常系**: 全 Master CONNECTED → 全 Member CONNECTED
2. **部分障害**: 一部 Master OFFLINE → 該当 Member のみ ENABLED
3. **Slave 障害**: Slave OFFLINE → 全 Member DISABLED

### 表示テスト

1. **Edge**: 各 Edge が独立した status を反映
2. **Badge**: 複数接続時に最小 status を表示
3. **EA Panel**: 各 config が独立した status を表示

### 後方互換性

1. **API**: フィールド名・型は変更なし（値のセマンティクスのみ変更）
2. **EA**: `SlaveConfigMessage` の構造は変更なし
3. **Web UI**: `runtime_status` フィールドは既に per-member で存在
