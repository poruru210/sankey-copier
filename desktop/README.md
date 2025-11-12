# SANKEY Copier Desktop Application

Tauriベースのデスクトップアプリケーション。

## 前提条件

- **Node.js**: システムにNode.jsがインストールされている必要があります（実行時にも必要）
- **Rust**: Cargo and Rustツールチェーン（ビルド時のみ）
- **pnpm**: web-uiのビルドに使用（ビルド時のみ）

## 開発

```bash
# 依存関係のインストール
pnpm install

# 開発モードで起動
pnpm tauri dev
```

## ビルド

ビルド時に`beforeBuildCommand`で自動的にweb-uiがビルドされ、バンドルされます。

```bash
# Windows (PowerShell)
# 依存関係のインストール
cd desktop
pnpm install

# 本番ビルド
pnpm tauri build

# デバッグビルド
pnpm tauri build --debug
```

```bash
# Linux/macOS
# 依存関係のインストール
cd desktop
pnpm install

# 本番ビルド
pnpm tauri build

# デバッグビルド
pnpm tauri build --debug
```

### ビルドプロセス

1. `tauri build`実行時に`beforeBuildCommand`が自動的に実行されます
2. `prepare-web-ui.js`（Node.jsスクリプト）が実行されます
   - Bash版（`prepare-web-ui.sh`）とPowerShell版（`prepare-web-ui.ps1`）も利用可能
3. web-uiのNext.jsスタンドアロンビルドが生成されます
4. スタンドアロンビルドが`desktop/web-ui/`ディレクトリにコピーされます
   - `server.js` - Next.jsサーバー
   - `node_modules/` - 必要な依存関係のみ
   - `.next/static/` - 静的アセット
   - `public/` - 公開ファイル
5. Tauriがweb-uiディレクトリをアプリケーションにバンドルします

## 動作

1. アプリ起動時にスプラッシュスクリーンを表示
2. バックグラウンドで空きポートを自動検出
3. システムの`node`コマンドでweb-ui standaloneサーバーを起動
4. サーバー準備完了後、スプラッシュを閉じてメインウィンドウを表示
5. web-uiはlocalhost:3000（rust-server）に接続

### エラーハンドリング

- Node.jsが見つからない場合: エラーメッセージを表示
- ポートが見つからない場合: エラーメッセージを表示
- サーバー起動に失敗した場合: 詳細なエラーを表示

## 成果物

- **Windows**: `src-tauri/target/release/bundle/msi/*.msi`
- **実行ファイル**: `src-tauri/target/release/sankey-copier-desktop.exe`
