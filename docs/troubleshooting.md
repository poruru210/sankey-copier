# SANKEY Copier トラブルシューティングガイド

このドキュメントでは、SANKEY Copierの使用中に発生する可能性のある問題と解決方法をまとめています。

## 目次

1. [セットアップ時の問題](#セットアップ時の問題)
2. [接続の問題](#接続の問題)
3. [トレードコピーの問題](#トレードコピーの問題)
4. [パフォーマンスの問題](#パフォーマンスの問題)
5. [ビルド・コンパイルの問題](#ビルドコンパイルの問題)
6. [よくある質問（FAQ）](#よくある質問faq)

---

## セットアップ時の問題

### DLLファイルが見つからない

**症状:**
- MT4/MT5のエキスパートログに「'sankey_copier_zmq.dll' failed to load」などのエラーが表示される
- EAがロードされない

**原因:**
1. DLLファイルが正しい場所にコピーされていない
2. DLLのビット数（32-bit/64-bit）がMT4/MT5と一致していない
3. DLLの使用が許可されていない

**解決方法:**

1. **DLLファイルの配置を確認:**
   ```
   MT4: C:\Program Files (x86)\[Broker] MetaTrader 4\MQL4\Libraries\sankey_copier_zmq.dll
   MT5: C:\Program Files\[Broker] MetaTrader 5\MQL5\Libraries\sankey_copier_zmq.dll
   ```

2. **MT4/MT5のビット数を確認:**
   - MT5の場合: 「ヘルプ」→「バージョン情報」で確認
   - MT4は通常32-bit、MT5は32-bit/64-bit両方存在

3. **正しいビット数のDLLをビルド:**
   ```bash
   # 32-bit版（MT4用、一部のMT5）
   cd mt-bridge
   rustup target add i686-pc-windows-msvc
   cargo build --release --target i686-pc-windows-msvc

   # 64-bit版（MT5用）
   cargo build --release
   ```

4. **DLL使用を許可:**
   - MT4/MT5の「ツール」→「オプション」
   - 「エキスパートアドバイザ」タブ
   - ✅「DLLの使用を許可する」にチェック
   - MT4/MT5を再起動

**確認コマンド:**
```powershell
# DLLファイルの存在確認
dir "C:\Program Files\[Broker] MetaTrader 5\MQL5\Libraries\sankey_copier_zmq.dll"
```

### コンパイルエラーが発生する

**症状:**
- MetaEditorでコンパイル時にエラーが表示される
- 「'Zmq' - file not found」などのエラー

**原因:**
1. mql-zmqのIncludeファイルがコピーされていない
2. MT5 Build 5120以降で古いmql-zmqを使用している

**解決方法:**

1. **Includeファイルのコピーを確認:**
   ```
   mql/Include/SankeyCopier/ → MT4/MT5のMQL4/MQL5/Include/SankeyCopier/
   ```

2. **MT5 Build 5120以降の場合:**
   - このプロジェクトでは既にビルド対応済みのコードを使用しています
   - オリジナルのmql-zmqではコンパイルエラーが発生する既知の問題があります

3. **MetaEditorのキャッシュをクリア:**
   - MetaEditorを閉じる
   - `%APPDATA%\MetaQuotes\Terminal\[TerminalID]\MQL4\` または `MQL5\` のキャッシュフォルダを削除
   - MetaEditorを再起動してコンパイル

**確認コマンド:**
```powershell
# Includeファイルの存在確認
dir "C:\Program Files\[Broker] MetaTrader 5\MQL5\Include\SankeyCopier\"
```

---

## 接続の問題

### EAがRustサーバーに接続できない

**症状:**
- MT4/MT5のエキスパートログに「Failed to connect to server」と表示される
- WebUIにEAのアカウントIDが表示されない

**原因:**
1. Rustサーバーが起動していない
2. ファイアウォールがポートをブロックしている
3. ServerAddressパラメータが間違っている
4. DLLの使用が許可されていない

**解決方法:**

1. **Rustサーバーの起動を確認:**
   ```bash
   cd relay-server
   cargo run --release
   ```

   以下のログが表示されるはず:
   ```
   INFO sankey_copier_server: ZeroMQ receiver started on tcp://*:5555
   INFO sankey_copier_server: HTTP server listening on http://0.0.0.0:8080
   ```

2. **ファイアウォール設定を確認:**
   ```powershell
   # 管理者権限で実行
   netsh advfirewall firewall add rule name="SANKEY Copier" dir=in action=allow protocol=TCP localport=5555,5556,8080
   ```

3. **EA パラメータを確認:**
   - Master EA: `ServerAddress = tcp://localhost:5555`
   - Slave EA: `TradeServerAddress = tcp://localhost:5556`

4. **DLL使用許可を確認:**
   - MT4/MT5の「ツール」→「オプション」→「エキスパートアドバイザ」
   - ✅「自動売買を許可する」
   - ✅「DLLの使用を許可する」

5. **ネットワーク確認:**
   ```powershell
   # ポート5555が開いているか確認
   netstat -an | findstr "5555"
   ```

**確認手順:**
1. Rustサーバーログで「Connection from EA: [AccountID]」が表示されるか確認
2. MT4/MT5ログで「Connected to server successfully」が表示されるか確認
3. WebUIの「EA Connections」セクションでステータスがOnlineか確認

### WebUIが表示されない・アクセスできない

**症状:**
- ブラウザで `http://localhost:5173` にアクセスできない
- 「このサイトにアクセスできません」と表示される

**原因:**
1. WebUI開発サーバーが起動していない
2. Rustサーバー（APIバックエンド）が起動していない
3. ポートが別のプロセスで使用されている
4. ファイアウォールがブロックしている

**解決方法:**

1. **WebUI開発サーバーの起動:**
   ```bash
   cd web-ui
   npm run dev
   ```

   以下のログが表示されるはず:
   ```
   VITE v5.x.x  ready in XXX ms
   ➜  Local:   http://localhost:5173/
   ```

2. **Rustサーバーの起動確認:**
   ```bash
   cd relay-server
   cargo run --release
   ```

3. **ポート衝突の確認:**
   ```powershell
   # ポート5173が使用されているか確認
   netstat -ano | findstr ":5173"
   ```

   別のプロセスが使用している場合は、そのプロセスを終了するか、WebUIのポートを変更:
   ```bash
   # package.jsonのdev scriptを編集
   "dev": "vite --port 5174"
   ```

4. **ブラウザのキャッシュをクリア:**
   - Ctrl + Shift + Delete でキャッシュを削除
   - シークレットモードで再度アクセス

5. **APIエンドポイントの確認:**
   ```bash
   # Rustサーバーのヘルスチェック
   curl http://localhost:8080/api/settings
   ```

**確認コマンド:**
```powershell
# すべての必要なポートが開いているか確認
netstat -an | findstr "5173 5555 5556 8080"
```

### ハートビートタイムアウトが発生する

**症状:**
- WebUIでEAのステータスが「Online」→「Offline」に頻繁に切り替わる
- Rustサーバーログに「Heartbeat timeout for [AccountID]」と表示される

**原因:**
1. ネットワーク遅延が大きい
2. MT4/MT5が固まっている・重い処理を実行中
3. サーバー側のタイムアウト設定が短すぎる

**解決方法:**

1. **タイムアウト設定の確認・調整:**

   `relay-server/config.toml` を編集:
   ```toml
   [zeromq]
   timeout_seconds = 30  # デフォルト30秒、必要に応じて60秒などに延長
   ```

2. **MT4/MT5のパフォーマンス確認:**
   - 他のEAやインディケータを停止
   - チャート数を減らす
   - PCのCPU/メモリ使用率を確認

3. **ネットワーク遅延の確認:**
   ```powershell
   # ローカルホストへのping
   ping localhost
   ```

   VPSを使用している場合:
   ```powershell
   # VPSへのping
   ping [VPS IP Address]
   ```

4. **ハートビート送信間隔の確認:**
   - EA側: `HEARTBEAT_INTERVAL_SECONDS = 30` (mql/Include/SankeyCopier/SankeyCopierCommon.mqh)
   - サーバー側: `timeout_seconds = 30` (relay-server/config.toml)
   - サーバー側のタイムアウトはEA側の送信間隔より長く設定する

---

## トレードコピーの問題

### トレードがコピーされない

**症状:**
- Master口座でトレードを実行してもSlave口座にコピーされない
- WebUIの「Recent Activity」にトレードシグナルが表示されない

**原因:**
1. コピー設定が無効になっている
2. AccountIDが一致していない
3. フィルター設定で除外されている
4. Slave EAで注文が許可されていない

**解決方法:**

**ステップ1: WebUIの設定確認**

1. WebUIで該当するコピー設定が「Enabled」になっているか確認
2. Master AccountとSlave AccountのIDが正しいか確認
3. フィルター設定を確認:
   - `allowed_symbols`: 指定されている場合、そのシンボルのみコピーされる
   - `blocked_symbols`: 指定されたシンボルは除外される
   - `allowed_magic_numbers`: 指定されている場合、そのマジックナンバーのみコピーされる

**ステップ2: EAログの確認**

1. **Master EAのログ（MT4/MT5のエキスパートタブ）:**
   ```
   期待されるログ:
   - "New order detected: #12345 EURUSD 0.1 lots"
   - "Sent Open signal for order #12345"

   エラー例:
   - "Failed to send message" → ZeroMQ接続エラー
   ```

2. **Rustサーバーのログ:**
   ```
   期待されるログ:
   - "Received trade signal: Open EURUSD from MASTER_001"
   - "Broadcasting trade signal to SLAVE_001"

   エラー例:
   - "No active copy settings for master MASTER_001" → 設定が無効またはAccountID不一致
   - "Trade filtered out: EURUSD" → フィルターで除外
   ```

3. **Slave EAのログ:**
   ```
   期待されるログ:
   - "Received MessagePack trade signal"
   - "Order opened successfully: slave #67890 from master #12345"

   エラー例:
   - "Trade filtered out" → フィルターで除外
   - "Failed to open order, Error: 4756" → MT4/MT5エラーコード確認
   ```

**ステップ3: 一般的なエラーコードと対処**

| MT4/MT5エラーコード | 意味 | 対処方法 |
|-----------------|------|---------|
| 4756 | 自動売買が無効 | MT4/MT5の「自動売買」ボタンを有効化 |
| 130 | 無効なストップロス/テイクプロフィット | ストップレベルを確認 |
| 131 | 無効なロットサイズ | ブローカーの最小/最大ロットを確認 |
| 134 | 余剰証拠金不足 | 口座残高を確認、lot_multiplierを調整 |
| 4051 | 無効な関数パラメータ | シンボル名が正しいか確認 |

**ステップ4: パラメータ設定確認**

Slave EAパラメータ:
- ✅ `AllowNewOrders = true`
- ✅ `AllowCloseOrders = true`
- 「自動売買を許可する」が有効

**デバッグ手順:**

1. **シンプルな構成でテスト:**
   - フィルターをすべて無効化
   - シンボルマッピングを削除
   - Reverse Tradeを無効化

2. **手動でトレードシグナルを送信:**
   ```bash
   # APIを使って手動でテスト
   curl -X POST http://localhost:8080/api/test-signal \
     -H "Content-Type: application/json" \
     -d '{
       "action": "Open",
       "ticket": 99999,
       "symbol": "EURUSD",
       "order_type": "Buy",
       "lots": 0.01,
       "open_price": 1.1000,
       "source_account": "TEST_MASTER"
     }'
   ```

### シンボル名のマッピングが機能しない

**症状:**
- シンボルマッピングを設定してもエラーが発生する
- 「Invalid symbol」エラーが表示される

**原因:**
1. Slave側のブローカーに該当シンボルが存在しない
2. シンボルマッピングの設定が間違っている
3. 大文字/小文字の違い

**解決方法:**

1. **Slave側のシンボル名を確認:**
   - MT4/MT5の「気配値表示」で利用可能なシンボルを確認
   - 「通貨ペア一覧」（Ctrl+U）で正確なシンボル名を確認

2. **シンボルマッピング設定例:**
   ```json
   {
     "symbol_mappings": [
       {
         "source_symbol": "EURUSD.raw",
         "target_symbol": "EURUSD"
       },
       {
         "source_symbol": "GBPUSD.raw",
         "target_symbol": "GBPUSD"
       }
     ]
   }
   ```

3. **大文字/小文字の統一:**
   - MT4/MT5では通常すべて大文字
   - 設定でも大文字で統一することを推奨

4. **ログで変換後のシンボル名を確認:**
   - Rustサーバーログで「Transformed symbol: EURUSD.raw -> EURUSD」を確認

### ロット計算が正しくない

**症状:**
- 設定したlot_multiplierと異なるロットでコピーされる
- ロットが小さすぎる/大きすぎる

**原因:**
1. lot_multiplier設定が間違っている
2. ブローカーの最小/最大ロット制限
3. 証拠金不足でロットが自動調整されている

**解決方法:**

1. **lot_multiplier設定を確認:**
   ```json
   {
     "lot_multiplier": 1.0    // Masterと同じロット
     "lot_multiplier": 0.5    // Masterの半分
     "lot_multiplier": 2.0    // Masterの2倍
   }
   ```

2. **ブローカーのロット制限を確認:**
   - MT4/MT5の「仕様」でシンボルの制限を確認
   - 最小ロット（例: 0.01）
   - 最大ロット（例: 100.0）
   - ロットステップ（例: 0.01）

3. **計算ロジックの確認:**

   Slave側で実行されるロット計算:
   ```
   transformed_lots = master_lots × lot_multiplier
   transformed_lots = round_to_lot_step(transformed_lots)
   transformed_lots = clamp(transformed_lots, min_lot, max_lot)
   ```

4. **ログで計算過程を確認:**
   - Rustサーバーログで「Transformed lot: 0.1 -> 0.05」を確認
   - Slave EAログで「Order opened with lot: 0.05」を確認

---

## パフォーマンスの問題

### トレードコピーの遅延が大きい

**症状:**
- MasterとSlaveのトレード実行に数秒の遅延がある
- スキャルピング戦略で価格がずれる

**原因:**
1. MT4のScanInterval設定が大きすぎる
2. ネットワーク遅延
3. サーバーの処理負荷が高い
4. MT5でOnTradeTransaction()が使用されていない

**解決方法:**

1. **MT4のScanIntervalを調整:**
   - Master EAパラメータ: `ScanInterval = 50`（デフォルト100ms）
   - 注意: 小さくしすぎるとCPU負荷が上がる

2. **MT5ではイベント駆動が有効か確認:**
   - MT5ではOnTradeTransaction()が自動的に使用される
   - 遅延は<10msになるはず

3. **ネットワーク遅延の測定:**
   ```powershell
   # ローカル環境
   ping localhost

   # VPS環境
   ping [VPS IP]
   ```

4. **パフォーマンス特性の理解:**

   | 項目 | MT4 | MT5 |
   |------|-----|-----|
   | トレード検出 | 最大100ms（OnTick定期スキャン） | <10ms（OnTradeTransaction イベント） |
   | レイテンシ目標 | 総計150-200ms | 総計<50ms |

5. **Rustサーバーのパフォーマンス確認:**
   - Rustサーバーログで「Processing time: Xms」を確認
   - CPU使用率を確認

**パフォーマンスチューニング:**

1. **ローカル実行を推奨:**
   - MT4/MT5とRustサーバーを同じPC上で実行

2. **VPS使用時:**
   - MT4/MT5サーバーと近いデータセンターのVPSを選択
   - 低遅延ネットワークを確保

### CPU使用率が高い

**症状:**
- Rustサーバーまたはmt4/MT5のCPU使用率が異常に高い
- システムが重くなる

**原因:**
1. ScanIntervalが短すぎる（MT4）
2. 大量のトレードを頻繁に実行している
3. ログ出力が多すぎる
4. メモリリーク

**解決方法:**

1. **ScanIntervalの調整（MT4）:**
   - 通常のトレード: 100ms（デフォルト）
   - 高頻度トレード: 50ms
   - スイングトレード: 200-500ms

2. **ログレベルの調整:**

   `relay-server/config.toml`:
   ```toml
   [logging]
   level = "info"  # "debug"から"info"に変更
   ```

3. **不要なEAを停止:**
   - 使用していないEAやインディケータを停止

4. **リソース使用状況の監視:**
   ```powershell
   # Windowsタスクマネージャーで確認
   # または
   Get-Process | Where-Object {$_.ProcessName -match "terminal|rust"}
   ```

---

## ビルド・コンパイルの問題

### Rustサーバーのビルドエラー

**症状:**
- `cargo build`実行時にエラーが発生する
- ZeroMQライブラリが見つからない

**原因:**
1. ZeroMQライブラリがインストールされていない
2. コンパイラのバージョンが古い

**解決方法:**

1. **Rustバージョンの確認・更新:**
   ```bash
   rustc --version
   # Rust 1.70以上が必要

   # 更新
   rustup update
   ```

2. **ZeroMQライブラリのインストール（Windows）:**
   ```powershell
   # vcpkgを使用
   git clone https://github.com/Microsoft/vcpkg.git
   cd vcpkg
   .\bootstrap-vcpkg.bat
   .\vcpkg install zeromq:x64-windows
   ```

3. **依存関係の再取得:**
   ```bash
   cd relay-server
   cargo clean
   cargo build --release
   ```

### WebUIのビルドエラー

**症状:**
- `npm install`または`npm run build`でエラーが発生する
- 依存関係の解決に失敗する

**原因:**
1. Node.jsのバージョンが古い
2. package-lock.jsonが壊れている
3. node_modulesが不完全

**解決方法:**

1. **Node.jsバージョンの確認:**
   ```bash
   node --version
   # v18以上が必要

   # 必要に応じて最新LTS版をインストール
   # https://nodejs.org/
   ```

2. **クリーンインストール:**
   ```bash
   cd web-ui

   # キャッシュとnode_modulesを削除
   rm -rf node_modules
   rm package-lock.json

   # 再インストール
   npm install
   ```

3. **pnpmの使用（推奨）:**
   ```bash
   # pnpmのインストール
   npm install -g pnpm

   # pnpmで依存関係をインストール
   pnpm install
   ```

---

## よくある質問（FAQ）

### Q1: MT4とMT5を混在させて使用できますか？

**A:** はい、可能です。SANKEY CopierはMT4↔MT5の双方向コピーに対応しています。

例:
- Master: MT4、Slave: MT5 ✅
- Master: MT5、Slave: MT4 ✅
- Master: MT4、Slave: MT4 ✅
- Master: MT5、Slave: MT5 ✅

### Q2: 複数のMaster口座から1つのSlave口座にコピーできますか？

**A:** いいえ、現在のバージョンでは1つのSlave口座は1つのMaster口座からのみコピーを受け付けます。

サポートされる構成:
- 1 Master → 複数 Slave ✅
- 複数 Master → 1 Slave ❌（将来のバージョンで対応予定）

### Q3: リモートVPSで実行できますか？

**A:** はい、可能です。以下の構成が推奨されます:

**構成1: すべてVPS上で実行**
```
[VPS]
  ├─ Rust Server
  ├─ MT4/MT5 (Master)
  └─ MT4/MT5 (Slave)

[ローカルPC]
  └─ WebUI (http://VPS_IP:8080 経由で接続)
```

**構成2: 分散構成**
```
[VPS1] MT4/MT5 (Master) + Rust Server
[VPS2] MT4/MT5 (Slave)
[ローカルPC] WebUI
```

セキュリティ注意:
- ファイアウォールで必要なポートのみ開放
- VPNの使用を推奨（Tailscale、WireGuardなど）

### Q4: 自動売買EAのトレードもコピーされますか？

**A:** はい、Master口座で実行されたすべてのトレード（手動・EA問わず）がコピーされます。

特定のEAのトレードのみコピーしたい場合:
- EAに固有のマジックナンバーを設定
- Master EAパラメータ: `MagicFilter = [EA's Magic Number]`
- またはWebUIのフィルター設定で`allowed_magic_numbers`を指定

### Q5: 決済のみコピーしない設定は可能ですか？

**A:** はい、Slave EAパラメータで制御できます:

```
AllowNewOrders = true   // 新規注文をコピー
AllowCloseOrders = false  // 決済はコピーしない
```

この設定では、Master口座が決済してもSlave口座では決済されず、手動で管理できます。

### Q6: バックアップはどうすればいいですか？

**A:** 以下のファイルをバックアップしてください:

1. **データベース:**
   ```
   relay-server/copier.db
   relay-server/copier.db-shm
   relay-server/copier.db-wal
   ```

2. **設定ファイル:**
   ```
   relay-server/config.toml
   web-ui/.env.local
   ```

自動バックアップスクリプトの設定方法は [operations.md](./operations.md#バックアップリストア) を参照してください。

### Q7: ログファイルはどこにありますか？

**A:**

- **MT4/MT5ログ:**
  ```
  C:\Users\[User]\AppData\Roaming\MetaQuotes\Terminal\[TerminalID]\MQL4\Logs\
  C:\Users\[User]\AppData\Roaming\MetaQuotes\Terminal\[TerminalID]\MQL5\Logs\
  ```

- **Rustサーバーログ:**
  - 標準出力（コンソール）
  - またはconfig.tomlで設定したファイル

- **WebUIログ:**
  - ブラウザのデベロッパーツール → コンソールタブ

### Q8: スマートフォンから制御できますか？

**A:** はい、WebUIはレスポンシブデザインでスマートフォンに対応しています。

アクセス方法:
1. **同じネットワーク内:**
   - サーバーのIPアドレスを確認（例: 192.168.1.100）
   - スマホのブラウザで `http://192.168.1.100:5173` にアクセス

2. **外部ネットワークから:**
   - VPNの使用を推奨（Tailscale、WireGuardなど）
   - セキュリティリスクのため、直接のポート開放は非推奨

---

## サポート

上記の解決方法で問題が解決しない場合は、以下の情報と共にGitHub Issuesで報告してください:

**必要な情報:**
- OS バージョン
- Rust バージョン（`rustc --version`）
- Node.js バージョン（`node --version`）
- MT4/MT5 ビルド番号
- エラーログ（Rustサーバー、MT4/MT5 EA、WebUI）
- 設定内容（config.toml、WebUIの設定画面のスクリーンショット）
- 再現手順

**ログの取得方法:**
```bash
# Rustサーバーログをファイルに出力
cargo run --release > server.log 2>&1

# MT4/MT5ログの場所
%APPDATA%\MetaQuotes\Terminal\[TerminalID]\MQL4\Logs\
```

---

**関連ドキュメント:**
- [セットアップガイド](./setup.md)
- [運用・デプロイガイド](./operations.md)
- [アーキテクチャ](./architecture.md)
- [API仕様](./api-specification.md)
