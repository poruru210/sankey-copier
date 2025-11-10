# SANKEY Copier 運用・デプロイメント手順書

## 目次

1. [環境要件](#環境要件)
2. [インストール手順](#インストール手順)
3. [初回セットアップ](#初回セットアップ)
4. [設定ファイル](#設定ファイル)
5. [ビルド・デプロイ](#ビルドデプロイ)
6. [起動・停止手順](#起動停止手順)
7. [トラブルシューティング](#トラブルシューティング)
8. [メンテナンス](#メンテナンス)
9. [バックアップ・リストア](#バックアップリストア)

---

## 環境要件

### サーバー側 (Rust Server)

| 項目 | 要件 |
|------|------|
| **OS** | Windows 10/11, Linux, macOS |
| **CPU** | Intel Core i3 以上 (2コア以上推奨) |
| **メモリ** | 最小 512MB / 推奨 2GB以上 |
| **ストレージ** | 100MB以上の空き容量 |
| **Rust** | 1.70以上 |
| **ZeroMQ** | libzmq (自動インストール) |

### Web UI

| 項目 | 要件 |
|------|------|
| **Node.js** | 18.0以上 |
| **npm / pnpm / yarn** | 最新版推奨 |
| **ブラウザ** | Chrome/Firefox/Safari/Edge (最新版) |

### MT4/MT5 EA

| 項目 | 要件 |
|------|------|
| **MetaTrader 4** | Build 1280以上 |
| **MetaTrader 5** | Build 3450以上 |
| **Rust** | 1.70以上（DLLビルド用） |
| **ターゲット** | `i686-pc-windows-msvc` (32-bit)<br/>`x86_64-pc-windows-msvc` (64-bit) |

### ネットワーク

| 項目 | 要件 |
|------|------|
| **ポート** | 5555/5556/5557 (ZeroMQ)<br/>8080 (HTTP/WebSocket) |
| **ファイアウォール** | 上記ポートを開放 |
| **インターネット** | WebUIアクセス時のみ必要 |

---

## インストール手順

### 1. Rustサーバーのインストール

#### 1-1. Rustのインストール

**Windows**:

```powershell
# rustupインストール
Invoke-WebRequest -Uri https://win.rustup.rs -OutFile rustup-init.exe
.\rustup-init.exe
```

**Linux/macOS**:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

#### 1-2. プロジェクトのクローン

```bash
git clone https://github.com/your-repo/sankey-copier.git
cd sankey-copier
```

#### 1-3. 依存関係のインストール

```bash
cd rust-server
cargo build --release
```

**初回ビルド時間**: 5-10分（依存関係のダウンロード・コンパイル）

#### 1-4. ビルド確認

```bash
cargo test
```

**期待される結果**: すべてのテスト合格 (46 tests passed)

---

### 2. Web UIのインストール

#### 2-1. Node.jsのインストール

**Windows**:

- [Node.js公式サイト](https://nodejs.org/)からインストーラーをダウンロード
- LTS版（推奨）をインストール

**Linux (Ubuntu/Debian)**:

```bash
curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
sudo apt-get install -y nodejs
```

**macOS (Homebrew)**:

```bash
brew install node
```

#### 2-2. 依存関係のインストール

```bash
cd web-ui

# pnpm推奨（高速）
npm install -g pnpm
pnpm install

# または npm
npm install
```

#### 2-3. ビルド確認

```bash
pnpm build
# または
npm run build
```

---

### 3. MT4/MT5 EAのインストール

#### 3-1. ZeroMQ DLLのビルド

**32-bit版** (MT4, 一部MT5):

```bash
cd mql-zmq-dll

# 32-bitターゲット追加
rustup target add i686-pc-windows-msvc

# ビルド
cargo build --release --target i686-pc-windows-msvc
```

**64-bit版** (MT5):

```bash
# ビルド
cargo build --release
```

#### 3-2. DLLの配置

**MT4への配置**:

```bash
# 32-bit DLL
cp target/i686-pc-windows-msvc/release/sankey_copier_zmq.dll \
   "C:/Program Files (x86)/MetaTrader 4/MQL4/Libraries/"
```

**MT5への配置**:

```bash
# 64-bit DLL (64-bit版MT5の場合)
cp target/release/sankey_copier_zmq.dll \
   "C:/Program Files/MetaTrader 5/MQL5/Libraries/"

# 32-bit DLL (32-bit版MT5の場合)
cp target/i686-pc-windows-msvc/release/sankey_copier_zmq.dll \
   "C:/Program Files/MetaTrader 5/MQL5/Libraries/"
```

**MT5のバージョン確認方法**:

1. MT5を起動
2. 「ヘルプ」→「バージョン情報」を確認
3. インストールフォルダに `terminal64.exe` があれば64-bit版

#### 3-3. EAファイルの配置

**MT4の場合**:

```
mql/MT4/Master/SankeyCopierMaster.mq4
  → C:/Program Files (x86)/MetaTrader 4/MQL4/Experts/

mql/MT4/Slave/SankeyCopierSlave.mq4
  → C:/Program Files (x86)/MetaTrader 4/MQL4/Experts/
```

**MT5の場合**:

```
mql/MT5/Master/SankeyCopierMaster.mq5
  → C:/Program Files/MetaTrader 5/MQL5/Experts/

mql/MT5/Slave/SankeyCopierSlave.mq5
  → C:/Program Files/MetaTrader 5/MQL5/Experts/
```

#### 3-4. MT4/MT5でのコンパイル

1. **MetaEditorを開く** (F4キー)
2. **各EAファイルを開く**
3. **コンパイル** (F7キー)
4. **エラーがないことを確認**

```
0 error(s), 0 warning(s)
```

#### 3-5. DLL使用許可

**重要**: MT4/MT5でDLLの使用を許可する必要があります。

1. **MT4/MT5を起動**
2. **「ツール」→「オプション」**
3. **「エキスパートアドバイザ」タブ**
4. **「DLLの使用を許可する」にチェック**
5. **「OK」をクリック**

---

## 初回セットアップ

### 1. サーバー設定

#### 1-1. 設定ファイルの作成

```bash
cd rust-server
cp config.toml.example config.toml
# または、以下の内容で新規作成
```

**`config.toml`**:

```toml
# SANKEY Copier Server Configuration

[server]
# Server host and port
host = "0.0.0.0"
port = 8080

[database]
# SQLite database URL
url = "sqlite://sankey_copier.db?mode=rwc"

[zeromq]
# ZeroMQ port configuration
receiver_port = 5555      # Port for receiving from EAs
sender_port = 5556        # Port for trade distribution
config_sender_port = 5557 # Port for config distribution
timeout_seconds = 30      # Connection timeout
```

#### 1-2. データベース初期化

```bash
cargo run --release
```

**初回起動時の動作**:

1. `sankey_copier.db` が自動作成される
2. テーブルが自動的に作成される (`copy_settings`, `symbol_mappings`, `trade_filters`)

**ログ出力例**:

```
[INFO] Starting SANKEY Copier Server...
[INFO] Server Version: a748f63
[INFO] Loaded configuration from config.toml
[INFO] Server will listen on: 0.0.0.0:8080
[INFO] ZMQ Receiver: tcp://*:5555
[INFO] ZMQ Sender: tcp://*:5556
[INFO] ZMQ Config Sender: tcp://*:5557
[INFO] Database initialized
[INFO] Connection manager initialized with 30s timeout
[INFO] ZeroMQ receiver started on tcp://*:5555
[INFO] HTTP server listening on http://0.0.0.0:8080
```

---

### 2. Web UI設定

#### 2-1. 環境変数の設定（オプション）

```bash
cd web-ui

# .env.local ファイルを作成
cat > .env.local <<EOF
NEXT_PUBLIC_API_URL=http://localhost:8080
EOF
```

#### 2-2. 開発サーバー起動

```bash
pnpm dev
# または
npm run dev
```

**ログ出力例**:

```
▲ Next.js 16.0.1
- Local:        http://localhost:5173
- Network:      http://192.168.1.100:5173

✓ Ready in 2.5s
```

#### 2-3. ブラウザでアクセス

```
http://localhost:5173
```

---

### 3. Master EA設定

#### 3-1. チャートにEAをアタッチ

1. **MT4/MT5でチャートを開く** (任意の通貨ペア・時間足)
2. **ナビゲーター** から `SankeyCopierMaster` をドラッグ
3. **パラメータを設定**

#### 3-2. Master EAパラメータ

| パラメータ | デフォルト値 | 説明 |
|-----------|------------|------|
| `ServerAddress` | `tcp://localhost:5555` | サーバーアドレス |
| `AccountID` | `(自動生成)` | アカウントID（ブローカー名-口座番号） |
| `MagicFilter` | `0` | コピーするマジックナンバー（0=すべて） |
| `ScanInterval` | `100` | スキャン間隔（ミリ秒） |

#### 3-3. 動作確認

**ログ例** (MT4/MT5のエキスパートログ):

```
SankeyCopierMaster: Initializing...
SankeyCopierMaster: Account ID: FXGT-12345
SankeyCopierMaster: ZMQ Context created
SankeyCopierMaster: ZMQ Socket created
SankeyCopierMaster: Connected to tcp://localhost:5555
SankeyCopierMaster: Sent REGISTER message
SankeyCopierMaster: Initialization complete
```

---

### 4. Slave EA設定

#### 4-1. チャートにEAをアタッチ

1. **別のMT4/MT5でチャートを開く**
2. **ナビゲーター** から `SankeyCopierSlave` をドラッグ
3. **パラメータを設定**

#### 4-2. Slave EAパラメータ

| パラメータ | デフォルト値 | 説明 |
|-----------|------------|------|
| `TradeServerAddress` | `tcp://localhost:5556` | トレードシグナル受信アドレス |
| `ConfigServerAddress` | `tcp://localhost:5557` | 設定受信アドレス |
| `AccountID` | `(自動生成)` | アカウントID |
| `Slippage` | `3` | 許容スリッページ（ポイント） |
| `MaxRetries` | `3` | 注文リトライ回数 |
| `AllowNewOrders` | `true` | 新規注文を許可 |
| `AllowCloseOrders` | `true` | 決済を許可 |

#### 4-3. 動作確認

**ログ例**:

```
SankeyCopierSlave: Initializing...
SankeyCopierSlave: Account ID: XM-67890
SankeyCopierSlave: ZMQ Trade Socket connected to tcp://localhost:5556
SankeyCopierSlave: ZMQ Config Socket connected to tcp://localhost:5557
SankeyCopierSlave: Sent REGISTER message
SankeyCopierSlave: CONFIG received from server
SankeyCopierSlave: Master account: FXGT-12345
SankeyCopierSlave: Lot multiplier: 1.0
SankeyCopierSlave: Enabled: true
SankeyCopierSlave: Initialization complete
```

---

### 5. Web UIで設定作成

#### 5-1. 設定画面を開く

```
http://localhost:5173
```

#### 5-2. 新規設定作成

1. **「+ New Setting」ボタンをクリック**
2. **以下を入力**:
   - **Master Account**: `FXGT-12345` (Master EAのアカウントIDと一致)
   - **Slave Account**: `XM-67890` (Slave EAのアカウントIDと一致)
   - **Lot Multiplier**: `1.0` (同じロット数でコピー)
   - **Reverse Trade**: `false` (売買反転しない)
3. **「Create」をクリック**

#### 5-3. 動作確認

**Slave EA側のログ**:

```
SankeyCopierSlave: CONFIG received from server
SankeyCopierSlave: Master account: FXGT-12345
SankeyCopierSlave: Lot multiplier: 1.0
SankeyCopierSlave: Configuration updated
```

---

## 設定ファイル

### config.toml (Rust Server)

**場所**: `rust-server/config.toml`

```toml
[server]
# サーバーバインドアドレス
host = "0.0.0.0"  # すべてのインターフェースでリッスン
port = 8080       # HTTPポート

[database]
# SQLiteデータベースファイル
url = "sqlite://sankey_copier.db?mode=rwc"

[zeromq]
# ZeroMQポート設定
receiver_port = 5555       # EA → Server (PULL)
sender_port = 5556         # Server → Slave EA (PUB, トレード)
config_sender_port = 5557  # Server → Slave EA (PUB, 設定)
timeout_seconds = 30       # 接続タイムアウト（秒）
                           # サーバー側で10秒ごとにタイムアウトチェック実行
```

**設定変更後の再起動**:

```bash
# サーバーを停止 (Ctrl+C)
# 設定ファイルを編集
# サーバーを再起動
cargo run --release
```

---

### .env.local (Web UI)

**場所**: `web-ui/.env.local`

```bash
# API URL (プロダクション環境では変更)
NEXT_PUBLIC_API_URL=http://localhost:8080
```

**設定変更後の再ビルド**:

```bash
pnpm build
pnpm start
```

---

## ビルド・デプロイ

### 本番環境へのデプロイ

#### 1. Rustサーバーのビルド

```bash
cd rust-server
cargo build --release
```

**生成される実行ファイル**:

- Windows: `target/release/sankey-copier-server.exe`
- Linux/macOS: `target/release/sankey-copier-server`

**配置先**:

```bash
# 本番サーバーに配置
scp target/release/sankey-copier-server user@server:/opt/sankey-copier/
scp config.toml user@server:/opt/sankey-copier/
```

#### 2. Web UIのビルド

```bash
cd web-ui
pnpm build
```

**生成されるファイル**:

- `.next/` フォルダ（ビルド成果物）

**スタンドアロンモード** (推奨):

```bash
# package.jsonに追記
{
  "scripts": {
    "build": "next build",
    "start": "next start -p 5173 -H 0.0.0.0"
  }
}

# ビルド
pnpm build

# デプロイ
scp -r .next node_modules package.json user@server:/opt/sankey-copier/web-ui/
```

#### 3. Systemdサービス化 (Linux)

**`/etc/systemd/system/sankey-copier.service`**:

```ini
[Unit]
Description=SANKEY Copier Server
After=network.target

[Service]
Type=simple
User=sankey
WorkingDirectory=/opt/sankey-copier
ExecStart=/opt/sankey-copier/sankey-copier-server
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
```

**有効化・起動**:

```bash
sudo systemctl daemon-reload
sudo systemctl enable sankey-copier
sudo systemctl start sankey-copier
sudo systemctl status sankey-copier
```

---

## 起動・停止手順

### サーバーの起動

**開発環境**:

```bash
cd rust-server
cargo run --release
```

**本番環境 (Systemd)**:

```bash
sudo systemctl start sankey-copier
```

**本番環境 (手動)**:

```bash
cd /opt/sankey-copier
./sankey-copier-server &
```

---

### サーバーの停止

**開発環境**:

```
Ctrl + C
```

**本番環境 (Systemd)**:

```bash
sudo systemctl stop sankey-copier
```

**本番環境 (手動)**:

```bash
pkill sankey-copier-server
```

---

### Web UIの起動

**開発環境**:

```bash
cd web-ui
pnpm dev
```

**本番環境**:

```bash
cd /opt/sankey-copier/web-ui
pnpm start
# または
npm start
```

---

## トラブルシューティング

運用中に発生する可能性のある問題と解決方法については、専用のトラブルシューティングガイドを参照してください:

**📖 [トラブルシューティングガイド](./troubleshooting.md)**

主なトピック:
- EAの接続問題
- DLL読み込みエラー
- トレードがコピーされない
- Web UIの問題
- データベースエラー
- パフォーマンスの問題
- よくある質問（FAQ）

---

## メンテナンス

### ログ確認

#### Rustサーバーログ

**標準出力**:

```bash
cargo run --release 2>&1 | tee server.log
```

**Systemd (Linux)**:

```bash
sudo journalctl -u sankey-copier -f
```

#### EA ログ

**MT4/MT5エキスパートログ**:

```
MT4: ファイル → データフォルダを開く → MQL4\Logs\
MT5: ファイル → データフォルダを開く → MQL5\Logs\
```

---

### データベースメンテナンス

#### VACUUMコマンド

```bash
sqlite3 sankey_copier.db "VACUUM;"
```

**効果**: データベースファイルの最適化、サイズ縮小

#### 古いログの削除

```sql
-- 30日以上前のログを削除（ログテーブルがある場合）
DELETE FROM logs WHERE created_at < datetime('now', '-30 days');
```

---

## バックアップ・リストア

### バックアップ

#### データベースバックアップ

```bash
# 日付付きバックアップ
DATE=$(date +%Y%m%d_%H%M%S)
cp sankey_copier.db backups/sankey_copier_$DATE.db
```

#### 設定ファイルバックアップ

```bash
tar -czf backup_$DATE.tar.gz \
    rust-server/config.toml \
    sankey_copier.db
```

#### 自動バックアップスクリプト (Linux)

**`/opt/sankey-copier/backup.sh`**:

```bash
#!/bin/bash
DATE=$(date +%Y%m%d_%H%M%S)
BACKUP_DIR="/opt/sankey-copier/backups"
mkdir -p $BACKUP_DIR

# データベースバックアップ
cp /opt/sankey-copier/sankey_copier.db $BACKUP_DIR/sankey_copier_$DATE.db

# 7日以上古いバックアップを削除
find $BACKUP_DIR -name "sankey_copier_*.db" -mtime +7 -delete
```

**Cron設定**:

```bash
# 毎日午前3時にバックアップ
0 3 * * * /opt/sankey-copier/backup.sh
```

---

### リストア

#### データベースリストア

```bash
# サーバー停止
sudo systemctl stop sankey-copier

# バックアップから復元
cp backups/sankey_copier_20251110_030000.db sankey_copier.db

# サーバー起動
sudo systemctl start sankey-copier
```

---

## セキュリティ

### 推奨事項

1. ✅ **外部公開しない**: ローカルネットワーク内のみで使用
2. ✅ **ファイアウォール設定**: 必要なポートのみ開放
3. ✅ **データベース暗号化**: SQLCipher等の使用を検討
4. ✅ **定期バックアップ**: 毎日自動バックアップを設定
5. ✅ **ログ監視**: 異常なアクティビティを検知

---

## まとめ

このドキュメントでは、SANKEY Copierの運用に必要なすべての手順を説明しました。

**重要なポイント**:

1. ✅ **環境要件を満たす**: Rust, Node.js, MT4/MT5
2. ✅ **DLL使用許可**: MT4/MT5の設定で有効化
3. ✅ **設定ファイル**: `config.toml` を正しく設定
4. ✅ **アカウントID一致**: Master/Slave EAとWeb UIの設定を一致させる
5. ✅ **ログ確認**: エラー時はログを確認してトラブルシューティング
6. ✅ **定期バックアップ**: データベースを定期的にバックアップ

**サポート**:

- 問題が発生した場合: [GitHub Issues](https://github.com/your-repo/sankey-copier/issues)
- ドキュメント: `docs/` フォルダ内の各種仕様書

これで、SANKEY Copierを安定して運用できます！
