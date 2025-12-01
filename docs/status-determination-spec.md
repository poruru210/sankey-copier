# ステータス判定仕様書（Status Determination Specification）

> **注意**: このドキュメントは作業用の参照資料です。修正完了後、本来のドキュメントにマージされます。

## 概要

このドキュメントは、SANKEY Copierシステムにおける、MasterおよびSlaveのステータス判定ロジックを定義します。

- `ENABLED (1)` は存在しません（Slave専用）
- `REMOVED (3)` は削除時の特殊状態

### ステータスを決定する4つの要素

1. **CONFIG設定の有無**: Slaveとの接続情報がデータベースに存在するか
2. **Web UI Switch**: Web UI上でON/OFFが切り替えられているか
3. **EA自動売買許可**: MT4/MT5のEA側で自動売買が許可されているか (`is_trade_allowed`)
4. **ハートビート送信**: EAからRelay Serverへハートビートが届いているか

### 判定ロジック

| CONFIG | Switch | 自動売買 | ハートビート | ステータス | 説明 |
|:------:|:------:|:--------:|:------------:|:----------:|------|
| ✅ | ✅ | ✅ | ✅ | `CONNECTED (2)` | トレードシグナルを正常にRelay Serverに送信できる状態 |
| ❌ | - | - | - | `DISABLED (0)` | CONFIG設定なし |
| ✅ | ❌ | - | - | `DISABLED (0)` | Web UIでOFF |
| ✅ | ✅ | ❌ | - | `DISABLED (0)` | EA自動売買がOFF |
| ✅ | ✅ | ✅ | ❌ | `DISABLED (0)` | EAが起動していない、またはタイムアウト |

**重要**: **4つの条件すべてを満たす場合のみ `CONNECTED (2)`、それ以外はすべて `DISABLED (0)`**

### コード実装

```rust
// relay-server/src/models/status.rs
pub fn calculate_master_status(input: &MasterStatusInput) -> i32 {
    if !input.web_ui_enabled || !input.is_trade_allowed {
        STATUS_DISABLED  // 0
    } else {
        STATUS_CONNECTED  // 2
    }
}
```

**注**: ハートビートの有無は、`is_trade_allowed`の取得可否で判定されます（ハートビートがない場合、`is_trade_allowed`は取得できません）。

---

## Slaveのステータス判定

### 基本原則

**Slaveには `DISABLED (0)`, `ENABLED (1)`, `CONNECTED (2)` の3つの状態が存在します。**

### ステータスを決定する要素

Slaveのステータスは、**Slave自体の条件**と**接続しているMasterの状態**の両方で決定されます。

#### Slave自体の条件（Masterと同様）

1. **CONFIG設定の有無**: Masterとの接続情報がデータベースに存在するか
2. **Web UI Switch**: Web UI上でON/OFFが切り替えられているか
3. **EA自動売買許可**: MT4/MT5のEA側で自動売買が許可されているか (`is_trade_allowed`)
4. **ハートビート送信**: EAからRelay Serverへハートビートが届いているか

#### 接続Masterの状態

5. **接続しているMasterのステータス**: 接続している各Masterが `CONNECTED` か `DISABLED` か

### 判定ロジック

| Slave自体の条件 | 接続Masterの状態 | Slaveのステータス | 説明 |
|----------------|-----------------|:----------------:|------|
| Switch❌ または 自動売買❌ または ハートビート❌ | - | `DISABLED (0)` | Slave自体が無効状態 |
| Switch✅ かつ 自動売買✅ かつ ハートビート✅ | **少なくとも1つのMasterが DISABLED** | `ENABLED (1)` | Slave自体は正常だが、Masterからの信号を受信できない |
| Switch✅ かつ 自動売買✅ かつ ハートビート✅ | **すべてのMasterが CONNECTED** | `CONNECTED (2)` | 正常にコピー取引可能 |

### N:N接続の考慮

このシステムはMasterとSlaveのN:N接続を許可します。

**例**: Slave Aが Master1, Master2, Master3 に接続している場合

| Master1 | Master2 | Master3 | Slave Aのステータス |
|:-------:|:-------:|:-------:|:------------------:|
| CONNECTED | CONNECTED | CONNECTED | `CONNECTED (2)` |
| CONNECTED | CONNECTED | DISABLED | `ENABLED (1)` |
| DISABLED | DISABLED | DISABLED | `ENABLED (1)` |

**ルール**: **すべてのMasterが `CONNECTED` の場合のみ Slaveは `CONNECTED (2)`、それ以外は `ENABLED (1)`**

### コード実装

```rust
// relay-server/src/models/status.rs
pub fn calculate_slave_status(input: &SlaveStatusInput) -> i32 {
    // Slave自体が無効な場合
    if !input.web_ui_enabled || !input.is_trade_allowed {
        return STATUS_DISABLED;  // 0
    }
    
    // Slave自体は有効だが、Masterの状態で判定
    if input.master_status == STATUS_CONNECTED {
        STATUS_CONNECTED  // 2
    } else {
        STATUS_ENABLED    // 1
    }
}
```

---

## Web UIでの表示

### Switchの色

- **青色**: ON（有効化）
- **灰色**: OFF（無効化）

### ステータスバー（ノードの枠線）の色

- **緑色**: ハートビートあり（EAが正常に接続中）
- **灰色**: ハートビートなし（EAが起動していない、またはタイムアウト）
- **赤色**: エラー状態（将来の拡張用）

### 表示の組み合わせ例

| Switch | ステータスバー | EA内部ステータス | 意味 |
|:------:|:-------------:|:---------------:|------|
| 青 | 緑 | CONNECTED | 正常動作中 |
| 青 | 灰 | DISABLED | ONにしたがEA未起動 |
| 灰 | 緑 | DISABLED | EAは動作中だがOFF設定 |
| 灰 | 灰 | DISABLED | OFFかつEA未起動 |

---

## 現在の問題（2025-11-30）

### 症状

**Exness Master (277195421)** において:

- **Web UI表示**: Switch青色、ステータスバー緑色 → `CONNECTED` と認識
- **Relay Serverログ**: `status=2 enabled=true is_trade_allowed=true` → `CONNECTED (2)` を送信
- **Master EAパネル**: `Status: 0 (DISABLED)` と表示 → `DISABLED` を受信

### 確認された条件

| 条件 | 状態 | 確認方法 |
|------|------|---------|
| CONFIG設定 | ✅ あり | Web UIでXM Slaveとの接続線を確認 |
| Switch | ✅ ON | Web UIで青色表示 |
| 自動売買 | ✅ ON | Relay Serverログで `is_trade_allowed=true` |
| ハートビート | ✅ あり | Web UIステータスバー緑色 |

**期待されるステータス**: `CONNECTED (2)`  
**実際のEA表示**: `DISABLED (0)`

### 調査が必要な項目

1. データベースの`trade_groups`テーブルで`enabled`フィールドの値を確認
2. 動作中のRelay Serverプロセスを確認（開発版 vs プロダクション版）
3. Master EAがどのRelay Serverに接続しているか確認
4. MessagePackのシリアライゼーション/デシリアライゼーションを確認

---

## 参考情報

### 関連ファイル

- **ステータス計算ロジック**: `relay-server/src/models/status.rs`
- **Master設定送信**: `relay-server/src/api/trade_groups.rs` (`send_config_to_master`)
- **Slave設定送信**: `relay-server/src/api/trade_groups.rs` (`send_config_to_slaves`)
- **ハートビート処理**: `relay-server/src/message_handler/heartbeat.rs`
- **設定リクエスト処理**: `relay-server/src/message_handler/config_request.rs`

### 過去の関連修正

- [Conversation 58e53c3e](https://github.com/user/repo): Fix Slave Symbol Prefix - Slave設定の送信バグ修正
- [Conversation 4716d959](https://github.com/user/repo): Refine Relay Server Logic - ステータス計算ロジックの修正

---

## 実装の詳細

### Slaveステータス更新のトリガー条件

**原則**: Slaveに`SlaveConfigMessage`を送信するのは、**Slave自身のstatusが変更されたとき**のみ。

#### 実装 (relay-server/src/message_handler/heartbeat.rs)

Master の heartbeat を受信したとき：

1. その Master に接続している Slave のリストを取得
2. 各 Slave について：
   - その Slave が接続している**すべての Master** の status を取得
   - すべての Master が CONNECTED かどうかを判定
   - Slave の新しい status を計算：
     - Slave 自体が無効（web_ui OFF または is_trade_allowed OFF）→ `DISABLED (0)`
     - すべての Master が CONNECTED → `CONNECTED (2)`
     - それ以外 → `ENABLED (1)`
   - 前回の status（データベースの `member.status`）と比較
   - **status が変更された場合のみ** `SlaveConfigMessage` を送信
   - データベースの status を更新

#### データベースメソッド

```rust
// relay-server/src/db/trade_group_members.rs

/// Get all Masters (trade_group_ids) that a Slave is connected to
pub async fn get_masters_for_slave(&self, slave_account: &str) -> Result<Vec<String>>
```

このメソッドは、指定された Slave が接続しているすべての Master のリストを返します。

#### ステータス変更検出

```rust
let old_slave_status = member.status;
if new_slave_status == old_slave_status {
    // Status unchanged, skip sending config
    continue;
}
// Status changed, send SlaveConfigMessage and update database
```

### N:N接続のサポート

このシステムは Master と Slave の N:N 接続を完全にサポートします。

**例**: Slave A が Master1, Master2, Master3 に接続している場合

- Master1 が heartbeat を送信 → Slave A の status を計算（Master1, Master2, Master3 すべての status を考慮）
- Master2 が heartbeat を送信 → Slave A の status を計算（Master1, Master2, Master3 すべての status を考慮）
- いずれの場合も、Slave A の status が変更された場合のみ `SlaveConfigMessage` を送信

---

**作成日**: 2025-11-30  
**最終更新**: 2025-12-01  
**ステータス**: 実装完了
