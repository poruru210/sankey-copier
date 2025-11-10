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

```
┌─────────────────┐      ZeroMQ        ┌──────────────────┐      ZeroMQ        ┌─────────────────┐
│  MT4/MT5 Master │ ───────────────> │  Rust中継サーバー  │ ───────────────> │  MT4/MT5 Slave  │
│       EA        │   ポート: 5555    │  + SQLite DB     │   ポート: 5556    │       EA        │
└─────────────────┘                  └──────────────────┘                  └─────────────────┘
                                             │ ▲
                                             │ │ HTTP/WebSocket
                                             │ │ ポート: 8080
                                             ▼ │
                                      ┌──────────────────┐
                                      │   Web UI         │
                                      │  (Next.js 16)    │
                                      └──────────────────┘
                                             ▲
                                             │
                                      [ スマホ / PC ]
```

## クイックスタート

### 必要要件

- **Windows 10/11** (MT4/MT5用)
- **Rust 1.70以上**
- **Node.js 18以上**
- **MetaTrader 4 または 5**

### 基本的なセットアップ

1. **Rustサーバーの起動**
   ```bash
   cd rust-server
   cargo run --release
   ```

2. **WebUIの起動**
   ```bash
   cd web-ui
   pnpm install
   pnpm dev
   ```
   ブラウザで http://localhost:5173 を開く

3. **MT4/MT5 EAの設定**
   - Master EA (`SankeyCopierMaster`) をMaster口座のチャートにアタッチ
   - Slave EA (`SankeyCopierSlave`) をSlave口座のチャートにアタッチ

4. **WebUIでコピー設定を作成**
   - 「+ New Setting」からMaster/Slave口座を選択
   - Lot Multiplierなどを設定して「Create」→「Enable」

詳細なセットアップ手順は [docs/setup.md](docs/setup.md) を参照してください。

## ドキュメント

### セットアップ・運用
- **[セットアップガイド](docs/setup.md)** - 初心者向けの詳細なインストール手順
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

- 本番環境では適切なファイアウォール設定を行ってください
- WebUIには認証機能がないため、外部に公開しないでください
- 外部アクセスが必要な場合はVPN（Tailscale、WireGuardなど）の使用を推奨します

## サポート

問題が発生した場合:

1. [トラブルシューティングガイド](docs/troubleshooting.md)を確認
2. [GitHubのIssues](https://github.com/[your-repo]/issues)で既存の問題を検索
3. 解決しない場合は新しいIssueを作成してください

## 貢献

プルリクエストを歓迎します！バグ報告や機能提案もお気軽にどうぞ。

## ロードマップ

- [ ] WebUI認証機能
- [ ] 詳細なトレード履歴とパフォーマンス分析
- [ ] Telegram/Discord通知
- [ ] クラウド版の提供
- [ ] スマホアプリ (iOS/Android)

---

**注意**: このソフトウェアは実験的なものです。リアル口座で使用する前に、必ずデモ口座で十分なテストを行ってください。
