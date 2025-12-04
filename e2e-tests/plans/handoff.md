# E2Eテスト移行 - 引き継ぎ情報

## 現在のステータス

| ステップ | 状態 | 内容 |
|---------|------|------|
| Step 0 | ✅ 完了 | `RelayServerProcess` 一時ディレクトリ方式 |
| Step 1 | ✅ 完了 | `helpers.rs` ヘルパー関数抽出 |
| Step 2 | ✅ 完了 | シミュレーターメソッド追加 |
| Step 3 | ✅ 完了 | テストファイル移行 |
| Step 4 | ⏳ 未着手 | 旧テストファイル削除 |

## 移行済みテストファイル (合計67テスト)

| ファイル | テスト数 | 状態 |
|---------|---------|------|
| `smoke_test.rs` | 4 | ✅ 作成完了・ビルド通過 |
| `order_lifecycle.rs` | 10 | ✅ 作成完了・ビルド通過 |
| `order_routing.rs` | 8 | ✅ 作成完了・ビルド通過 (delayed signal tests追加) |
| `order_transform.rs` | 5 | ✅ 作成完了・ビルド通過 |
| `order_filter.rs` | 11 | ✅ 作成完了・ビルド通過 |
| `config_distribution.rs` | 15 | ✅ 作成完了・ビルド通過 (Master config, member mgmt tests追加) |
| `sync_protocol.rs` | 7 | ✅ 作成完了・ビルド通過 |
| `global_config.rs` | 6 | ✅ 作成完了・ビルド通過 (VLogs ZMQ broadcast tests追加) |
| `runtime_status.rs` | 1 | ✅ 作成完了・ビルド通過 |

## 追加された主要機能

### lib.rs シミュレーターメソッド追加

**MasterEaSimulator:**
- `send_request_config()` - Master設定リクエスト送信
- `try_receive_master_config()` - Master設定受信
- `create_delayed_signal()` - 遅延タイムスタンプ付きシグナル作成
- `subscribe_to_global_config()` - VLogs用グローバルトピック購読
- `try_receive_vlogs_config()` - VLogs設定受信

**SlaveEaSimulator:**
- `send_request_config()` - Slave設定リクエスト送信
- `subscribe_to_global_config()` - VLogs用グローバルトピック購読
- `try_receive_vlogs_config()` - VLogs設定受信

### カバレッジ改善で追加されたテスト

1. **Master設定配信** (config_distribution.rs)
   - `test_master_config_distribution`
   - `test_master_config_not_found`

2. **遅延シグナル処理** (order_routing.rs)
   - `test_delayed_signal_immediate`
   - `test_delayed_signal_acceptable`
   - `test_stale_signal_too_old`

3. **エラーハンドリング** (order_lifecycle.rs)
   - `test_close_nonexistent_position`
   - `test_close_already_closed`
   - `test_modify_multiple_times`

4. **メンバー管理とステータス変更** (config_distribution.rs)
   - `test_toggle_member_status_off_sends_disabled_config`
   - `test_delete_member_sends_disabled_config`
   - `test_allow_new_orders_follows_status`
   - `test_slave_config_prefix_distribution`

5. **VLogs ZMQブロードキャスト** (global_config.rs)
   - `test_master_receives_vlogs_on_registration`
   - `test_slave_receives_vlogs_on_registration`
   - `test_vlogs_broadcast_on_api_update`

## 次のセッションで実行すべきこと

### 1. E2Eテスト実行・検証

すべてのテストファイル移行が完了しました。次は実際にテストを実行して検証します。

```powershell
# 全テスト一覧確認
cd e2e-tests
cargo test -- --list

# 単一テスト実行（例）
cargo test --test order_lifecycle test_open_close_cycle -- --ignored --nocapture

# 全E2Eテスト実行
cargo test -- --ignored
```

### 2. 旧テストファイル削除（全テスト通過後）

```powershell
Remove-Item relay-server/tests/test_server.rs
Remove-Item relay-server/tests/e2e_trade_signal_test.rs
Remove-Item relay-server/tests/e2e_config_test.rs
Remove-Item relay-server/tests/e2e_sync_protocol_test.rs
Remove-Item relay-server/tests/e2e_global_config_test.rs
Remove-Item relay-server/tests/e2e_runtime_status_test.rs
```

### 3. 設計ノート

**遅延シグナルテストについて:**
- サーバー側ではシグナルのタイムスタンプフィルタリングは行わない
- EA側で `max_signal_delay_ms` を使用して古いシグナルを無視する
- テストはサーバーが正常にシグナルを配信することを確認（EA側の判断をシミュレート）

**VLogs設定テストについて:**
- グローバルトピック `config/global` は `mt-bridge/src/ffi.rs` の `get_global_config_topic()` から取得
- VLogsConfigMessage はEA登録時およびAPI更新時にブロードキャストされる
Remove-Item relay-server/tests/e2e_global_config_test.rs
Remove-Item relay-server/tests/e2e_runtime_status_test.rs
```

## 重要ファイルパス

| ファイル | 用途 |
|---------|------|
| `e2e-tests/plans/migration-plan.md` | 詳細計画 |
| `e2e-tests/src/relay_server_process.rs` | サーバープロセス管理 |
| `e2e-tests/src/helpers.rs` | 共通ヘルパー関数 |
| `e2e-tests/src/lib.rs` | シミュレーター実装 |
| `e2e-tests/tests/*.rs` | 移行済みテスト |
| `relay-server/tests/e2e_*.rs` | 元テスト（移行元、削除待ち） |
| `mt-bridge/src/ffi.rs` | FFI関数（get_global_config_topic等） |

## テスト実行コマンド

```bash
# ビルドチェック
cd e2e-tests && cargo check --tests

# 単一テスト実行
cargo test --test order_lifecycle test_open_close_cycle -- --ignored --nocapture

# 全E2Eテスト実行
cargo test -- --ignored

# テスト一覧表示
cargo test -- --list
```

## 移行完了サマリー

- **総テスト数**: 67テスト
- **テストファイル**: 9ファイル
- **lib.rs追加メソッド**: 8メソッド（MasterEaSimulator 5, SlaveEaSimulator 3）
- **カバレッジ**: 旧テストの必須項目 + 追加カバレッジ（遅延シグナル、Master設定、VLogs ZMQ配信）
