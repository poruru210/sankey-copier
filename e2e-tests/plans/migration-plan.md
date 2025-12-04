# E2Eテスト移行計画 (TestServer → RelayServerProcess)

## 概要

全68テストを `relay-server/tests/` から `e2e-tests/tests/` に移行。
一時ディレクトリ方式で並列実行対応し、テストを論理カテゴリで8ファイルに再編成する。

## Steps

### Step 0: `RelayServerProcess` を一時ディレクトリ方式に修正

**ファイル**: `e2e-tests/src/relay_server_process.rs`

**変更内容**:
- `tempfile::tempdir()` で一意な作業ディレクトリ作成
- `config.test.toml`, `certs/` を一時ディレクトリにコピー
- `runtime.toml`, `e2e_test.db` は一時ディレクトリ内に生成
- `Cargo.toml` に `tempfile` 依存追加

**目的**: 並列テスト実行時のリソース競合（DB、runtime.toml）を防止

### Step 1: ヘルパー関数を `e2e-tests/src/helpers.rs` に抽出

**新規ファイル**: `e2e-tests/src/helpers.rs`

**抽出する関数**:
- `default_test_slave_settings()` → `SlaveSettings` デフォルト値
- `setup_test_scenario(db, master, slaves, settings_fn)` → DB初期化
- `set_member_status(db, master, slave, status)` → Status変更
- `register_all_eas(master, slaves)` → Heartbeat送信

**引数変更**: `server: &TestServer` → `db: &Database`

### Step 2: シミュレーターに不足メソッドを追加

**ファイル**: `e2e-tests/src/lib.rs`

**MasterEaSimulator 追加メソッド**:
- `send_position_snapshot(positions: Vec<PositionInfo>)`
- `try_receive_sync_request(timeout_ms: i32) -> Option<SyncRequestMessage>`

**SlaveEaSimulator 追加メソッド**:
- `send_sync_request(last_sync_time: Option<String>)`
- `send_request_config()`
- `try_receive_position_snapshot(timeout_ms: i32) -> Option<PositionSnapshotMessage>`

### Step 3: テストファイル移行・再編成 (8ファイル)

| 新ファイル名 | テスト数 | 内容 |
|-------------|---------|------|
| `order_lifecycle.rs` | 12 | Open/Close/Modify基本サイクル、複数オーダー順次/並列/連射 |
| `order_routing.rs` | 8 | マルチマスター分離、マルチスレーブ配信、遅延・タイムスタンプ検証 |
| `order_transform.rs` | 6 | シンボルPrefix/Suffix変換、シンボルマッピング、リバーストレード |
| `order_filter.rs` | 10 | 部分決済、ロット制限、シンボル/マジック許可・拒否、指値注文 |
| `config_distribution.rs` | 17 | Config配信、Status変更、SyncPolicy、シンボルPrefix設定 |
| `sync_protocol.rs` | 7 | PositionSnapshot配信、SyncRequestルーティング |
| `global_config.rs` | 7 | VLogsConfig配信・API・FFI解析 |
| `runtime_status.rs` | 1 | RuntimeStatus遷移（Standby→Connected） |

### Step 4: 旧テストファイル削除

**削除対象**:
- `relay-server/tests/test_server.rs`
- `relay-server/tests/e2e_trade_signal_test.rs`
- `relay-server/tests/e2e_config_test.rs`
- `relay-server/tests/e2e_sync_protocol_test.rs`
- `relay-server/tests/e2e_global_config_test.rs`
- `relay-server/tests/e2e_runtime_status_test.rs`

---

## テスト詳細マッピング

### order_lifecycle.rs (12テスト)

| 元行番号 | テスト名 | 内容 |
|---------|---------|------|
| 651 | `test_open_close_cycle` | 基本Open→Close |
| 750 | `test_open_modify_close_cycle` | Open→Modify→Close |
| 826 | `test_close_nonexistent_position` | 存在しないポジションClose |
| 877 | `test_close_already_closed` | 既にCloseしたポジションClose |
| 945 | `test_modify_sl_only` | SLのみ変更 |
| 996 | `test_modify_tp_only` | TPのみ変更 |
| 1046 | `test_modify_both_sl_tp` | SL/TP両方変更 |
| 1096 | `test_modify_multiple_times` | 複数回Modify |
| 1161 | `test_multiple_open_sequential` | 順次Open |
| 1217 | `test_multiple_open_parallel` | 並列Open |
| 1275 | `test_multiple_close_sequential` | 順次Close |
| 1350 | `test_rapid_fire_signals` | 高速連射 |

### order_routing.rs (8テスト)

| 元行番号 | テスト名 | 内容 |
|---------|---------|------|
| 1420 | `test_multi_master_signal_isolation` | マルチマスター分離 |
| 1522 | `test_multi_master_same_symbol_open` | 同一シンボル複数マスター |
| 1617 | `test_signal_broadcast_to_all_slaves` | 全スレーブ配信 |
| 1708 | `test_slave_individual_lot_multiplier` | スレーブ別ロット乗数 |
| 1791 | `test_signal_latency_measurement` | レイテンシ計測 |
| 1864 | `test_delayed_signal_immediate` | 即時遅延シグナル |
| 1919 | `test_delayed_signal_acceptable` | 許容遅延シグナル |
| 1974 | `test_stale_signal_too_old` | 古すぎるシグナル拒否 |

### order_transform.rs (6テスト)

| 元行番号 | テスト名 | 内容 |
|---------|---------|------|
| 2178 | `test_symbol_prefix_suffix_transformation` | Prefix/Suffix変換 |
| 2265 | `test_master_sends_all_symbols_no_filtering` | 全シンボル透過 |
| 2435 | `test_symbol_mapping` | シンボルマッピング |
| 2816 | `test_reverse_trade_buy_to_sell` | Buy→Sellリバース |
| 2872 | `test_reverse_trade_pending_orders` | 指値リバース |

### order_filter.rs (10テスト)

| 元行番号 | テスト名 | 内容 |
|---------|---------|------|
| 2040 | `test_partial_close_signal` | 部分決済 |
| 2119 | `test_full_close_signal_no_ratio` | 全決済（ratio無し） |
| 2504 | `test_allowed_symbols_filter` | 許可シンボル |
| 2572 | `test_blocked_symbols_filter` | 拒否シンボル |
| 2636 | `test_allowed_magic_numbers_filter` | 許可マジック番号 |
| 2724 | `test_blocked_magic_numbers_filter` | 拒否マジック番号 |
| 2941 | `test_source_lot_min_filter` | 最小ロット制限 |
| 3008 | `test_source_lot_max_filter` | 最大ロット制限 |
| 3080 | `test_multiple_sequential_partial_closes` | 連続部分決済 |
| 3158 | `test_pending_order_types` | 指値注文タイプ |
| 3271 | `test_pending_orders_disabled` | 指値無効時 |

### config_distribution.rs (17テスト)

元ファイル: `e2e_config_test.rs` 全体

### sync_protocol.rs (7テスト)

元ファイル: `e2e_sync_protocol_test.rs` 全体

### global_config.rs (7テスト)

元ファイル: `e2e_global_config_test.rs` 全体

### runtime_status.rs (1テスト)

元ファイル: `e2e_runtime_status_test.rs` 全体

---

## 変換パターン

```rust
// Before (TestServer)
let server = TestServer::start().await.expect("...");
let master = MasterEaSimulator::new(
    &server.zmq_pull_address(),
    &server.zmq_pub_config_address(),  // 3-port名残
    master_account,
);
let slave = SlaveEaSimulator::new(
    &server.zmq_pull_address(),
    &server.zmq_pub_config_address(),
    &server.zmq_pub_trade_address(),   // 3-port名残
    slave_account,
);
server.db.create_trade_group(master_account).await?;
server.shutdown().await;

// After (RelayServerProcess)
let server = RelayServerProcess::start().expect("...");
let db = Database::new(&server.db_url()).await?;
let master = MasterEaSimulator::new(
    &server.zmq_pull_address(),
    &server.zmq_pub_address(),         // 2-port統合
    master_account,
);
let slave = SlaveEaSimulator::new(
    &server.zmq_pull_address(),
    &server.zmq_pub_address(),         // 2-port統合
    slave_account,
    master_account,                     // 4th param追加
);
db.create_trade_group(master_account).await?;
server.shutdown();  // sync, not async
```

---

## テスト実行方法

```bash
# 全E2Eテスト実行
cargo test -p sankey-copier-e2e-tests --ignored

# 特定カテゴリのみ
cargo test -p sankey-copier-e2e-tests --test order_lifecycle --ignored

# 単一テスト
cargo test -p sankey-copier-e2e-tests --test order_lifecycle test_open_close_cycle --ignored
```
