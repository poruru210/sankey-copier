# SANKEY Copier セットアップガイド

このガイドでは、SANKEY Copierを初めて使用する方向けに、詳細なセットアップ手順を説明します。

## 目次

1. [前提条件](#前提条件)
2. [環境準備](#環境準備)
3. [Rustサーバーのセットアップ](#rustサーバーのセットアップ)
4. [WebUIのセットアップ](#webuiのセットアップ)
5. [MT4/MT5 EAのインストール](#mt4mt5-eaのインストール)
6. [初回起動と動作確認](#初回起動と動作確認)
7. [実用例: 1 Master → 2 Slaveの設定](#実用例-1-master--2-slaveの設定)
8. [高度な設定](#高度な設定)
9. [次のステップ](#次のステップ)

---

## 前提条件

以下がインストールされていることを確認してください:

### 必須ソフトウェア

- **Windows 10/11** (MT4/MT5用)
- **Rust 1.70以上**: https://www.rust-lang.org/tools/install
- **Node.js 18以上**: https://nodejs.org/ (LTS版推奨)
- **mise**: https://mise.jdx.dev/ （`.mise.toml`でpnpm 10.20.0を固定管理）
- **MetaTrader 4 または 5**: ブローカーからダウンロード

### 確認コマンド

```powershell
# Rustのバージョン確認
rustc --version
cargo --version

# Node.jsのバージョン確認
node --version
npm --version
```

すべてでバージョン情報が表示されればOKです。

---

## 環境準備

### ZeroMQ DLLのビルド

このプロジェクトでは、RustでビルドしたカスタムZeroMQ DLL (`sankey_copier_zmq.dll`) を使用します。

#### 1. MT4/MT5のビット数を確認

**MT5の場合:**
- MT5を起動
- 「ヘルプ」→「バージョン情報」を確認
- 「64-bit」と表示されていれば64-bit版

**MT4の場合:**
- 通常は32-bit版

#### 2. DLLのビルド

**32-bit版のビルド（MT4用、一部のMT5）:**

```powershell
cd mt-bridge

# 32-bitターゲットの追加
rustup target add i686-pc-windows-msvc

# ビルド
cargo build --release --target i686-pc-windows-msvc

# ビルド完了確認
dir target\i686-pc-windows-msvc\release\sankey_copier_zmq.dll
```

**64-bit版のビルド（64-bit MT5用）:**

```powershell
cd mt-bridge

# ビルド
cargo build --release

# ビルド完了確認
dir target\release\sankey_copier_zmq.dll
```

#### 3. DLLの配置

**MT4への配置:**

```powershell
# プロジェクトのDLLをコピー
copy target\i686-pc-windows-msvc\release\sankey_copier_zmq.dll ..\mql\MT4\Libraries\

# MT4インストールフォルダにコピー
copy ..\mql\MT4\Libraries\sankey_copier_zmq.dll "C:\Program Files (x86)\[Broker] MetaTrader 4\MQL4\Libraries\"
```

**MT5への配置（32-bit版の場合）:**

```powershell
copy target\i686-pc-windows-msvc\release\sankey_copier_zmq.dll ..\mql\MT5\Libraries\
copy ..\mql\MT5\Libraries\sankey_copier_zmq.dll "C:\Program Files\[Broker] MetaTrader 5\MQL5\Libraries\"
```

**MT5への配置（64-bit版の場合）:**

```powershell
copy target\release\sankey_copier_zmq.dll ..\mql\MT5\Libraries\
copy ..\mql\MT5\Libraries\sankey_copier_zmq.dll "C:\Program Files\[Broker] MetaTrader 5\MQL5\Libraries\"
```

### MT4/MT5の設定

#### DLL使用の許可

1. MT4/MT5を起動
2. 「ツール」→「オプション」を開く
3. 「エキスパートアドバイザ」タブを選択
4. 以下にチェックを入れる:
   - ✅ **自動売買を許可する**
   - ✅ **DLLの使用を許可する**
   - ✅ WebRequestを許可するURLリスト（必要に応じて）
5. 「OK」をクリック
6. **MT4/MT5を再起動**（重要）

---

## Rustサーバーのセットアップ

### 初回ビルド

```powershell
# プロジェクトフォルダに移動
cd relay-server

# 初回ビルド（時間がかかります: 5-10分程度）
cargo build --release
```

### 設定ファイルの確認

`relay-server/config.toml` を確認:

```toml
[database]
path = "copier.db"

[zeromq]
receiver_port = 5555        # Master EAからの受信ポート
sender_port = 5556          # Slave EAへの送信ポート
config_sender_port = 5557   # 設定配信ポート
timeout_seconds = 30        # ハートビートタイムアウト

[server]
host = "0.0.0.0"
port = 8080                 # WebUI API用ポート
```

デフォルト設定で問題なければ、そのまま使用できます。

### サーバーの起動

```powershell
# サーバーの起動
cargo run --release
```

**起動成功のログ例:**

```
INFO sankey_copier_server: Starting SANKEY Copier Server...
INFO sankey_copier_server: Database initialized at copier.db
INFO sankey_copier_server: Connection manager initialized
INFO sankey_copier_server: ZeroMQ receiver started on tcp://*:5555
INFO sankey_copier_server: ZeroMQ sender started on tcp://*:5556
INFO sankey_copier_server: Config sender started on tcp://*:5557
INFO sankey_copier_server: Loaded 0 copy settings
INFO sankey_copier_server: HTTP server listening on http://0.0.0.0:8080
```

### Windowsサービスとして実行（オプション）

バックグラウンドで常時実行したい場合は、NSSM (Non-Sucking Service Manager) を使用:

1. https://nssm.cc/download からダウンロード
2. 管理者権限でコマンドプロンプトを開く
3. 以下を実行:

```cmd
# インストール（パスは環境に合わせて変更）
nssm install SankeyCopier "D:\projects\test\forex-copier\relay-server\target\release\sankey-copier-server.exe"
nssm set SankeyCopier AppDirectory "D:\projects\test\forex-copier\relay-server"

# 起動
nssm start SankeyCopier

# 状態確認
nssm status SankeyCopier
```

詳細は [operations.md](./operations.md#起動停止手順) を参照してください。

---

## WebUIのセットアップ

### 依存関係のインストール

```powershell
# ルートでmiseを実行（pnpm 10.20.0を取得）
mise install

# WebUIフォルダに移動
cd web-ui

# 依存関係のインストール
pnpm install
```

※`npm install`でも動作しますが、本番と同じ依存ツリーを再現するためpnpm 10.20.0の使用を推奨しています。

### 開発サーバーの起動

```powershell
# 開発サーバーの起動
pnpm dev
```

**起動成功のログ例:**

```
VITE v5.x.x  ready in 500 ms

➜  Local:   http://localhost:5173/
➜  Network: use --host to expose
➜  press h + enter to show help
```

ブラウザで http://localhost:5173 にアクセスして動作確認してください。

### 本番環境用ビルド（オプション）

```powershell
# 本番用ビルド
pnpm build

# ビルド結果の確認
dir .next

# 本番サーバーの起動
pnpm start
```

ビルドされたアプリケーションは `.next` フォルダに出力されます。

---

## MT4/MT5 EAのインストール

### Includeファイルのコピー

EAが使用する共通ライブラリをコピーします。

**MT4の場合:**

```powershell
# プロジェクトフォルダから実行
xcopy mql\Include\SankeyCopier "C:\Program Files (x86)\[Broker] MetaTrader 4\MQL4\Include\SankeyCopier\" /E /I /Y
```

**MT5の場合:**

```powershell
xcopy mql\Include\SankeyCopier "C:\Program Files\[Broker] MetaTrader 5\MQL5\Include\SankeyCopier\" /E /I /Y
```

### EAファイルのコピー

**MT4 Master EA:**

```powershell
copy mql\MT4\SankeyCopierMaster.mq4 "C:\Program Files (x86)\[Broker] MetaTrader 4\MQL4\Experts\"
```

**MT4 Slave EA:**

```powershell
copy mql\MT4\SankeyCopierSlave.mq4 "C:\Program Files (x86)\[Broker] MetaTrader 4\MQL4\Experts\"
```

**MT5 Master EA:**

```powershell
copy mql\MT5\SankeyCopierMaster.mq5 "C:\Program Files\[Broker] MetaTrader 5\MQL5\Experts\"
```

**MT5 Slave EA:**

```powershell
copy mql\MT5\SankeyCopierSlave.mq5 "C:\Program Files\[Broker] MetaTrader 5\MQL5\Experts\"
```

### EAのコンパイル

1. MT4/MT5でMetaEditorを開く（F4キー）
2. ナビゲーターから `SankeyCopierMaster.mq4` または `.mq5` を開く
3. コンパイルボタンをクリック（F7キー）
4. コンパイル結果を確認:
   - ✅ `0 error(s), 0 warning(s)` であればOK
   - ❌ エラーがある場合は [troubleshooting.md](./troubleshooting.md#コンパイルエラーが発生する) を参照

5. `SankeyCopierSlave` も同様にコンパイル

---

## 初回起動と動作確認

### 1. Rustサーバーの起動確認

```powershell
cd relay-server
cargo run --release
```

ログに以下が表示されていることを確認:
```
INFO sankey_copier_server: HTTP server listening on http://0.0.0.0:8080
```

### 2. WebUIの起動確認

別のターミナルを開いて:

```powershell
cd web-ui
pnpm dev
```

ブラウザで http://localhost:5173 にアクセスして画面が表示されることを確認。

### 3. Master EAの起動

#### MT4の場合:

1. MT4でチャートを開く（どの通貨ペアでも可、例: EURUSD）
2. ナビゲーターから「エキスパートアドバイザ」→「SankeyCopierMaster」を選択
3. チャートにドラッグ&ドロップ
4. パラメータ設定画面が表示されるので設定:

```
[全般タブ]
✅ 自動売買を許可する
✅ DLLの使用を許可する

[パラメータタブ]
ServerAddress: tcp://localhost:5555  ← デフォルトのまま
MagicFilter: 0                       ← すべてのトレードをコピー
ScanInterval: 100                    ← デフォルトのまま
```

5. 「OK」をクリック

#### MT5の場合:

上記MT4と同じ手順で設定します。

#### 起動確認:

MT4/MT5のエキスパートタブに以下のログが表示されればOK:

```
=== SankeyCopier Master EA (MT4/MT5) Starting ===
Auto-generated AccountID: [Broker]_[AccountNumber]
Connected to server successfully
Sent registration message
=== SankeyCopier Master EA (MT4/MT5) Initialized ===
```

WebUIの「EA Connections」セクションにMasterアカウントが表示され、ステータスが「Online」（緑色）になることを確認。

### 4. Slave EAの起動

#### MT4/MT5の場合:

1. **別のMT4/MT5インスタンス**を起動（同じブローカーでも別のブローカーでも可）
2. チャートを開く
3. 「SankeyCopierSlave」をチャートにドラッグ&ドロップ
4. パラメータ設定:

```
[全般タブ]
✅ 自動売買を許可する
✅ DLLの使用を許可する

[パラメータタブ]
TradeServerAddress: tcp://localhost:5556  ← デフォルトのまま
ConfigServerAddress: tcp://localhost:5557 ← デフォルトのまま
Slippage: 3                               ← デフォルトのまま
MaxRetries: 3                             ← デフォルトのまま
AllowNewOrders: true                      ← 新規注文を許可
AllowCloseOrders: true                    ← 決済を許可
MaxSignalDelayMs: 5000                    ← シグナル遅延の許容値
UsePendingOrderForDelayed: false          ← デフォルトのまま
```

5. 「OK」をクリック

#### 起動確認:

エキスパートタブに以下のログが表示されればOK:

```
=== SankeyCopier Slave EA (MT4/MT5) Starting ===
Auto-generated AccountID: [Broker]_[AccountNumber]
Connected to trade channel: tcp://localhost:5556
Connected to config channel: tcp://localhost:5557
Sent registration message
=== SankeyCopier Slave EA Initialized ===
```

WebUIの「EA Connections」セクションにSlaveアカウントも表示され、両方「Online」になることを確認。

### 5. コピー設定の作成

1. WebUIで「**+ New Setting**」ボタンをクリック
2. フォームに入力:
   - **Master Account**: プルダウンから Master EAのAccountIDを選択
   - **Slave Account**: プルダウンから Slave EAのAccountIDを選択
   - **Lot Multiplier**: `1.0`（Masterと同じロット）
   - **Reverse Trade**: チェックなし
3. 「**Create**」ボタンをクリック
4. 作成された設定の「**Enable**」ボタンをクリックして有効化

設定のステータスが「**Active**」（緑色）になることを確認。

### 6. 動作テスト

#### トレードの実行:

Master口座で手動で小ロットのトレードを実行:

```
例: EURUSD Buy 0.01 lot（成行注文）
```

#### 確認項目:

1. **Master EAログ:**
   ```
   New order detected: #12345 EURUSD 0.01 lots
   Sent Open signal for order #12345
   ```

2. **Rustサーバーログ:**
   ```
   INFO: Received trade signal: Open EURUSD from [Master AccountID]
   INFO: Broadcasting trade signal to [Slave AccountID]
   ```

3. **Slave EAログ:**
   ```
   Received MessagePack trade signal for topic '...'
   Order opened successfully: slave #67890 from master #12345
   ```

4. **WebUI「Recent Activity」:**
   - リアルタイムでトレードシグナルが表示される

5. **Slave口座のターミナル:**
   - 同じEURUSD Buy 0.01 lotの注文が作成される

---

## 実用例: 1 Master → 2 Slaveの設定

### シナリオ

1つのMaster口座から2つのSlave口座に異なるロット倍率でコピーする構成を作成します。

### 構成

```
[Master口座]
├─ Broker A - MT4
├─ Account: 12345
└─ Balance: $10,000

[Slave口座1]
├─ Broker B - MT5
├─ Account: 67890
├─ Balance: $10,000
└─ Lot Multiplier: 1.0（同じロット）

[Slave口座2]
├─ Broker C - MT4
├─ Account: 54321
├─ Balance: $5,000
└─ Lot Multiplier: 0.5（半分のロット）
```

### 設定手順

#### 1. Master EAの設定

Broker A - MT4 で:

```
EA: SankeyCopierMaster
ServerAddress: tcp://localhost:5555
MagicFilter: 0
```

起動後、AccountIDを確認（例: `BrokerA_12345`）

#### 2. Slave EA 1の設定

Broker B - MT5 で:

```
EA: SankeyCopierSlave
TradeServerAddress: tcp://localhost:5556
ConfigServerAddress: tcp://localhost:5557
AllowNewOrders: true
AllowCloseOrders: true
```

起動後、AccountIDを確認（例: `BrokerB_67890`）

#### 3. Slave EA 2の設定

Broker C - MT4 で:

```
EA: SankeyCopierSlave
TradeServerAddress: tcp://localhost:5556
ConfigServerAddress: tcp://localhost:5557
AllowNewOrders: true
AllowCloseOrders: true
```

起動後、AccountIDを確認（例: `BrokerC_54321`）

#### 4. WebUIで設定を作成

**設定1: Master → Slave1（同じロット）**

```json
{
  "master_account": "BrokerA_12345",
  "slave_account": "BrokerB_67890",
  "lot_multiplier": 1.0,
  "reverse_trade": false,
  "symbol_mappings": [],
  "filters": {
    "allowed_symbols": null,
    "blocked_symbols": null,
    "allowed_magic_numbers": null,
    "blocked_magic_numbers": null
  }
}
```

**設定2: Master → Slave2（半分のロット）**

```json
{
  "master_account": "BrokerA_12345",
  "slave_account": "BrokerC_54321",
  "lot_multiplier": 0.5,
  "reverse_trade": false,
  "symbol_mappings": [],
  "filters": {
    "allowed_symbols": null,
    "blocked_symbols": null,
    "allowed_magic_numbers": null,
    "blocked_magic_numbers": null
  }
}
```

両方の設定で「**Enable**」をクリック。

### 動作確認

Master口座で以下のトレードを実行:

```
EURUSD Buy 0.1 lot @ 1.10000
```

期待される結果:

| 口座 | アクション | ロット | 口座残高比率 |
|------|----------|--------|------------|
| Master (Broker A) | Buy | 0.10 | 100% |
| Slave1 (Broker B) | Buy | 0.10 | 100%（lot_multiplier: 1.0） |
| Slave2 (Broker C) | Buy | 0.05 | 50%（lot_multiplier: 0.5） |

### トラブルシューティング

期待通りにコピーされない場合は [troubleshooting.md](./troubleshooting.md#トレードがコピーされない) を参照してください。

---

## 高度な設定

### シンボル名の変換

ブローカー間でシンボル名が異なる場合の設定例。

#### 例: Masterが "EURUSD.raw", Slaveが "EURUSD"

WebUIで設定を編集（現在は直接編集が必要）、またはAPIを使用:

```bash
curl -X PUT http://localhost:8080/api/settings/1 \
  -H "Content-Type: application/json" \
  -d '{
    "id": 1,
    "enabled": true,
    "master_account": "BrokerA_12345",
    "slave_account": "BrokerB_67890",
    "lot_multiplier": 1.0,
    "reverse_trade": false,
    "symbol_mappings": [
      {
        "source_symbol": "EURUSD.raw",
        "target_symbol": "EURUSD"
      },
      {
        "source_symbol": "GBPUSD.raw",
        "target_symbol": "GBPUSD"
      }
    ],
    "filters": {
      "allowed_symbols": null,
      "blocked_symbols": null,
      "allowed_magic_numbers": null,
      "blocked_magic_numbers": null
    }
  }'
```

### トレードフィルター

特定の通貨ペアやマジックナンバーのみコピーする設定。

#### 例: EUR系とGBP系の通貨ペアのみコピー

```json
{
  "filters": {
    "allowed_symbols": ["EURUSD", "EURJPY", "EURGBP", "GBPUSD", "GBPJPY"],
    "blocked_symbols": null,
    "allowed_magic_numbers": null,
    "blocked_magic_numbers": null
  }
}
```

#### 例: マジックナンバー12345のトレードのみコピー

```json
{
  "filters": {
    "allowed_symbols": null,
    "blocked_symbols": null,
    "allowed_magic_numbers": [12345],
    "blocked_magic_numbers": null
  }
}
```

### 売買反転

Masterの売買を反転してコピーする設定。

```json
{
  "reverse_trade": true
}
```

この設定では:
- Master: Buy → Slave: Sell
- Master: Sell → Slave: Buy

### スマートフォンからのアクセス

#### 同じネットワーク内からアクセス

1. サーバーのIPアドレスを確認:
   ```powershell
   ipconfig
   ```
   例: `192.168.1.100`

2. スマホのブラウザで以下にアクセス:
   ```
   http://192.168.1.100:5173
   ```

#### 外部ネットワークからアクセス（VPN推奨）

セキュリティのため、VPNの使用を強く推奨します:

**推奨VPNソリューション:**

1. **Tailscale**（推奨）: https://tailscale.com/
   - 簡単にセキュアなVPNを構築
   - 無料プランで十分
   - Windows、iOS、Androidアプリあり

2. **WireGuard**: https://www.wireguard.com/
   - より高度な設定が可能
   - オープンソース

**Tailscaleのセットアップ例:**

1. サーバーPCとスマホにTailscaleアプリをインストール
2. 同じアカウントでログイン
3. スマホからサーバーのTailscale IPアドレスでアクセス:
   ```
   http://100.x.x.x:5173
   ```

---

## 次のステップ

### 基本的な運用

セットアップが完了したら、以下を試してみてください:

1. **デモ口座で十分にテスト**
   - 最低1週間はデモ口座で動作確認
   - 様々な相場状況（トレンド、レンジ、急変動）で検証

2. **小ロットでリアル口座テスト**
   - 最小ロット（0.01 lot）から開始
   - 問題がなければ徐々にロットを増やす

3. **ログの定期確認**
   - Rustサーバーログを毎日確認
   - MT4/MT5ログでエラーがないか確認

### 本番環境へのデプロイ

本番環境（VPS等）へのデプロイ方法は [operations.md](./operations.md) を参照してください。

以下のトピックが含まれています:

- Systemdサービス化（Linux）
- NSSMサービス化（Windows）
- 自動バックアップの設定
- セキュリティ設定
- パフォーマンスチューニング

### さらに学ぶ

- **アーキテクチャ**: [architecture.md](./architecture.md) - システムの内部構造を理解
- **API仕様**: [api-specification.md](./api-specification.md) - REST APIやZeroMQプロトコルの詳細
- **データモデル**: [data-model.md](./data-model.md) - データベーススキーマとデータ構造

---

## サポート

問題が発生した場合:

1. **トラブルシューティングガイドを確認**: [troubleshooting.md](./troubleshooting.md)
2. **GitHubで既存のIssueを検索**: https://github.com/[your-repo]/issues
3. **新しいIssueを作成**: 以下の情報を含めてください
   - OS バージョン
   - Rust、Node.js、MT4/MT5 のバージョン
   - エラーログ
   - 再現手順

---

**Happy Trading! 🚀**
