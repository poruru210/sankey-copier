# SANKEY Copier Desktop Application

Tauriベースのデスクトップアプリケーション。

## 前提条件

- **Node.js**: システムにNode.jsがインストールされている必要があります
- **Rust**: Cargo and Rustツールチェーン
- **web-ui**: Next.js standalone ビルドが完了していること

## 開発

```bash
# 依存関係のインストール
pnpm install

# 開発モードで起動
pnpm tauri dev
```

## ビルド

```bash
# 本番ビルド
pnpm tauri build

# デバッグビルド
pnpm tauri build --debug
```

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
