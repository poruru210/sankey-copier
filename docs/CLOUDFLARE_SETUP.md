# Cloudflare Setup Guide

このガイドでは、Cloudflare TunnelとAccessを使用して、イントラネット内のrust-serverを安全にインターネットに公開する方法を説明します。

## アーキテクチャ概要

```
User
  ↓
Cloudflare Access (認証)
  ↓
┌──────────────────────┬──────────────────────┐
│                      │                      │
↓                      ↓                      ↓
Vercel                 Cloudflare Tunnel      Cloudflare DNS
(web-ui)              (rust-server)
                       ↓
                  イントラ内サーバー
                  (localhost:3000)
```

## 前提条件

- Cloudflareアカウント（無料プランでOK）
- ドメイン管理がCloudflareに移管済み
- イントラネット内でrust-serverが起動している（localhost:3000）

---

## 1. Cloudflare Tunnelのセットアップ

### 1.1 cloudflaredのインストール

**Windows:**
```powershell
# PowerShellで実行
Invoke-WebRequest -Uri "https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-windows-amd64.exe" -OutFile "cloudflared.exe"

# インストール
.\cloudflared.exe service install
```

**Linux:**
```bash
# Debianベース
wget https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64.deb
sudo dpkg -i cloudflared-linux-amd64.deb

# Red Hatベース
sudo rpm -i https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-x86_64.rpm
```

**macOS:**
```bash
brew install cloudflared
```

### 1.2 Cloudflareにログイン

```bash
cloudflared tunnel login
```

ブラウザが開くので、Cloudflareアカウントでログインし、ドメインを選択します。

### 1.3 トンネルの作成

```bash
# トンネル作成
cloudflared tunnel create sankey-server

# 成功すると以下が表示されます:
# Tunnel credentials written to /path/to/.cloudflared/<TUNNEL-ID>.json
# Created tunnel sankey-server with id <TUNNEL-ID>
```

**重要**: `<TUNNEL-ID>`をメモしてください。

### 1.4 設定ファイルの作成

`~/.cloudflared/config.yml`（Windowsの場合は`C:\Users\<USERNAME>\.cloudflared\config.yml`）を作成:

```yaml
tunnel: <TUNNEL-ID>
credentials-file: /path/to/.cloudflared/<TUNNEL-ID>.json

ingress:
  # rust-serverへのルーティング
  - hostname: api.yourdomain.com
    service: http://localhost:3000

  # 404ページ（必須）
  - service: http_status:404
```

**設定のポイント:**
- `hostname`: あなたのドメインのサブドメイン（例: `api.example.com`）
- `service`: ローカルのrust-serverのURL（`http://localhost:3000`）

### 1.5 DNSルーティングの設定

```bash
cloudflared tunnel route dns sankey-server api.yourdomain.com
```

これにより、`api.yourdomain.com` → Cloudflare Tunnel → `localhost:3000`のルーティングが作成されます。

### 1.6 トンネルの起動

**手動起動:**
```bash
cloudflared tunnel run sankey-server
```

**Windowsサービスとして起動:**
```powershell
# サービスとしてインストール
cloudflared service install

# サービス開始
net start cloudflared
```

**Linuxのsystemdサービス:**
```bash
sudo cloudflared service install
sudo systemctl start cloudflared
sudo systemctl enable cloudflared
```

---

## 2. Cloudflare Accessのセットアップ（認証）

Cloudflare Accessを使用して、rust-serverへのアクセスを認証で保護します。

### 2.1 Zero Trustダッシュボードにアクセス

1. [Cloudflare Dashboard](https://dash.cloudflare.com/)にログイン
2. 左メニューから「Zero Trust」を選択
3. 初回の場合、チーム名を設定（例: `yourcompany`）

### 2.2 Identity Providerの設定

1. **Settings** → **Authentication** → **Login methods**に移動
2. 認証方法を追加:
   - **One-time PIN** (メールアドレス) - 無料、最もシンプル
   - **Google** - Googleアカウントでログイン
   - **GitHub** - GitHubアカウントでログイン
   - その他（Azure AD、Okta等）

**推奨**: GoogleまたはGitHub（設定が簡単で安全）

### 2.3 Access Policyの作成（rust-server用）

1. **Access** → **Applications**に移動
2. **Add an application**をクリック
3. **Self-hosted**を選択

**Application設定:**
- **Application name**: `Sankey Copier API`
- **Session Duration**: `24 hours`（お好みで調整）
- **Application domain**: `api.yourdomain.com`

**Policy設定:**
- **Policy name**: `Allow authorized users`
- **Action**: `Allow`
- **Include**:
  - **Emails**: 許可するメールアドレス（例: `user@example.com`）
  - または **Emails ending in**: 特定ドメイン（例: `@yourcompany.com`）

**Create application**をクリック。

### 2.4 WebSocket対応の有効化

rust-serverはWebSocketを使用するため、以下を確認:

1. 作成したアプリケーション設定を開く
2. **Advanced settings** → **Additional settings**
3. **WebSocket support**: `Enabled`（デフォルトで有効）

---

## 3. Vercel + Cloudflareの統合（web-ui用）

### 3.1 VercelにWeb-UIをデプロイ

```bash
cd web-ui

# Vercel CLIインストール（初回のみ）
npm i -g vercel

# デプロイ
vercel --prod
```

### 3.2 Vercelのカスタムドメイン設定

1. Vercelダッシュボードでプロジェクトを開く
2. **Settings** → **Domains**
3. カスタムドメインを追加（例: `app.yourdomain.com`）
4. 表示されたDNS設定を**Cloudflare DNS**に追加

### 3.3 Cloudflare DNSの設定

Cloudflare Dashboardで:

1. **DNS** → **Records**に移動
2. 以下のレコードを追加:

```
Type: CNAME
Name: app
Content: cname.vercel-dns.com
Proxy status: Proxied (オレンジ色のクラウド)
```

### 3.4 VercelアプリをCloudflare Accessで保護

**Option A: サブドメイン全体を保護**

1. Zero Trust → **Access** → **Applications**
2. **Add an application** → **Self-hosted**
3. 設定:
   - **Application domain**: `app.yourdomain.com`
   - **Policy**: rust-serverと同じポリシーを使用

**Option B: Vercel側で認証（推奨）**

web-uiは公開し、rust-server（機密データ）のみAccessで保護する方が柔軟性が高い場合があります。

---

## 4. Web-UIのSite機能でrust-serverを登録

Cloudflare Tunnelで公開したrust-serverをweb-uiに登録します。

1. ブラウザで`https://app.yourdomain.com`にアクセス
2. サイドバーから**Sites**ページに移動
3. **Add Site**をクリック
4. 以下を入力:
   - **Site Name**: `Production Server`
   - **Site URL**: `https://api.yourdomain.com`（Cloudflare Tunnel経由）

5. **Save**をクリック

初回アクセス時、Cloudflare Accessの認証画面が表示されます。設定したIdentity Provider（Google/GitHub等）でログインしてください。

---

## 5. 動作確認

### 5.1 Tunnelの状態確認

```bash
cloudflared tunnel info sankey-server
```

### 5.2 rust-serverへのアクセステスト

ブラウザで`https://api.yourdomain.com/api/settings`にアクセス:

1. Cloudflare Accessのログイン画面が表示される
2. 認証後、rust-serverのレスポンスが表示される

### 5.3 WebSocket接続テスト

Web-UIから:

1. ダッシュボードを開く
2. リアルタイム更新が動作するか確認（WebSocket接続）

---

## 6. トラブルシューティング

### Tunnel接続エラー

```bash
# ログ確認
cloudflared tunnel info sankey-server

# トンネルを再起動
cloudflared tunnel run sankey-server
```

### Accessログイン失敗

1. Zero Trust → **Logs** → **Access**でログを確認
2. ポリシーが正しく設定されているか確認
3. メールアドレス/ドメインが許可リストに含まれているか確認

### WebSocket接続失敗

1. Cloudflare Access設定で**WebSocket support**が有効か確認
2. rust-serverのログを確認:
   ```bash
   # rust-serverのログディレクトリ
   tail -f logs/sankey-copier-*.log
   ```

### CORSエラー

Cloudflare Accessを使用する場合、CORSは自動的に処理されます。エラーが出る場合:

1. `rust-server/config.toml`で`cors.disable = false`に設定
2. `cors.additional_origins`に`https://app.yourdomain.com`を追加

---

## 7. セキュリティのベストプラクティス

### 7.1 最小権限の原則

Accessポリシーで必要最小限のユーザーのみ許可:

```yaml
# 例: 特定のメールアドレスのみ
Include:
  - user1@example.com
  - user2@example.com
```

### 7.2 セッションタイムアウトの設定

- 推奨: 24時間以内
- 機密性が高い場合: 1時間

### 7.3 監査ログの確認

定期的にZero Trust → **Logs** → **Access**でアクセスログを確認。

### 7.4 IPホワイトリスト（オプション）

さらなる保護が必要な場合、特定IPからのみアクセスを許可:

Access Policy → **Include** → **IP ranges**

---

## 8. コスト

- **Cloudflare Tunnel**: 無料
- **Cloudflare Access**: 無料プラン（最大50ユーザー）
- **Vercel**: Hobbyプラン無料、Proプラン$20/月（必要に応じて）

---

## 参考リンク

- [Cloudflare Tunnel Documentation](https://developers.cloudflare.com/cloudflare-one/connections/connect-apps/)
- [Cloudflare Access Documentation](https://developers.cloudflare.com/cloudflare-one/policies/access/)
- [Vercel Custom Domains](https://vercel.com/docs/concepts/projects/custom-domains)

---

## まとめ

この設定により:
- ✅ イントラネット内のrust-serverがインターネットからアクセス可能
- ✅ Cloudflare Accessによる強固な認証
- ✅ WebSocket通信も自動的に保護
- ✅ 固定IPアドレス不要
- ✅ 無料（Cloudflare無料プラン + Vercel無料プラン）

設定完了後、web-uiのSite機能で複数のrust-serverを登録・切り替え可能になります。
