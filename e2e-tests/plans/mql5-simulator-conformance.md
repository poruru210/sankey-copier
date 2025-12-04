# EA Simulator MQL5完全準拠実装

## 目的

MQL5 EA (`SankeyCopierSlave.mq5`, `SankeyCopierMaster.mq5`) の実装に完全準拠したSimulatorを実装。
OnTimerループ、グローバル変数、ステート遷移を忠実に再現。
外部からの操作は不可、読み取り専用の観測のみ許可。

## 設計原則

1. **SimulatorはEAの実装に完全準拠する** - MQL5のバグも再現し、E2Eテストでバグ発見を可能にする
2. **外部からの操作禁止** - `send_heartbeat()`, `send_request_config()` 等の内部操作メソッドは公開しない
3. **読み取り専用観測のみ** - `get_status()`, `wait_for_status()`, `has_received_config()` 等で状態を観測
4. **単一OnTimerスレッド** - MQL5と同じく、1つのスレッドで全処理（heartbeat, config要求, 受信）を実行

## MQL5参照箇所

| ファイル | 行範囲 | 内容 |
|---------|--------|------|
| `SankeyCopierSlave.mq5` | L48-67 | グローバル変数定義 |
| `SankeyCopierSlave.mq5` | L234-418 | OnTimer() 実装 |
| `SankeyCopierMaster.mq5` | L46-67 | グローバル変数定義 |
| `SankeyCopierMaster.mq5` | L225-343 | OnTimer() 実装 |
| `Common.mqh` | L28-49 | 定数定義 (HEARTBEAT_INTERVAL_SECONDS, STATUS_*) |

## 実装ステップ

### Step 1: types.rs にMQL5定数を追加 ✅

```rust
// MQL5 Common.mqh L28-49 準拠
pub const HEARTBEAT_INTERVAL_SECONDS: u64 = 30;
pub const ONTIMER_INTERVAL_MS: u64 = 100;
pub const STATUS_NO_CONFIG: i32 = -1;
pub const STATUS_DISABLED: i32 = 0;
pub const STATUS_ENABLED: i32 = 1;
pub const STATUS_CONNECTED: i32 = 2;
```

### Step 2: base.rs をZMQ接続管理のみに簡素化

- heartbeatスレッド削除
- `start()` メソッド削除
- ソケットハンドルを `pub(crate)` で公開
- Slave/Master が独自の OnTimer スレッドを実装

### Step 3: slave.rs MQL5 OnTimer() 完全準拠

**グローバル変数 (MQL5 L48-67準拠):**
```rust
g_initialized: bool
g_last_heartbeat: Instant      // datetime → Instant
g_config_requested: bool
g_last_trade_allowed: bool = false  // MQL5と同じ初期値
g_has_received_config: bool
g_configs: Vec<SlaveConfig>    // CopyConfig g_configs[]
```

**OnTimerループ (100ms間隔、MQL5 L234-418準拠):**
```
loop (100ms):
  1. ProcessTradeSignals() - trade socketをノンブロッキング受信
  
  2. Heartbeat判定:
     should_send = (now - g_last_heartbeat >= 30s) OR trade_state_changed
  
  3. Heartbeat送信成功時:
     - trade_state_changed → log, update g_last_trade_allowed
     - current_trade_allowed AND !g_config_requested → SendRequestConfig, g_config_requested = true
     - !trade_state_changed AND !g_config_requested → SendRequestConfig, g_config_requested = true
  
  4. Config受信 (ノンブロッキング):
     - topic解析 (space separator)
     - SlaveConfig → g_configs更新, g_has_received_config = true
     - trade topic自動購読
```

**外部API:**
- `new(push_addr, config_addr, account_id, master_account)` - コンストラクタ
- `start()` - OnTimerスレッド開始
- `set_trade_allowed(bool)` - auto-trading状態変更
- `get_status() -> i32` - 最後に受信したステータス
- `wait_for_status(expected, timeout) -> Option<SlaveConfig>` - ステータス待機
- `has_received_config() -> bool` - config受信済みか
- `account_id() -> &str` - アカウントID取得

### Step 4: master.rs MQL5 OnTimer() 完全準拠

**グローバル変数 (MQL5 L46-67準拠):**
```rust
g_initialized: bool
g_last_heartbeat: Instant
g_config_requested: bool
g_last_trade_allowed: bool = false  // MQL5と同じ初期値
g_server_status: i32 = STATUS_NO_CONFIG
g_symbol_prefix: String
g_symbol_suffix: String
```

**OnTimerループ (100ms間隔、MQL5 L225-343準拠):**
```
loop (100ms):
  1. Heartbeat判定:
     should_send = (now - g_last_heartbeat >= 30s) OR trade_state_changed
  
  2. Heartbeat送信成功時:
     - !g_config_requested AND current_trade_allowed → SendRequestConfig
  
  3. Config受信 (ノンブロッキング):
     - MasterConfig → g_server_status更新, symbol prefix/suffix更新
     - SyncRequest → ProcessSyncRequest()
```

**外部API:**
- `new(push_addr, config_addr, account_id)` - コンストラクタ
- `start()` - OnTimerスレッド開始
- `set_trade_allowed(bool)` - auto-trading状態変更
- `get_server_status() -> i32` - サーバーステータス
- `account_id() -> &str` - アカウントID取得
- Trade送信メソッド (OnTick/OnTradeTransaction相当として維持):
  - `send_trade_signal()`
  - `send_position_snapshot()`
  - `create_open_signal()`, `create_close_signal()`, etc.

### Step 5: テストファイル修正

**削除対象の呼び出し:**
- `slave_sim.send_heartbeat()`
- `slave_sim.send_request_config()`
- `master_sim.send_heartbeat()`
- `master_sim.send_request_config()`

**変更パターン:**
```rust
// Before
let mut slave_sim = SlaveEaSimulator::new(...)?;
slave_sim.send_heartbeat()?;
slave_sim.send_request_config()?;
let config = slave_sim.try_receive_config(1000)?;

// After
let mut slave_sim = SlaveEaSimulator::new(...)?;
slave_sim.set_trade_allowed(true);  // auto-trading ON
slave_sim.start()?;
let config = slave_sim.wait_for_status(STATUS_CONNECTED, 5000)?;
```

## 実装順序

1. ✅ types.rs - 定数追加
2. 🔄 base.rs - heartbeatスレッド削除、ソケット公開
3. 🔄 slave.rs - 完全書き換え
4. 🔄 master.rs - 完全書き換え
5. 🔄 テストファイル修正

## 注意事項

- `g_last_trade_allowed = false` 初期化により、`set_trade_allowed(true)` を呼ぶまでRequestConfigは送信されない
- Config受信時にtrade topic購読を自動実行（MQL5のProcessConfigMessage内で動的購読と同じ）
- shutdown時のUnregister送信は将来対応（現時点では不要）
