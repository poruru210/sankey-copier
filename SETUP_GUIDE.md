# Forex Copier セットアップガイド

このガイドでは、Forex Copierを初めて使用する方向けに、詳細なセットアップ手順を説明します。

## 前提条件

以下がインストールされていることを確認してください:

- **Windows 10/11** (MT4/MT5用)
- **Rust**: https://www.rust-lang.org/tools/install
- **Node.js**: https://nodejs.org/ (LTS版推奨)
- **MetaTrader 4 または 5**: ブローカーからダウンロード

## ステップ1: ZeroMQのセットアップ

### ZeroMQライブラリのダウンロード

1. https://github.com/dingmaotu/mql-zmq/releases にアクセス
2. 最新リリースから `libzmq.dll` をダウンロード
3. ダウンロードしたDLLを以下の場所にコピー:

**MT4の場合:**
```
C:\Program Files\[Your Broker] MetaTrader 4\MQL4\Libraries\libzmq.dll
```

**MT5の場合:**
```
C:\Program Files\[Your Broker] MetaTrader 5\MQL5\Libraries\libzmq.dll
```

### DLL使用の許可

1. MT4/MT5を起動
2. 「ツール」→「オプション」を開く
3. 「エキスパートアドバイザ」タブを選択
4. 以下にチェックを入れる:
   - ✅ 自動売買を許可する
   - ✅ DLLの使用を許可する
   - ✅ WebRequestを許可するURLリスト (必要に応じて)
5. 「OK」をクリック

## ステップ2: Rustサーバーのセットアップ

### Rustのインストール確認

コマンドプロンプトまたはPowerShellで以下を実行:

```powershell
rustc --version
cargo --version
```

バージョン情報が表示されればOKです。

### プロジェクトのビルドと実行

```powershell
# プロジェクトフォルダに移動
cd D:\projects\test\forex-copier\rust-server

# 初回ビルド (時間がかかります)
cargo build --release

# サーバーの起動
cargo run --release
```

以下のようなログが表示されれば成功:
```
INFO forex_copier_server: Starting Forex Copier Server...
INFO forex_copier_server: Database initialized
INFO forex_copier_server: ZeroMQ receiver started on tcp://*:5555
INFO forex_copier_server: HTTP server listening on http://0.0.0.0:8080
```

### サーバーをバックグラウンドで実行

Windowsサービスとして実行したい場合は、NSSM (Non-Sucking Service Manager) を使用できます:

1. https://nssm.cc/download からNSSMをダウンロード
2. 管理者権限でコマンドプロンプトを開く
3. 以下を実行:

```cmd
nssm install ForexCopier "D:\projects\test\forex-copier\rust-server\target\release\forex-copier-server.exe"
nssm start ForexCopier
```

## ステップ3: WebUIのセットアップ

```powershell
# WebUIフォルダに移動
cd D:\projects\test\forex-copier\web-ui

# 依存関係のインストール
npm install

# 開発サーバーの起動
npm run dev
```

ブラウザで http://localhost:5173 にアクセスして動作確認

### 本番環境用にビルド

```powershell
npm run build
```

ビルドされたファイルは `dist` フォルダに出力されます。これをWebサーバー(nginx, Apache等)でホストできます。

## ステップ4: MT4/MT5 EAのインストール

### ファイルのコピー

**MT4の場合:**

1. `forex-copier/mql/Include/ZeroMQ/` フォルダ全体を以下にコピー:
   ```
   C:\Program Files\[Your Broker] MetaTrader 4\MQL4\Include\ZeroMQ\
   ```

2. Master EA をコピー:
   ```
   forex-copier/mql/MT4/Master/ForexCopierMaster.mq4
   → C:\Program Files\[Your Broker] MetaTrader 4\MQL4\Experts\
   ```

3. Slave EA をコピー:
   ```
   forex-copier/mql/MT4/Slave/ForexCopierSlave.mq4
   → C:\Program Files\[Your Broker] MetaTrader 4\MQL4\Experts\
   ```

**MT5の場合も同様に MQL5 フォルダ構造で配置**

### EAのコンパイル

1. MT4/MT5でMetaEditorを開く (F4キー)
2. Navigator から ForexCopierMaster を開く
3. コンパイルボタンをクリック (F7キー)
4. エラーログを確認:
   - `0 error(s), 0 warning(s)` であればOK
   - エラーがある場合は `libzmq.dll` のパスを確認

5. ForexCopierSlaveも同様にコンパイル

## ステップ5: 実際の運用例

### シナリオ: 1つのMaster口座から2つのSlave口座にコピー

#### Master口座の設定

1. MT4でEURUSDチャートを開く
2. ForexCopierMaster をチャートにドラッグ
3. パラメータを設定:
   - ServerAddress: `tcp://localhost:5555`
   - AccountID: `MASTER_001`
   - MagicFilter: `0` (すべてのトレードをコピー)
   - ScanInterval: `100`
4. 「OK」をクリック

#### Slave口座1の設定

1. 別のMT4でEURUSDチャートを開く
2. ForexCopierSlave をチャートにドラッグ
3. パラメータを設定:
   - ServerAddress: `tcp://localhost:5556`
   - AccountID: `SLAVE_001`
   - Slippage: `3`
   - MaxRetries: `3`
   - AllowNewOrders: `true`
   - AllowCloseOrders: `true`
4. 「OK」をクリック

#### Slave口座2の設定

Slave口座1と同じ手順で、`AccountID` を `SLAVE_002` に変更

#### WebUIで設定を作成

1. http://localhost:5173 にアクセス
2. 「+ New Setting」をクリック

**設定1: MASTER → SLAVE_001**
- Master Account: `MASTER_001`
- Slave Account: `SLAVE_001`
- Lot Multiplier: `1.0` (同じロット)
- Reverse Trade: チェックなし

**設定2: MASTER → SLAVE_002**
- Master Account: `MASTER_001`
- Slave Account: `SLAVE_002`
- Lot Multiplier: `0.5` (半分のロット)
- Reverse Trade: チェックなし

3. 両方の設定で「Enable」をクリック

### 動作確認

1. Master口座で成行注文を実行 (例: EURUSD Buy 0.1 lot)
2. WebUIの「Recent Activity」でメッセージを確認:
   ```
   trade_received:MASTER_001:EURUSD:0.1
   trade_copied:SLAVE_001:EURUSD:0.1:1
   trade_copied:SLAVE_002:EURUSD:0.05:2
   ```
3. 各Slave口座で注文が作成されたことを確認

## ステップ6: シンボル変換の設定

ブローカー間でシンボル名が異なる場合の設定例:

### 例: Master が "EURUSD.raw", Slave が "EURUSD"

WebUIで設定を編集し、Symbol Mappingsを追加する必要があります。
現在のWebUIでは簡易版のため、直接データベースを編集するか、APIを使用:

```bash
curl -X PUT http://localhost:8080/api/settings/1 \
  -H "Content-Type: application/json" \
  -d '{
    "id": 1,
    "enabled": true,
    "master_account": "MASTER_001",
    "slave_account": "SLAVE_001",
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

## ステップ7: スマホからの制御

### 同じネットワーク内からアクセス

1. サーバーのIPアドレスを確認:
   ```powershell
   ipconfig
   ```
   例: `192.168.1.100`

2. スマホのブラウザで以下にアクセス:
   ```
   http://192.168.1.100:5173
   ```

3. WebUIが表示され、設定の有効化/無効化が可能

### 外部からのアクセス (VPN推奨)

セキュリティのため、外部公開する場合はVPNの使用を強く推奨します:

1. **Tailscale** (推奨): https://tailscale.com/
   - 簡単にセキュアなVPNを構築
   - 無料プランで十分

2. **WireGuard**: https://www.wireguard.com/
   - より高度な設定が可能

## トラブルシューティング

### Q1: EAがサーバーに接続できない

**確認項目:**
- [ ] Rustサーバーが起動している
- [ ] MT4/MT5で「DLLの使用を許可する」がON
- [ ] `libzmq.dll` が正しい場所にある
- [ ] ファイアウォールでポート5555, 5556がブロックされていない

**ファイアウォールの確認:**
```powershell
# 管理者権限で実行
netsh advfirewall firewall add rule name="Forex Copier" dir=in action=allow protocol=TCP localport=5555,5556,8080
```

### Q2: WebUIに接続できない

**確認項目:**
- [ ] `npm run dev` が実行中
- [ ] ブラウザで http://localhost:5173 にアクセス
- [ ] Rustサーバーが起動している (API用)

### Q3: トレードがコピーされない

**デバッグ手順:**

1. Master EAのログを確認:
   - MT4/MT5の「エキスパート」タブ
   - "Sent Open signal for order #..." が表示されるはず

2. Rustサーバーのログを確認:
   - "Processing trade signal: ..." が表示されるはず

3. Slave EAのログを確認:
   - "Order opened successfully: ..." が表示されるはず

4. WebUIの設定を確認:
   - AccountIDが一致しているか
   - 設定がActiveになっているか
   - フィルターで除外されていないか

### Q4: ビルドエラーが出る

**Rustサーバーのビルドエラー:**
```powershell
# ZeroMQライブラリが見つからない場合
# vcpkg を使用してインストール
git clone https://github.com/Microsoft/vcpkg.git
cd vcpkg
.\bootstrap-vcpkg.bat
.\vcpkg install zeromq:x64-windows
```

**WebUIのビルドエラー:**
```powershell
# node_modulesを削除して再インストール
rm -r node_modules
rm package-lock.json
npm install
```

## パフォーマンスチューニング

### 低遅延化

1. **ScanIntervalを短縮** (Master EA):
   - デフォルト: 100ms
   - 推奨: 50ms (高頻度トレード用)

2. **Rustサーバーのスレッド数を調整**:
   ```rust
   // main.rs で tokio のスレッド数を設定
   #[tokio::main(worker_threads = 4)]
   ```

3. **ネットワーク遅延の削減**:
   - ローカル実行を推奨
   - VPSを使用する場合は、MT4/MT5と同じデータセンターを選択

## 次のステップ

- フィルター設定を試す
- 複数のMaster/Slave構成を試す
- Telegram通知の実装 (今後のバージョン)

## サポート

問題が解決しない場合は、以下の情報と共にIssueを作成してください:

- OS バージョン
- Rust バージョン
- MT4/MT5 バージョン
- エラーログ (サーバー、EA、WebUI)
- 設定内容 (JSON)

---

Happy Trading!
