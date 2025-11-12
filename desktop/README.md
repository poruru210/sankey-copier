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

1. アプリ起動時に空きポートを自動検出
2. システムの`node`コマンドでweb-ui standaloneサーバーを起動
3. 検出されたポートでTauriウィンドウを開く
4. web-uiはlocalhost:3000（rust-server）に接続

## 成果物

- **Windows**: `src-tauri/target/release/bundle/msi/*.msi`
- **実行ファイル**: `src-tauri/target/release/sankey-copier-desktop.exe`
