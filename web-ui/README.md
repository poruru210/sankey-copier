# SANKEY SANKEY Copier - Web UI

Next.js 16 + Intlayer による多言語対応Web UI

## 特徴

- **Next.js 16** - 最新のReactフレームワーク
- **Intlayer** - 型安全な多言語化（英語・日本語）
- **TypeScript** - 完全な型サポート
- **Tailwind CSS** - モダンなスタイリング
- **WebSocket** - リアルタイム更新
- **Responsive Design** - スマホ対応

## 多言語対応

このアプリケーションは [Intlayer](https://intlayer.org/ja/doc/environment/nextjs) を使用して、英語と日本語の多言語化を実現しています。

### サポート言語

- 英語 (`/en`)
- 日本語 (`/ja`)

言語は自動的にブラウザの設定から検出され、ヘッダーの言語切り替えボタンで変更できます。

## セットアップ

### 依存関係のインストール

```bash
pnpm install
```

### 開発サーバーの起動

```bash
pnpm dev
```

開発サーバー: [http://localhost:5173](http://localhost:5173)

デフォルトでは `/en` (英語) にリダイレクトされます。

### 本番ビルド

```bash
pnpm build
pnpm start
```

## プロジェクト構造

```
web-ui/
├── app/
│   ├── [locale]/           # 多言語ルーティング
│   │   ├── layout.tsx      # ルートレイアウト
│   │   ├── page.tsx        # メインページ
│   │   └── page.content.ts # 翻訳コンテンツ
│   └── globals.css         # グローバルスタイル
├── components/
│   ├── ui/                 # 基本UIコンポーネント
│   │   ├── button.tsx
│   │   ├── card.tsx
│   │   ├── dialog.tsx
│   │   ├── input.tsx
│   │   ├── switch.tsx
│   │   └── ...
│   ├── ActivityLog.tsx     # アクティビティログ
│   ├── ConnectionsView.tsx # 接続一覧表示
│   ├── LanguageSwitcher.tsx # 言語切り替え
│   └── SettingsDialog.tsx  # 設定ダイアログ
├── hooks/
│   └── useSankeyCopier.ts   # API統合フック
├── lib/
│   └── utils.ts            # ユーティリティ関数
├── types/
│   └── index.ts            # TypeScript型定義
├── intlayer.config.ts      # Intlayer設定
├── next.config.ts          # Next.js設定
├── middleware.ts           # 多言語ミドルウェア
└── tailwind.config.ts      # Tailwind設定
```

## API統合

Next.jsのrewrite機能により、フロントエンドから直接Rust Serverにアクセスできます。

- `/api/*` → `http://localhost:8080/api/*`
- `/ws` → `http://localhost:8080/ws` (WebSocket)

## 翻訳の追加

1. `*.content.ts` ファイルを編集：

```typescript
import { t, type DeclarationContent } from 'intlayer';

const content = {
  key: 'my-component',
  content: {
    title: t({
      en: 'Hello World',
      ja: 'こんにちは世界',
    }),
  },
} satisfies DeclarationContent;

export default content;
```

2. コンポーネントで使用：

```typescript
import { useIntlayer } from 'next-intlayer';

export function MyComponent() {
  const { title } = useIntlayer('my-component');
  return <h1>{title}</h1>;
}
```

## 環境変数

必要に応じて `.env.local` を作成：

```env
# Next.js
NEXT_PUBLIC_API_URL=http://localhost:8080

# その他の環境変数
```

## 既知の問題

### Windows環境での警告

Windows環境で開発サーバー起動時に以下の警告が表示される場合がありますが、動作には問題ありません：

```
X [ERROR] Expected "." but found "IDEA"
    process.env.IntelliJ IDEA
```

これはWindowsの環境変数名にスペースや括弧が含まれているためですが、Next.jsの動作には影響しません。

## ライセンス

MIT License
