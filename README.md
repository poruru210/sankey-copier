# Forex Copier

高性能なMT4/MT5トレードコピーシステム。低遅延のローカル通信とスマートフォンからの制御が可能です。

## 特徴

- **双方向対応**: MT4↔MT5、MT4↔MT4、MT5↔MT5すべての組み合わせに対応
- **低遅延**: ZeroMQを使用したローカル通信で最小限の遅延
- **リモート制御**: スマートフォンからコピーのON/OFF、設定変更が可能
- **柔軟なシンボル変換**: プリフィックス/サフィックス削除・追加、完全なシンボル名マッピング
- **高度なフィルタリング**: 通貨ペア、マジックナンバーでフィルタリング
- **ロット調整**: 固定倍率、残高比率などに対応
- **リアルタイム監視**: WebUIでトレードコピーの状態をリアルタイム確認

## アーキテクチャ

```
[MT4/MT5 Master EA] --ZeroMQ--> [Rust中継サーバー] --ZeroMQ--> [MT4/MT5 Slave EA]
                                        ↑
                                   [Web UI API]
                                        ↑
                                  [スマホブラウザ]
```

## 必要要件

### サーバー側
- Rust (1.70以上)
- ZeroMQ library (libzmq)
- SQLite

### MT4/MT5側
- MetaTrader 4 または MetaTrader 5
- Rust (1.70以上) - ZeroMQ DLLのビルドに必要

### WebUI
- Node.js (18以上)
- npm または yarn

## インストール

### 1. Rustサーバーのセットアップ

```bash
cd rust-server

# 依存関係のインストール
# 初回実行時に自動でビルドされます

# サーバーの起動
cargo run --release
```

サーバーは以下のポートで起動します:
- HTTP/WebSocket API: `http://localhost:8080`
- ZeroMQ Master受信: `tcp://*:5555`
- ZeroMQ Slave送信: `tcp://localhost:5556`

### 2. WebUIのセットアップ

```bash
cd web-ui

# 依存関係のインストール
npm install

# 開発サーバーの起動
npm run dev

# 本番ビルド
npm run build
```

開発サーバー: `http://localhost:5173`

### 3. MT4/MT5 EAのインストール

#### ZeroMQ DLLのビルドと配置

このプロジェクトでは、RustでビルドしたカスタムZeroMQ DLL (`forex_copier_zmq.dll`) を使用します。

**DLLのビルド:**

MT4/MT5には32-bit版と64-bit版があります。お使いのMT4/MT5に合わせてビルドしてください。

```bash
cd mql-zmq-dll

# 32-bit版のビルド (MT4は通常32-bit、MT5は両方存在)
rustup target add i686-pc-windows-msvc
cargo build --release --target i686-pc-windows-msvc

# ビルドされたDLLをLibrariesフォルダにコピー
cp target/i686-pc-windows-msvc/release/forex_copier_zmq.dll ../mql/MT4/Libraries/
cp target/i686-pc-windows-msvc/release/forex_copier_zmq.dll ../mql/MT5/Libraries/
```

**64-bit版のMT5をお使いの場合:**

```bash
# 64-bit版のビルド
cargo build --release

# ビルドされたDLLをLibrariesフォルダにコピー
cp target/release/forex_copier_zmq.dll ../mql/MT5/Libraries/
```

**注意**: MT5のバージョン確認方法:
- MT5を起動し、「ヘルプ」→「バージョン情報」を確認
- または、MT5のインストールフォルダに `terminal64.exe` があれば64-bit版です

**MT4への配置:**
- `mql/MT4/Libraries/forex_copier_zmq.dll` を MT4の `MQL4/Libraries/` フォルダにコピー

**MT5への配置:**
- `mql/MT5/Libraries/forex_copier_zmq.dll` を MT5の `MQL5/Libraries/` フォルダにコピー

**重要**: MT4/MT5の設定で DLL の使用を許可する必要があります:
1. MT4/MT5の「ツール」→「オプション」を開く
2. 「エキスパートアドバイザ」タブを選択
3. 「DLLの使用を許可する」にチェックを入れる

#### EAファイルの配置

**MT4の場合:**
```
mql/MT4/Master/ForexCopierMaster.mq4
  → [MT4インストールフォルダ]/MQL4/Experts/

mql/MT4/Slave/ForexCopierSlave.mq4
  → [MT4インストールフォルダ]/MQL4/Experts/
```

**MT5の場合:**
```
mql/MT5/Master/ForexCopierMaster.mq5
  → [MT5インストールフォルダ]/MQL5/Experts/

mql/MT5/Slave/ForexCopierSlave.mq5
  → [MT5インストールフォルダ]/MQL5/Experts/
```

#### MT4/MT5でのコンパイル

1. MetaEditorを開く
2. 各EAファイルを開いてコンパイル (F7)
3. エラーがないことを確認

## 使用方法

### 1. サーバーの起動

```bash
cd rust-server
cargo run --release
```

### 2. WebUIの起動

```bash
cd web-ui
npm run dev
```

ブラウザで `http://localhost:5173` を開く

### 3. Master EAの設定

MT4/MT5でチャートを開き、Master EAをアタッチ:

**パラメータ:**
- `ServerAddress`: `tcp://localhost:5555` (デフォルト)
- `AccountID`: マスターアカウントの識別子 (例: `MASTER_001`)
- `MagicFilter`: コピーするマジックナンバー (0=すべて)
- `ScanInterval`: スキャン間隔 (ミリ秒)

### 4. Slave EAの設定

別のMT4/MT5でチャートを開き、Slave EAをアタッチ:

**パラメータ:**
- `ServerAddress`: `tcp://localhost:5556` (デフォルト)
- `AccountID`: スレーブアカウントの識別子 (例: `SLAVE_001`)
- `Slippage`: 許容スリッページ (ポイント)
- `MaxRetries`: 注文リトライ回数
- `AllowNewOrders`: 新規注文を許可
- `AllowCloseOrders`: 決済を許可

### 5. WebUIでコピー設定を作成

1. WebUIで「+ New Setting」をクリック
2. 以下を入力:
   - Master Account: `MASTER_001` (Master EAのAccountIDと一致)
   - Slave Account: `SLAVE_001` (Slave EAのAccountIDと一致)
   - Lot Multiplier: ロット倍率 (例: `1.0` で同じロット)
   - Reverse Trade: 売買反転する場合はチェック
3. 「Create」をクリック

### 6. コピーの開始

- WebUIで作成した設定の「Enable」ボタンをクリック
- Master口座でトレードを行うと、自動的にSlave口座にコピーされます
- WebUIの「Recent Activity」でリアルタイムに状態を確認できます

## 高度な設定

### シンボル変換

ブローカーによってシンボル名が異なる場合に使用します。

**例: プリフィックス/サフィックスの変換**
- Master: `EURUSD.raw`
- Slave: `EURUSD`

設定でシンボルマッピングを追加:
```json
{
  "source_symbol": "EURUSD.raw",
  "target_symbol": "EURUSD"
}
```

### フィルター設定

特定の通貨ペアやマジックナンバーのみコピー:

```json
{
  "filters": {
    "allowed_symbols": ["EURUSD", "GBPUSD"],
    "blocked_symbols": null,
    "allowed_magic_numbers": [12345],
    "blocked_magic_numbers": null
  }
}
```

### ロット計算戦略

Rust側のコードで以下の戦略を実装済み:

1. **固定倍率**: `lot_multiplier` で指定
2. **残高比率**: 口座残高に応じて自動計算
3. **リスク比率**: リスク%とストップロスから計算

## トラブルシューティング

### EAが接続できない

1. Rustサーバーが起動しているか確認
2. MT4/MT5の「ツール」→「オプション」→「エキスパートアドバイザ」で「DLLの使用を許可する」がチェックされているか確認
3. `forex_copier_zmq.dll` が正しい場所に配置されているか確認
4. MT4/MT5のエキスパートログで DLL ロードエラーが出ていないか確認

### トレードがコピーされない

1. WebUIでコピー設定が「Active」になっているか確認
2. Master EAとSlave EAの`AccountID`とWebUIの設定が一致しているか確認
3. フィルター設定で対象トレードが除外されていないか確認
4. Slave EAの`AllowNewOrders`が有効になっているか確認

### WebUIが表示されない

1. Rustサーバーが起動しているか確認
2. ブラウザのコンソールでエラーを確認
3. ファイアウォールで8080ポートがブロックされていないか確認

## セキュリティ

- 本番環境では必ず適切なファイアウォール設定を行ってください
- WebUIには認証機能がないため、外部に公開しないでください
- 機密情報(API Key等)をログに出力しないよう注意してください

## ライセンス

MIT License

## サポート

問題が発生した場合は、GitHubのIssuesでお知らせください。

## 貢献

プルリクエストを歓迎します!

## ロードマップ

- [ ] WebUI認証機能
- [ ] 詳細なトレード履歴
- [ ] パフォーマンス分析
- [ ] Telegram通知
- [ ] クラウド版の提供
- [ ] スマホアプリ (iOS/Android)

---

**注意**: このソフトウェアは実験的なものです。リアル口座で使用する前に、必ずデモ口座で十分なテストを行ってください。
