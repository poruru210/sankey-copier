# E2E Testing for Forex Copier

## 概要

このディレクトリには、Forex Copierシステムの統合テスト（E2Eテスト）が含まれています。

## テストシナリオ

`e2e_test.py` は以下のシナリオをテストします:

1. **Master EA登録**: Master EAがサーバーに登録される
2. **Slave EA登録**: Slave EAがサーバーに登録される
3. **接続確認**: 両方のEAが正しく接続されているか確認
4. **設定作成と配信**:
   - Web-UIから設定を作成
   - Slaveが設定メッセージを受信
   - 設定内容が正しいか検証
5. **トレードシグナル配信**:
   - Masterからトレードシグナルを送信
   - Slaveがシグナルを受信
   - シグナル内容が正しいか検証

## セットアップ

### 1. Python依存関係のインストール

```bash
cd tests
pip install -r requirements.txt
```

### 2. Rust Serverの起動

別のターミナルで:

```bash
cd rust-server
cargo run --release
```

サーバーが起動するのを待ちます（ポート: 8080, 5555, 5556, 5557）

## テスト実行

```bash
cd tests
python e2e_test.py
```

## 期待される出力

テストが成功すると、以下のような出力が表示されます:

```
============================================================
  Forex Copier E2E Test
============================================================

=== Step 1: Register Master EA ===
  Sent: {...}
✓ Master EA registered

=== Step 2: Register Slave EA ===
  Sent: {...}
✓ Slave EA registered

=== Step 3: Verify Connections ===
  Connections: {...}
✓ Master EA (MASTER_001) is connected
✓ Slave EA (SLAVE_001) is connected

=== Step 4: Create Copy Settings (triggers config message) ===
  Response: {...}
✓ Copy settings created (ID: 1)
  Listening for config messages on topic 'SLAVE_001'...
  Received raw: SLAVE_001 {...}
  Parsed config: {...}
✓ Slave received config message
✓ Config message verified

=== Step 5: Send Trade Signal and Verify Reception ===
  Listening for trade signals on topic 'MASTER_001'...
  Sent: {...}
✓ Trade signal sent from Master
  Received raw: MASTER_001 {...}
  Parsed signal: {...}
✓ Slave received trade signal
✓ Trade signal verified

============================================================
  ALL E2E TESTS PASSED!
============================================================
```

## トラブルシューティング

### サーバーに接続できない

- Rust Serverが起動しているか確認
- ポート 8080, 5555, 5556, 5557 が使用可能か確認

### タイムアウトエラー

- Serverのログを確認
- ZeroMQの接続が正しく確立されているか確認

### メッセージが受信されない

- Serverのログでメッセージ送信を確認
- ZeroMQのトピックが正しいか確認（MASTER_001, SLAVE_001）

## 追加テスト

将来的に追加できるテストシナリオ:

- [ ] 複数Slaveへの同時配信
- [ ] Lot multiplier適用の検証
- [ ] Reverse trade機能の検証
- [ ] Symbol mapping機能の検証
- [ ] フィルター機能（allowed/blocked symbols）の検証
- [ ] 設定の更新と再配信
- [ ] EA接続タイムアウト処理
