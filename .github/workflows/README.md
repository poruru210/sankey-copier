# GitHub Actions ビルドワークフロー

このディレクトリには、SANKEY Copierプロジェクトの自動ビルドを行うGitHub Actionsワークフローが含まれています。

## ワークフロー一覧

### `build.yml` - メインビルドワークフロー

手動実行（workflow_dispatch）により、プロジェクトの各コンポーネントをビルドします。

## 使用方法

### 1. GitHubリポジトリでの実行

1. GitHubリポジトリページにアクセス
2. 「Actions」タブをクリック
3. 左サイドバーから「Build SANKEY Copier」を選択
4. 「Run workflow」ボタンをクリック
5. ビルド対象を選択:
   - **all**: すべてのコンポーネントをビルド（推奨）
   - **rust-dll**: ZeroMQ DLLのみビルド
   - **rust-server**: Rustサーバーのみビルド
   - **web-ui**: Web UIのみビルド
   - **mql**: MQL4/MQL5ファイルのみコンパイル
6. 「Run workflow」をクリック

### 2. ビルド対象の詳細

#### `rust-dll` - Rust ZeroMQ DLL
- **ビルド内容**:
  - 32-bit版（MT4用）: `i686-pc-windows-msvc`
  - 64-bit版（MT5用）: `x86_64-pc-windows-msvc`
- **成果物**: `forex-copier-dll`
  - `MT4/sankey_copier_zmq.dll` (32-bit)
  - `MT5/sankey_copier_zmq.dll` (64-bit)
- **ランナー**: `windows-latest`

#### `rust-server` - Rust Server
- **ビルド内容**: メインのSANKEY Copierサーバー
- **成果物**: `forex-copier-server`
  - `forex-copier-server.exe`
- **ランナー**: `windows-latest`

#### `web-ui` - Web UI
- **ビルド内容**: Next.js 16 + Intlayer Web UI
- **ビルドツール**: pnpm
- **成果物**: `forex-copier-web-ui`
  - `.next/` ディレクトリ（本番ビルド）
- **ランナー**: `ubuntu-latest`

#### `mql` - MQL4/MQL5 Compilation
- **ビルド内容**: MT4/MT5 EAファイルのコンパイル
- **使用Action**: `fx31337/mql-compile-action`
- **成果物**:
  - `forex-copier-mql-MT4`: MT4のコンパイル済みEA（.ex4）
  - `forex-copier-mql-MT5`: MT5のコンパイル済みEA（.ex5）
- **ランナー**: `windows-latest`

#### `all` - Complete Build
上記すべてのジョブを実行し、最終的に統合リリースパッケージを作成します。

- **成果物**: `forex-copier-release-package`
  - すべてのコンポーネントを含むtar.gzアーカイブ

### 3. 成果物のダウンロード

1. ワークフロー実行が完了したら、実行詳細ページに移動
2. 下部の「Artifacts」セクションから必要なファイルをダウンロード
3. ダウンロードしたZIPファイルを解凍して使用

### 4. 保持期間

- 個別コンポーネント: 30日間
- リリースパッケージ: 90日間

## トラブルシューティング

### Rust DLLビルドエラー

**症状**: `cargo build`がZeroMQライブラリが見つからないというエラーで失敗

**対処法**:
- `mql-zmq-dll/Cargo.toml`の依存関係を確認
- ZeroMQの依存関係はRustの`zmq`クレートが自動的に処理するため、通常は問題なし

### MQLコンパイルエラー

**症状**: `fx31337/mql-compile-action`がファイルを見つけられない

**対処法**:
- `path`パラメータが正しいか確認（`mql/MT4`または`mql/MT5`）
- `include`パラメータが正しいか確認（`mql/Include`）

### Web UIビルドエラー

**症状**: pnpmの依存関係インストールエラー

**対処法**:
- `pnpm-lock.yaml`が最新か確認
- ローカルで`pnpm install`と`pnpm build`が成功するか確認

## ローカルでのテスト

ワークフローをプッシュする前に、各コンポーネントをローカルでビルドしてテストすることを推奨します：

```bash
# Rust DLL
cd mql-zmq-dll
cargo build --release --target i686-pc-windows-msvc
cargo build --release --target x86_64-pc-windows-msvc

# Rust Server
cd rust-server
cargo build --release

# Web UI
cd web-ui
pnpm install
pnpm build

# MQL（MetaEditorを使用）
# MT4/MT5のMetaEditorで各.mq4/.mq5ファイルをコンパイル
```

## カスタマイズ

### ビルドターゲットの追加

新しいRustターゲット（例: Linux版）を追加する場合:

```yaml
- name: Build Linux Server
  working-directory: rust-server
  run: |
    rustup target add x86_64-unknown-linux-gnu
    cargo build --release --target x86_64-unknown-linux-gnu
```

### キャッシュの調整

ビルド時間を短縮するため、Rustキャッシュとpnpmキャッシュを使用しています。
キャッシュをクリアしたい場合は、GitHubのActionsキャッシュ管理から削除できます。

## 継続的インテグレーション（CI）の追加

現在は手動実行のみですが、自動実行を有効にする場合は`on`セクションを変更：

```yaml
on:
  push:
    branches: [ master, develop ]
  pull_request:
    branches: [ master ]
  workflow_dispatch:
    inputs:
      # ...
```

## セキュリティ

- GitHub Secretsに機密情報を保存しないでください（現在は不要）
- 将来、署名やデプロイを追加する場合は、適切にSecretsを使用してください

## 参考リンク

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Rust Toolchain Action](https://github.com/dtolnay/rust-toolchain)
- [MQL Compile Action](https://github.com/FX31337/MQL-Compile-Action)
- [pnpm Action](https://github.com/pnpm/action-setup)
