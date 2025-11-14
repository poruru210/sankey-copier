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

## アーキテクチャ

### クラウド版（推奨）

```
[ スマホ / PC / タブレット ]
         │
         ↓ HTTPS
┌─────────────────────────┐
│  Cloudflare Access      │ ← 認証（Google/GitHub/Email）
│  (認証ゲートウェイ)      │
└────────┬────────────────┘
         │
    ┌────┴─────┐
    │          │
    ↓          ↓
┌─────────┐  ┌────────────────────┐
│ Vercel  │  │ Cloudflare Tunnel  │
│ Web UI  │  │ (暗号化トンネル)    │
└─────────┘  └──────────┬─────────┘
                        │
                        ↓ localhost:3000
              ┌──────────────────┐
              │  Rust中継サーバー  │
              │  + SQLite DB     │
              └──────┬───────────┘
                     │
         ┌───────────┴───────────┐
         │ ZeroMQ                │ ZeroMQ
         ↓ (5555)                ↓ (5556)
┌─────────────────┐      ┌─────────────────┐
│  MT4/MT5 Master │      │  MT4/MT5 Slave  │
│       EA        │      │       EA        │
└─────────────────┘      └─────────────────┘
```

### Desktop App版（ローカル設定用）

```
┌─────────────────┐      ZeroMQ        ┌──────────────────┐      ZeroMQ        ┌─────────────────┐
│  MT4/MT5 Master │ ───────────────> │  Rust中継サーバー  │ ───────────────> │  MT4/MT5 Slave  │
│       EA        │   ポート: 5555    │  + SQLite DB     │   ポート: 5556    │       EA        │
└─────────────────┘                  └──────────────────┘                  └─────────────────┘
                                            │ ▲
                                            │ │ HTTP/WebSocket
                                            │ │ ポート: 3000
                                            ▼ │
                                     ┌──────────────────┐
                                     │   Desktop App    │
                                     │  (Tauri + Next)  │
                                     └──────────────────┘
```

## セットアップ

SANKEY Copierは2つのデプロイ方法をサポートしています：

### 1. クラウド版（推奨） - どこからでもアクセス可能

イントラネット内のrust-serverをCloudflare Tunnelで公開し、Web-UIをVercelにデプロイします。

**メリット:**
- スマホ・タブレットから**どこからでも**アクセス可能
- 固定IPアドレス不要
- Cloudflare Accessによる強固な認証（Google/GitHub/メール）
- 自動SSL証明書、無料で始められる

**必要要件:**
- Cloudflareアカウント（無料プランでOK）
- Cloudflareでドメイン管理（既存ドメインをCloudflareに移管）
- Vercelアカウント（無料プランでOK）

**手順:**

1. **Rust-Serverのセットアップ（イントラネット内のPC）**
   ```bash
   cd rust-server
   cargo run --release
   ```

2. **Cloudflare Tunnelのセットアップ**
   - 詳細は [docs/CLOUDFLARE_SETUP.md](docs/CLOUDFLARE_SETUP.md) を参照
   - Cloudflare Tunnelでrust-serverを公開（例: `https://api.yourdomain.com`）
   - Cloudflare Accessで認証を設定

3. **VercelにWeb-UIをデプロイ**
   - 詳細は [docs/VERCEL_DEPLOYMENT.md](docs/VERCEL_DEPLOYMENT.md) を参照
   - VercelにWeb-UIをデプロイ（例: `https://app.yourdomain.com`）

4. **Web-UIでrust-serverを登録**
   - Web-UIの「Sites」ページで`https://api.yourdomain.com`を登録
   - 認証後、どこからでもアクセス可能

### 2. Desktop App版 - ローカル設定用

ローカルネットワーク内で完結するデスクトップアプリ版です。

**メリット:**
- インターネット接続不要
- ローカルで高速動作
- 設定変更が簡単

**必要要件:**
- Windows 10/11
- MetaTrader 4 または 5

**手順:**

1. **Desktop Appのビルド**
   ```bash
   cd desktop-app
   npm install
   npm run tauri build
   ```

2. **アプリケーションの起動**
   - `src-tauri/target/release/sankey-copier.exe`を実行
   - 自動的にrust-serverとweb-uiが起動

3. **MT4/MT5 コンポーネントのインストール**
   - Desktop Appの「MT4/MT5 Installations」ページで自動検出
   - 「Install」ボタンでDLL/EAをインストール

### 3. トレードコピーの設定（共通）

1. **Master EA をアタッチ**
   - MT4/MT5のチャートに`SankeyCopierMaster.ex4/.ex5`をドロップ
   - EAパラメータは基本的にデフォルトでOK

2. **Slave EA をアタッチ**
   - Slave口座のチャートに`SankeyCopierSlave.ex4/.ex5`をドロップ

3. **Web-UIでコピー設定を作成**
   - 「+ New Setting」をクリック
   - Master/Slave口座を選択
   - ロット倍率、シンボル変換等を設定
   - 「Enable」でコピー開始

### 開発者向けセットアップ

開発やカスタマイズを行う場合：

**必要要件:**
- Rust 1.70以上
- Node.js 20以上
- MetaTrader 4 または 5

**手順:**
1. Rustサーバー: `cd rust-server && cargo run --release`
2. Web-UI開発サーバー: `cd web-ui && npm install && npm run dev`
3. http://localhost:8080 でアクセス

## ドキュメント

### セットアップ・運用
- **[Cloudflare Setup Guide](docs/CLOUDFLARE_SETUP.md)** - Cloudflare Tunnel + Accessの設定手順（WebSocket対応）
- **[Vercel Deployment Guide](docs/VERCEL_DEPLOYMENT.md)** - VercelへのWeb-UIデプロイ手順
- **[GitHub Actions Vercel Deploy](docs/GITHUB_ACTIONS_VERCEL.md)** - GitHub Actions経由での自動デプロイ設定
- **[セットアップガイド](docs/setup.md)** - 初心者向けの詳細なインストール手順（Desktop App版）
- **[運用・デプロイガイド](docs/operations.md)** - 本番環境へのデプロイ、メンテナンス、バックアップ
- **[トラブルシューティング](docs/troubleshooting.md)** - よくある問題と解決方法

### 技術仕様
- **[システムアーキテクチャ](docs/architecture.md)** - システム設計、コンポーネント構成、データフロー
- **[API仕様](docs/api-specification.md)** - REST API、WebSocket、ZeroMQプロトコル
- **[データモデル](docs/data-model.md)** - データベーススキーマ、データ構造

## 主要な機能

### シンボル名の変換

ブローカーによってシンボル名が異なる場合に対応:

```json
{
  "symbol_mappings": [
    { "source_symbol": "EURUSD.raw", "target_symbol": "EURUSD" }
  ]
}
```

### トレードフィルター

特定の通貨ペアやマジックナンバーのみコピー:

```json
{
  "filters": {
    "allowed_symbols": ["EURUSD", "GBPUSD"],
    "allowed_magic_numbers": [12345]
  }
}
```

### ロット調整

```json
{
  "lot_multiplier": 1.0  // Masterと同じロット
  "lot_multiplier": 0.5  // Masterの半分
  "lot_multiplier": 2.0  // Masterの2倍
}
```

### 売買反転

```json
{
  "reverse_trade": true  // Buy ↔ Sell を反転
}
```

## パフォーマンス

| 項目 | MT4 | MT5 |
|------|-----|-----|
| **トレード検出** | 最大100ms（OnTick定期スキャン） | <10ms（OnTradeTransaction イベント駆動） |
| **総レイテンシ** | 150-200ms | <50ms |
| **適用シーン** | デイトレード、スイングトレード | スキャルピング、高頻度トレード |

詳細は [docs/architecture.md](docs/architecture.md) を参照してください。

## ライセンス

MIT License

## セキュリティ

### クラウド版
- **Cloudflare Access**による認証を必ず設定してください
- Google/GitHub/メールアドレスによるSSO認証に対応
- 許可されたユーザーのみがアクセス可能
- WebSocket通信も自動的に保護されます

### Desktop App版
- ローカルネットワーク内でのみ使用してください
- 外部に公開する場合は適切なファイアウォール設定を行ってください
- VPN（Tailscale、WireGuardなど）の使用を推奨します

## サポート

問題が発生した場合:

1. [トラブルシューティングガイド](docs/troubleshooting.md)を確認
2. [GitHubのIssues](https://github.com/[your-repo]/issues)で既存の問題を検索
3. 解決しない場合は新しいIssueを作成してください

## 貢献

プルリクエストを歓迎します！バグ報告や機能提案もお気軽にどうぞ。

## ロードマップ

- [x] クラウド版の提供（Vercel + Cloudflare Tunnel）
- [x] WebUI認証機能（Cloudflare Access）
- [ ] 詳細なトレード履歴とパフォーマンス分析
- [ ] Telegram/Discord通知
- [ ] スマホアプリ (iOS/Android)

---

**注意**: このソフトウェアは実験的なものです。リアル口座で使用する前に、必ずデモ口座で十分なテストを行ってください。
