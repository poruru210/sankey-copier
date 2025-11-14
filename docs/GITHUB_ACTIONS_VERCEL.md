# GitHub Actions経由でのVercel自動デプロイ設定

このガイドでは、GitHub Actionsを使用してWeb-UIを自動的にVercelにデプロイする方法を説明します。

## 概要

```
git push → GitHub Actions → Vercel Deploy → 本番環境
```

- **mainブランチへのpush**: 本番環境（Production）へ自動デプロイ
- **Pull Request作成時**: プレビュー環境（Preview）へ自動デプロイ

---

## 前提条件

- GitHubリポジトリ
- Vercelアカウント（無料プランでOK）
- Vercelプロジェクトが作成済み

---

## 1. Vercelトークンの取得

### 1.1 Vercel Access Tokenの作成

1. [Vercel Dashboard](https://vercel.com/account/tokens)にアクセス
2. **Create Token**をクリック
3. トークン名を入力（例: `GitHub Actions Deploy`）
4. **Scope**: `Full Account`を選択（または特定プロジェクトのみ）
5. **Expiration**: お好みで設定（推奨: `No Expiration`または`1 year`）
6. **Create**をクリック
7. 表示されたトークンを**コピー**（一度しか表示されません）

**例:**
```
vercel_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
```

---

## 2. Vercel Project IDとOrg IDの取得

### 2.1 Vercel CLIで取得（推奨）

```bash
cd web-ui

# Vercel CLIをインストール（まだの場合）
npm install -g vercel

# Vercelにログイン
vercel login

# プロジェクトをリンク
vercel link

# Project IDとOrg IDが .vercel/project.json に保存されます
cat .vercel/project.json
```

出力例:
```json
{
  "orgId": "team_xxxxxxxxxxxxxxxxxxxx",
  "projectId": "prj_xxxxxxxxxxxxxxxxxxxx"
}
```

### 2.2 Vercel Dashboardから取得（代替方法）

1. [Vercel Dashboard](https://vercel.com/dashboard)を開く
2. プロジェクトを選択
3. **Settings** → **General**
4. **Project ID**をコピー
5. **Team ID**（または**Personal Account ID**）をコピー

---

## 3. GitHubシークレットの設定

### 3.1 GitHubリポジトリでシークレットを追加

1. GitHubリポジトリを開く
2. **Settings** → **Secrets and variables** → **Actions**
3. **New repository secret**をクリック

以下の3つのシークレットを追加:

| Name | Value | 説明 |
|------|-------|------|
| `VERCEL_TOKEN` | `vercel_xxx...` | 手順1で取得したアクセストークン |
| `VERCEL_ORG_ID` | `team_xxx...` | 手順2で取得したOrg ID |
| `VERCEL_PROJECT_ID` | `prj_xxx...` | 手順2で取得したProject ID |

**重要**: シークレット名は正確に入力してください（大文字小文字を区別します）。

---

## 4. GitHub Actionsワークフローの確認

リポジトリには既に`.github/workflows/deploy-vercel.yml`が含まれています。

### 4.1 ワークフローの動作

**トリガー:**
- `main`ブランチへのpush時（web-uiディレクトリの変更時のみ）
- Pull Request作成/更新時（web-uiディレクトリの変更時のみ）

**デプロイ先:**
- **mainブランチ**: Production環境
- **Pull Request**: Preview環境（PRごとに一時的なURL）

### 4.2 ワークフローの内容

```yaml
name: Deploy to Vercel

on:
  push:
    branches:
      - main
    paths:
      - 'web-ui/**'
  pull_request:
    branches:
      - main
    paths:
      - 'web-ui/**'

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - Checkout code
      - Setup Node.js
      - Install Vercel CLI
      - Pull Vercel環境情報
      - Build
      - Deploy to Vercel
      - PRにデプロイURLをコメント（Preview時のみ）
```

---

## 5. デプロイの実行

### 5.1 初回デプロイ

1. **変更をコミット**
   ```bash
   cd web-ui
   # 何か変更を加える（例: README更新）
   echo "# Test" >> README.md
   git add .
   git commit -m "test: Trigger Vercel deployment"
   ```

2. **mainブランチにプッシュ**
   ```bash
   git push origin main
   ```

3. **GitHub Actionsを確認**
   - GitHubリポジトリの**Actions**タブを開く
   - "Deploy to Vercel"ワークフローが実行中
   - ログでデプロイURLを確認

4. **デプロイ完了**
   - ✅ ワークフローが成功すると、Production環境にデプロイされます
   - URLは`https://your-project.vercel.app`

### 5.2 Pull Requestでのプレビューデプロイ

1. **新しいブランチを作成**
   ```bash
   git checkout -b feature/update-ui
   ```

2. **変更をコミット**
   ```bash
   # web-uiに変更を加える
   git add .
   git commit -m "feat: Update UI design"
   git push origin feature/update-ui
   ```

3. **Pull Requestを作成**
   - GitHubでPRを作成
   - GitHub ActionsがPreview環境に自動デプロイ
   - PR内にデプロイURLがコメントされます

**PRコメント例:**
```
✅ Preview deployment ready!

🔗 URL: https://your-project-git-feature-update-ui.vercel.app
```

---

## 6. トラブルシューティング

### エラー: "Error: No token provided"

**原因**: `VERCEL_TOKEN`シークレットが設定されていない

**解決方法**:
1. GitHubリポジトリの**Settings** → **Secrets**を確認
2. `VERCEL_TOKEN`が存在するか確認
3. 存在しない場合、手順3で再設定

### エラー: "Error: Project not found"

**原因**: `VERCEL_PROJECT_ID`または`VERCEL_ORG_ID`が間違っている

**解決方法**:
1. `.vercel/project.json`の内容を再確認
2. Vercel Dashboardで正しいIDを確認
3. GitHubシークレットを更新

### エラー: "Error: Invalid token"

**原因**: トークンが無効または期限切れ

**解決方法**:
1. Vercel Dashboardで新しいトークンを作成
2. GitHubシークレット`VERCEL_TOKEN`を更新

### ワークフローがトリガーされない

**原因**: `web-ui/**`以外のファイルのみ変更された

**解決方法**:
- `web-ui`ディレクトリ内のファイルを変更してpush
- またはワークフローの`paths`設定を調整

### デプロイは成功するが、サイトが表示されない

**原因**: ビルドエラーまたは設定ミス

**解決方法**:
1. GitHub Actionsのログを確認
2. Vercel Dashboardの**Deployments**でログを確認
3. ローカルで`npm run build`を実行してエラーを確認

---

## 7. 高度な設定

### 7.1 環境変数の追加

Vercel環境変数をGitHub Actionsから設定する場合:

```yaml
- name: Build Project Artifacts
  working-directory: web-ui
  env:
    NEXT_PUBLIC_API_URL: ${{ secrets.API_URL }}
  run: vercel build --prod --token=${{ secrets.VERCEL_TOKEN }}
```

### 7.2 カスタムドメインの設定

Vercel Dashboardで設定:
1. **Settings** → **Domains**
2. カスタムドメインを追加
3. DNS設定（Cloudflareの場合、CNAMEレコードを追加）

GitHub Actionsは自動的にカスタムドメインにもデプロイします。

### 7.3 複数ブランチのデプロイ

`develop`ブランチもデプロイする場合:

```yaml
on:
  push:
    branches:
      - main
      - develop  # 追加
```

---

## 8. デプロイステータスバッジの追加

README.mdにデプロイステータスを表示:

```markdown
![Deploy to Vercel](https://github.com/your-org/sankey-copier/actions/workflows/deploy-vercel.yml/badge.svg)
```

---

## 9. コスト

- **GitHub Actions**: 無料プランで月2,000分（このワークフローは1回約2-3分）
- **Vercel**: 無料プランで無制限デプロイ（商用利用の場合はProプラン推奨）

---

## 10. セキュリティのベストプラクティス

### 10.1 トークンの管理

- ❌ トークンをコードにコミットしない
- ✅ GitHubシークレットを使用
- ✅ 必要最小限のスコープでトークン作成
- ✅ 定期的にトークンをローテーション

### 10.2 ブランチ保護

mainブランチを保護:
1. **Settings** → **Branches** → **Add rule**
2. **Branch name pattern**: `main`
3. **Require status checks to pass**: チェック
4. **Require deployments to succeed**: チェック

---

## 11. まとめ

GitHub Actionsによる自動デプロイの利点:

- ✅ **自動化**: pushするだけで自動デプロイ
- ✅ **PR Preview**: レビュー前に変更を確認可能
- ✅ **履歴管理**: すべてのデプロイがGitHub Actionsログに記録
- ✅ **ロールバック**: Vercel Dashboardで簡単にロールバック可能
- ✅ **無料**: GitHub + Vercel無料プランで完全に運用可能

---

## 参考リンク

- [Vercel CLI Documentation](https://vercel.com/docs/cli)
- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Vercel Deployments with GitHub Actions](https://vercel.com/guides/how-can-i-use-github-actions-with-vercel)
