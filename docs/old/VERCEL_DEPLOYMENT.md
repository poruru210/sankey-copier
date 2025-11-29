# Vercel Deployment Guide for Web-UI

このガイドでは、Sankey Copier Web-UIをVercelにデプロイする方法を説明します。

## デプロイ方法の選択

3つのデプロイ方法があります：

1. **GitHub Actions（推奨）** - 自動デプロイ、CI/CD完全統合
2. **Vercel CLI** - 手動デプロイ、シンプル
3. **Vercel Git連携** - GitHubと自動連携

## 前提条件

- GitHubアカウント
- Vercelアカウント（無料プラン可）
- Node.js 20.x以上

---

## デプロイ方法 1: GitHub Actions（推奨）

### 概要

GitHub Actionsを使用して、`main`ブランチへのpush時に自動的にVercelにデプロイします。

**メリット:**
- ✅ 完全自動化（pushするだけ）
- ✅ Pull RequestごとにPreview環境
- ✅ デプロイ履歴がGitHub Actionsに記録
- ✅ ロールバックが簡単

**詳細な設定手順:**

👉 **[GitHub Actions経由でのVercel自動デプロイ設定ガイド](./GITHUB_ACTIONS_VERCEL.md)**を参照してください。

### クイックスタート

1. **GitHubシークレットを設定**
   - `VERCEL_TOKEN`: Vercelアクセストークン
   - `VERCEL_ORG_ID`: VercelのOrg ID
   - `VERCEL_PROJECT_ID`: VercelのProject ID

2. **変更をpush**
   ```bash
   git push origin main
   ```

3. **GitHub Actionsが自動実行**
   - GitHubの**Actions**タブで進捗確認
   - デプロイ完了後、URLがログに表示

---

## デプロイ方法 2: Vercel CLI

### 2.1 Vercel CLIのインストール

```bash
npm install -g vercel
```

### 2.2 Vercelにログイン

```bash
vercel login
```

ブラウザが開くので、GitHubアカウント等でログインします。

### 2.3 プロジェクトをデプロイ

```bash
cd web-ui

# 本番デプロイ
vercel --prod
```

初回デプロイ時の質問:
- **Set up and deploy "~/sankey-copier/web-ui"?**: `Y`
- **Which scope do you want to deploy to?**: あなたのアカウント名を選択
- **Link to existing project?**: `N`
- **What's your project's name?**: `sankey-copier-web` (任意)
- **In which directory is your code located?**: `./` (そのままEnter)
- **Want to override the settings?**: `N`

デプロイが完了すると、URLが表示されます:
```
✅ Production: https://sankey-copier-web.vercel.app
```

---

## デプロイ方法 3: Vercel Git連携

### 3.1 GitHubにコードをプッシュ

```bash
cd /home/user/sankey-copier
git push origin main
```

### 3.2 Vercelダッシュボードでプロジェクト作成

1. [Vercel Dashboard](https://vercel.com/dashboard)にアクセス
2. **Add New...** → **Project**をクリック
3. **Import Git Repository**でGitHubリポジトリを連携
4. `poruru210/sankey-copier`を選択

### 3.3 ビルド設定

- **Framework Preset**: `Next.js`（自動検出されます）
- **Root Directory**: `web-ui`
- **Build Command**: `npm run build`（デフォルト）
- **Output Directory**: `.next`（デフォルト）
- **Install Command**: `npm install`（デフォルト）

**Environment Variables**:
なし（現時点では不要）

### 3.4 デプロイ

**Deploy**をクリック。

数分後、デプロイが完了し、URLが発行されます:
```
https://sankey-copier-web-<random>.vercel.app
```

### 3.5 自動デプロイの設定

GitHub連携済みの場合、`main`ブランチへのpush時に自動デプロイされます。

---

## 4. カスタムドメインの設定

### 4.1 Vercelでドメインを追加

1. Vercelダッシュボードでプロジェクトを開く
2. **Settings** → **Domains**
3. カスタムドメインを入力（例: `app.yourdomain.com`）
4. **Add**をクリック

### 4.2 Cloudflare DNSの設定

Vercelが表示するDNS設定をCloudflareに追加します。

1. [Cloudflare Dashboard](https://dash.cloudflare.com/)にログイン
2. **DNS** → **Records**
3. 以下のレコードを追加:

```
Type: CNAME
Name: app (またはあなたの希望するサブドメイン)
Content: cname.vercel-dns.com
Proxy status: Proxied (オレンジ色のクラウドアイコン)
TTL: Auto
```

### 4.3 SSL証明書の確認

Cloudflareの**SSL/TLS**設定を確認:
- **Encryption mode**: `Full` または `Full (strict)`

数分待つと、Vercel側でSSL証明書が自動発行され、`https://app.yourdomain.com`でアクセス可能になります。

---

## 4. Cloudflare Accessでの保護（オプション）

Web-UIもCloudflare Accessで保護する場合:

### 4.1 Access Applicationの作成

1. Cloudflare Zero Trust → **Access** → **Applications**
2. **Add an application** → **Self-hosted**
3. 設定:
   - **Application name**: `Sankey Copier Web`
   - **Application domain**: `app.yourdomain.com`
   - **Session Duration**: `24 hours`

### 4.2 Policy設定

- **Policy name**: `Allow authorized users`
- **Action**: `Allow`
- **Include**: 許可するユーザー（メールアドレス、ドメイン等）

### 4.3 動作確認

`https://app.yourdomain.com`にアクセスすると、Cloudflare Accessのログイン画面が表示されます。

---

## 5. 環境変数の設定（必要に応じて）

Vercel環境変数は、Dashboardの**Settings** → **Environment Variables**で設定できます。

現時点では不要ですが、将来的に設定が必要になる場合:

```bash
# 例: Next.jsのビルドモード（Desktop App用）
NEXT_BUILD_MODE=export  # ※Vercelでは設定不要（デフォルトでSSR）
```

---

## 6. ビルドログの確認

デプロイに失敗した場合:

1. Vercelダッシュボード → **Deployments**
2. 失敗したデプロイをクリック
3. **Build Logs**でエラーを確認

---

## 7. 動作確認

### 7.1 Web-UIにアクセス

```
https://app.yourdomain.com
```

または

```
https://sankey-copier-web.vercel.app
```

### 7.2 relay-serverへの接続テスト

1. Web-UIのサイドバーから**Sites**を開く
2. **Add Site**をクリック
3. Cloudflare Tunnelで公開したrelay-serverを登録:
   - **Site Name**: `Production Server`
   - **Site URL**: `https://api.yourdomain.com`

4. サイト選択後、ダッシュボードでデータが表示されるか確認

---

## 8. トラブルシューティング

### ビルドエラー: `Module not found`

```bash
# web-uiディレクトリで依存関係を確認
cd web-ui
npm install
npm run build
```

ローカルでビルドが成功すれば、Vercelでも成功するはずです。

### 404エラー（ページが見つからない）

Next.jsのルーティング設定を確認:
- `web-ui/app/[locale]/layout.tsx`が存在するか
- `web-ui/app/[locale]/page.tsx`が存在するか

### CORS エラー

relay-serverの`config.toml`を確認:

```toml
[cors]
disable = false
additional_origins = ["https://app.yourdomain.com"]
```

### WebSocket接続失敗

1. Cloudflare Tunnel設定でWebSocketサポートが有効か確認
2. ブラウザの開発者ツール（Console）でエラーメッセージを確認

---

## 9. パフォーマンス最適化

### 9.1 Edge Functionsの活用

Vercelは自動的にEdge Functionsを活用し、グローバルに高速なレスポンスを提供します。

### 9.2 画像最適化

Next.jsの画像最適化は自動的に有効です（`next/image`使用時）。

### 9.3 静的ページのキャッシュ

頻繁に変更されないページは、Next.jsの`export`モードまたは`getStaticProps`で静的生成可能です。

---

## 10. コスト

### Vercel料金プラン

- **Hobby（無料）**:
  - 個人プロジェクト向け
  - 無制限のデプロイ
  - 100GB帯域幅/月
  - 十分な性能

- **Pro（$20/月）**:
  - チーム向け
  - パスワード保護機能
  - より高い帯域幅制限

**推奨**: Hobbyプランで開始し、必要に応じてアップグレード

---

## 11. 継続的デプロイメント

GitHub連携済みの場合:

```bash
# 変更をコミット
git add .
git commit -m "feat: update web-ui"

# mainブランチにプッシュ
git push origin main
```

数分後、Vercelが自動的に最新版をデプロイします。

### デプロイ通知

Vercel Slackアプリをインストールすると、デプロイ成功/失敗時に通知を受け取れます。

---

## 12. まとめ

Vercelデプロイの利点:
- ✅ 数分で本番環境にデプロイ可能
- ✅ 自動SSL証明書
- ✅ グローバルCDN
- ✅ GitHub連携で自動デプロイ
- ✅ 無料プランで十分な性能

次のステップ:
1. [CLOUDFLARE_SETUP.md](./CLOUDFLARE_SETUP.md)を参照し、relay-serverをCloudflare Tunnelで公開
2. Web-UIのSite機能でrelay-serverを登録
3. Cloudflare Accessで認証を設定

---

## 参考リンク

- [Vercel Documentation](https://vercel.com/docs)
- [Next.js Deployment](https://nextjs.org/docs/deployment)
- [Vercel CLI Reference](https://vercel.com/docs/cli)
