# SANKEY Copier

高性能なMT4/MT5トレードコピーシステム。低遅延のローカル通信とスマートフォンからのリモート制御が可能です。

## 主な特徴

- **双方向対応**: MT4↔MT5、MT4↔MT4、MT5↔MT5のすべての組み合わせに対応
- **低遅延**: ZeroMQによるローカル通信で最小限の遅延（MT5: <10ms、MT4: <100ms）
- **リモート制御**: スマートフォンからコピー設定のON/OFF、リアルタイム監視が可能
- **柔軟なシンボル変換**: プリフィックス/サフィックス削除・追加、完全なシンボル名マッピング
- **高度なフィルタリング**: 通貨ペア、マジックナンバーによるフィルタリング
- **ロット調整**: 固定倍率、残高比率などに対応
- **リアルタイム監視**: WebUIでトレードコピーの状態をリアルタイム確認
- **多言語対応**: 英語・日本語に対応したWebUI

## システム構成

```
┌─────────────────┐      ZeroMQ        ┌──────────────────┐      ZeroMQ        ┌─────────────────┐
│  MT4/MT5 Master │ ───────────────> │  Relay Server    │ ───────────────> │  MT4/MT5 Slave  │
│       EA        │   ポート: 5555    │  + SQLite DB     │   ポート: 5556    │       EA        │
└─────────────────┘                  └──────────────────┘                  └─────────────────┘
                                            │ ▲
                                            │ │ HTTPS/WebSocket
                                            │ │ ポート: 3000
                                            ▼ │
                                     ┌──────────────────┐
                                     │   Web-UI         │
                                     │  (React/Next.js) │
                                     └──────────────────┘
```

詳細なアーキテクチャ図は [docs/README.md](docs/README.md) を参照してください。

## クイックスタート

### 必要要件

- Windows 10/11 (64-bit)
- MetaTrader 4 または 5

### インストール

1. **インストーラーをダウンロード**
   - [GitHubのReleases](https://github.com/poruru210/sankey-copier/releases)から最新版をダウンロード
   - `SankeyCopierSetup-x.x.x.exe`を実行

2. **MT4/MT5 コンポーネントのインストール**
   - Web-UIの「Installations」ページでMT4/MT5を自動検出
   - 「Install」ボタンでDLL/EAをインストール

3. **EAのセットアップ**
   - Master口座: `SankeyCopierMaster.ex4/.ex5`をチャートにドロップ
   - Slave口座: `SankeyCopierSlave.ex4/.ex5`をチャートにドロップ

4. **コピー設定の作成**
   - Web-UIの「Connections」ページで「+ Create」をクリック
   - Master/Slave口座を選択、ロット倍率等を設定
   - 「Enable」でコピー開始

## 主要な機能

### シンボル名の変換

ブローカーによってシンボル名が異なる場合に対応:

| Master | Slave | 設定 |
|--------|-------|------|
| EURUSD.raw | EURUSD | symbol_suffix: ".raw" を削除 |
| EURUSDm | EURUSD | symbol_suffix: "m" を削除 |
| XAUUSD | GOLD | symbol_mappings で変換 |

### トレードフィルター

- **allowed_symbols**: 指定した通貨ペアのみコピー
- **blocked_symbols**: 指定した通貨ペアを除外
- **allowed_magic_numbers**: 指定したマジックナンバーのみコピー
- **blocked_magic_numbers**: 指定したマジックナンバーを除外

### ロット調整

| モード | 説明 |
|--------|------|
| Multiplier | 固定倍率（例: 2.0 = Masterの2倍） |
| MarginRatio | 残高比率（Slave残高 / Master残高） |

### その他の機能

- **reverse_trade**: Buy ↔ Sell を反転
- **sync_mode**: 初回接続時のポジション同期
- **max_slippage**: 最大スリッページ設定
- **max_retries**: 注文失敗時のリトライ回数

## パフォーマンス

| 項目 | MT4 | MT5 |
|------|-----|-----|
| **トレード検出** | 最大100ms（OnTick定期スキャン） | <10ms（OnTradeTransaction イベント駆動） |
| **総レイテンシ** | 150-200ms | <50ms |
| **適用シーン** | デイトレード、スイングトレード | スキャルピング、高頻度トレード |

## 開発者向け

### 必要要件

- Rust 1.70以上
- Node.js 20以上
- mise ([公式サイト](https://mise.jdx.dev/))

### ビルド・実行

```bash
# pnpm のインストール
mise install

# Relay Server
cd relay-server && cargo run --release

# Web-UI 開発サーバー
cd web-ui && pnpm install && pnpm dev
```

http://localhost:8080 でアクセス

## ドキュメント

### 技術ドキュメント

- **[ドキュメントインデックス](docs/README.md)** - システム全体構成図、アーキテクチャ図
- **[relay-server](docs/relay-server.md)** - 中継サーバーの詳細
- **[mt-bridge](docs/mt-bridge.md)** - 通信DLLの詳細
- **[mt-advisors](docs/mt-advisors.md)** - EA (Master/Slave)の詳細
- **[web-ui](docs/web-ui.md)** - WebUIの詳細

### 旧ドキュメント

過去のドキュメントは [docs/old/](docs/old/) に保存されています。

## ライセンス

MIT License

## サポート

問題が発生した場合:

1. [GitHubのIssues](https://github.com/poruru210/sankey-copier/issues)で既存の問題を検索
2. 解決しない場合は新しいIssueを作成してください

## 貢献

プルリクエストを歓迎します！バグ報告や機能提案もお気軽にどうぞ。

---

**注意**: このソフトウェアは実験的なものです。リアル口座で使用する前に、必ずデモ口座で十分なテストを行ってください。
