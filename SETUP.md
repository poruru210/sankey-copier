# SANKEY Copier セットアップガイド

## 必要なもの

- MetaTrader 4 または MetaTrader 5
- Rustサーバー（既にビルド済み）
- WebUI（既にセットアップ済み）
- mql-zmqライブラリ（既にインストール済み）

## セットアップ手順

### 1. ZeroMQ DLLのインストール

#### MT4の場合
1. `mql/MT4/Libraries/` フォルダ内の以下のDLLファイルを確認：
   - `libzmq.dll`
   - `libsodium.dll`

2. これらのDLLを MT4 の `MQL4/Libraries/` フォルダにコピー：
   ```
   C:\Program Files (x86)\MetaTrader 4\MQL4\Libraries\
   または
   C:\Users\[ユーザー名]\AppData\Roaming\MetaQuotes\Terminal\[ターミナルID]\MQL4\Libraries\
   ```

#### MT5の場合
1. `mql/MT5/Libraries/` フォルダ内の以下のDLLファイルを確認：
   - `libzmq.dll`
   - `libsodium.dll`

2. これらのDLLを MT5 の `MQL5/Libraries/` フォルダにコピー：
   ```
   C:\Program Files\MetaTrader 5\MQL5\Libraries\
   または
   C:\Users\[ユーザー名]\AppData\Roaming\MetaQuotes\Terminal\[ターミナルID]\MQL5\Libraries\
   ```

### 2. mql-zmq Includeファイルのインストール

`mql/Include/` フォルダ内の `Mql` と `Zmq` フォルダを：

**MT4の場合:**
```
C:\Program Files (x86)\MetaTrader 4\MQL4\Include\
```

**MT5の場合:**
```
C:\Program Files\MetaTrader 5\MQL5\Include\
```
にコピー

### 3. Expert Advisorのコンパイルとインストール

#### Master EA (MT4)
1. `mql/MT4/Master/SankeyCopierMaster.mq4` を MT4 の `MQL4/Experts/` フォルダにコピー
2. MetaEditorで開いてコンパイル（F7キー）
3. エラーがないことを確認

#### Master EA (MT5)
1. `mql/MT5/Master/SankeyCopierMaster.mq5` を MT5 の `MQL5/Experts/` フォルダにコピー
2. MetaEditorで開いてコンパイル（F7キー）
3. エラーがないことを確認

#### Slave EA (MT4)
1. `mql/MT4/Slave/SankeyCopierSlave.mq4` を MT4 の `MQL4/Experts/` フォルダにコピー
2. MetaEditorで開いてコンパイル（F7キー）
3. エラーがないことを確認

#### Slave EA (MT5)
1. `mql/MT5/Slave/SankeyCopierSlave.mq5` を MT5 の `MQL5/Experts/` フォルダにコピー
2. MetaEditorで開いてコンパイル（F7キー）
3. エラーがないことを確認

### 4. サーバーとWebUIの起動

#### Rustサーバー
```bash
cd rust-server
cargo run --release
```

サーバーが起動すると以下のように表示されます：
```
Starting SANKEY Copier Server...
Database initialized
Connection manager initialized
ZeroMQ receiver started on tcp://*:5555
Loaded X copy settings
HTTP server listening on http://0.0.0.0:8080
```

#### WebUI
```bash
cd web-ui
npm run dev
```

WebUIが起動すると：
```
VITE v5.x.x  ready in XXX ms

➜  Local:   http://localhost:5175/
```

### 5. Expert Advisorの起動

#### Master EA の設定
1. MT4/MT5のナビゲーターから `SankeyCopierMaster` を選択
2. チャートにドラッグ&ドロップ
3. パラメーター設定：
   - **AccountID**: `master-001` （任意のユニークID）
   - **ServerAddress**: `tcp://localhost:5555` （デフォルト）
   - **EnableAutoAlgo**: `true` （Allow automated trading）
   - **AllowDLL**: `true` （DLL imports を許可）

4. OKをクリック

#### Slave EA の設定
1. MT4/MT5のナビゲーターから `SankeyCopierSlave` を選択
2. チャートにドラッグ&ドロップ
3. パラメーター設定：
   - **AccountID**: `slave-001` （任意のユニークID）
   - **ServerAddress**: `tcp://localhost:5556` （デフォルト）
   - **EnableAutoAlgo**: `true` （Allow automated trading）
   - **AllowDLL**: `true` （DLL imports を許可）

4. OKをクリック

### 6. 動作確認

#### WebUIでの確認
1. ブラウザで http://localhost:5175/ を開く
2. **Copy Connections** セクションに Master と Slave のアカウントが表示されることを確認
3. ステータスが **Online** （緑色）になっていることを確認

#### Copy Settingsの作成
1. WebUIの **Copy Settings** セクションで「+ New Setting」をクリック
2. Master Account と Slave Account を選択
3. Lot Multiplier を設定（例: 1.0）
4. 必要に応じて Reverse Trade を有効化
5. 「Create」をクリック

#### トレードのテスト
1. Master EA が稼働しているチャートで手動で注文を出す
2. WebUIの **Recent Activity** にトレード信号が表示されることを確認
3. Slave EA のチャートに同じ注文が自動的にコピーされることを確認

## トラブルシューティング

### EA が接続できない
- MT4/MT5 の「ツール」→「オプション」→「エキスパートアドバイザ」で以下を確認：
  - ☑ 自動売買を許可する
  - ☑ DLL の使用を許可する

### DLL エラーが出る
- DLLファイルが正しい場所にコピーされているか確認
- MT4/MT5 を再起動

### コンパイルエラー（MT5 Build 5120以降）
- このプロジェクトでは **Furious-Production-LTD/mql-zmq** フォークを使用しています
- オリジナルの mql-zmq では MT5 Build 5120以降でコンパイルエラーが発生する既知の問題があります

### ZeroMQ エラー
- Rustサーバーが起動しているか確認
- ファイアウォールがポート 5555, 5556 をブロックしていないか確認

## 注意事項

### mql-zmq ライブラリについて
- **バージョン**: Furious-Production-LTD/mql-zmq フォーク
- **理由**: オリジナル版は2017年から更新されておらず、MT5 Build 5120以降でコンパイルエラーが発生
- **ライセンス**: Apache-2.0
- **メンテナンス状況**: フォーク版は新しいMT5ビルドに対応済み

### セキュリティ
- このシステムはローカルネットワークでの使用を想定しています
- リモート接続を許可する場合は適切なセキュリティ対策を実施してください

### 本番環境への展開前に
1. デモアカウントで十分にテストする
2. 小さいロットサイズでテストする
3. フィルター機能を適切に設定する
4. ログを定期的に確認する
