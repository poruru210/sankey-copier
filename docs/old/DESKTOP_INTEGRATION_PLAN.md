# Desktop App + relay-server 統合実装計画

## 目標

Desktop App（Tauri）とrelay-serverを統合し、単一のインストーラーで配布できるようにする。

## アーキテクチャ

### 統合前（現状）

```
【別々にインストール・起動が必要】

1. relay-server (手動起動)
   └─ cargo run --release

2. Desktop App (別途起動)
   └─ web-ui（静的HTML）を表示
```

### 統合後（目標）

```
【Desktop App 1つで完結】

Desktop App (Tauri)
  ├─ web-ui (静的HTML/CSS/JS)
  └─ relay-server (Tauri sidecar)
       ├─ 自動起動（Desktop App起動時）
       ├─ 自動停止（Desktop App終了時）
       └─ localhost:3000でHTTP/WebSocket提供
```

---

## 実装ステップ

### ステップ1: relay-serverのsidecarビルド設定

#### 1.1 `relay-server/Cargo.toml`の修正

```toml
# 既存の[package]セクションは変更なし

[profile.release]
lto = true        # Link Time Optimization
codegen-units = 1 # シングルスレッド最適化
strip = true      # デバッグシンボル削除（サイズ削減）
opt-level = "z"   # サイズ最適化
```

**目的**: バイナリサイズを最小化（Desktop Appに同梱するため）

#### 1.2 relay-serverのビルドスクリプト作成

`relay-server/build-sidecar.ps1`:

```powershell
# Tauri sidecar用のrelay-serverビルドスクリプト

Write-Host "Building relay-server for Tauri sidecar..."

# リリースビルド
cargo build --release --target x86_64-pc-windows-msvc

# 出力先
$SOURCE = "target/x86_64-pc-windows-msvc/release/relay-server.exe"
$DEST = "../desktop-app/src-tauri/binaries"

# ディレクトリ作成
New-Item -ItemType Directory -Force -Path $DEST | Out-Null

# Tauri命名規則に従ってコピー
# 形式: {binary-name}-{target-triple}.exe
Copy-Item $SOURCE "$DEST/relay-server-x86_64-pc-windows-msvc.exe"

Write-Host "✅ Sidecar binary copied to: $DEST"
```

---

### ステップ2: Tauri設定の更新

#### 2.1 `desktop-app/src-tauri/tauri.conf.json`にsidecar追加

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "SANKEY Copier",
  "version": "1.0.0",
  "identifier": "com.sankey.copier.desktop",
  "build": {
    "beforeDevCommand": "cd ../../web-ui && pnpm run dev",
    "devUrl": "http://localhost:8080",
    "beforeBuildCommand": "cd ../../web-ui && NEXT_BUILD_MODE=export pnpm build",
    "frontendDist": "../../web-ui/out"
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": ["icons/icon.ico"],
    "externalBin": [
      "binaries/relay-server"
    ],
    "windows": {
      "certificateThumbprint": null,
      "digestAlgorithm": "sha256",
      "timestampUrl": ""
    }
  },
  "app": {
    "windows": [
      {
        "title": "SANKEY Copier",
        "width": 1200,
        "height": 800,
        "resizable": true,
        "fullscreen": false,
        "url": "http://localhost:3000"
      }
    ],
    "security": {
      "csp": null,
      "assetProtocol": {
        "scope": ["**"]
      }
    }
  }
}
```

**変更点:**
1. `bundle.externalBin`: relay-serverをsidecarとして登録
2. `beforeBuildCommand`: web-uiをexportモードでビルド
3. `app.windows.url`: relay-serverのURL（localhost:3000）を指定

---

### ステップ3: Desktop Appのrelay-server起動ロジック

#### 3.1 `desktop-app/src-tauri/src/main.rs`の実装

```rust
// SANKEY Copier Desktop Application
// Tauri-based desktop app with integrated relay-server sidecar

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{Manager, AppHandle};
use tauri_plugin_shell::ShellExt;
use std::time::Duration;
use std::net::TcpStream;

/// サーバーが起動するまで待機（最大30秒）
fn wait_for_server(port: u16, max_attempts: usize) -> bool {
    println!("Waiting for relay-server on port {}...", port);

    for i in 0..max_attempts {
        if TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok() {
            println!("✅ relay-server is ready!");
            return true;
        }

        std::thread::sleep(Duration::from_secs(1));
        if i % 5 == 0 {
            println!("Still waiting... ({}/{})", i, max_attempts);
        }
    }

    eprintln!("❌ relay-server failed to start within {} seconds", max_attempts);
    false
}

/// relay-serverのsidecarを起動
fn start_rust_server(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting relay-server sidecar...");

    // Tauri sidecarコマンドを取得
    let sidecar_command = app.shell()
        .sidecar("relay-server")?;

    // relay-serverを起動（バックグラウンド）
    let (_rx, _child) = sidecar_command.spawn()?;

    println!("relay-server process spawned");

    // サーバーが起動するまで待機
    if !wait_for_server(3000, 30) {
        return Err("Failed to start relay-server".into());
    }

    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // relay-serverを起動
            if let Err(e) = start_rust_server(&app.handle()) {
                eprintln!("Failed to start relay-server: {}", e);
                // エラーダイアログを表示してアプリを終了
                std::process::exit(1);
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Error while running Tauri application");
}
```

**ポイント:**
1. `start_rust_server()`: sidecarとしてrelay-serverを起動
2. `wait_for_server()`: サーバーが起動するまで待機（ポーリング）
3. エラーハンドリング: relay-server起動失敗時はアプリ終了

---

### ステップ4: ビルドプロセスの自動化

#### 4.1 `desktop-app/build-all.ps1`の作成

```powershell
# Desktop App + relay-server 統合ビルドスクリプト

param(
    [switch]$Release
)

$ErrorActionPreference = "Stop"

Write-Host "=====================================" -ForegroundColor Cyan
Write-Host "Building SANKEY Copier Desktop App" -ForegroundColor Cyan
Write-Host "=====================================" -ForegroundColor Cyan

# 1. relay-serverをsidecar用にビルド
Write-Host "`n[1/3] Building relay-server sidecar..." -ForegroundColor Yellow
Push-Location ../relay-server
& .\build-sidecar.ps1
if ($LASTEXITCODE -ne 0) { throw "relay-server build failed" }
Pop-Location

# 2. web-uiを静的エクスポートモードでビルド
Write-Host "`n[2/3] Building web-ui (static export)..." -ForegroundColor Yellow
Push-Location ../web-ui
$env:NEXT_BUILD_MODE = "export"
pnpm install
pnpm build
if ($LASTEXITCODE -ne 0) { throw "web-ui build failed" }
Pop-Location

# 3. Tauri Desktop Appをビルド
Write-Host "`n[3/3] Building Tauri Desktop App..." -ForegroundColor Yellow
pnpm install

if ($Release) {
    pnpm tauri build
} else {
    pnpm tauri build --debug
}

if ($LASTEXITCODE -ne 0) { throw "Tauri build failed" }

Write-Host "`n✅ Build completed!" -ForegroundColor Green
Write-Host "Output: src-tauri/target/release/sankey-copier-desktop.exe" -ForegroundColor Green
```

---

### ステップ5: GitHub Actions統合

#### 5.1 `.github/workflows/build-desktop.yml`の更新

```yaml
name: Build Desktop App

on:
  push:
    branches: [main]
    paths:
      - 'desktop-app/**'
      - 'relay-server/**'
      - 'web-ui/**'

jobs:
  build-desktop:
    runs-on: windows-latest

    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: x86_64-pc-windows-msvc

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'

      - name: Install pnpm
        uses: pnpm/action-setup@v2
        with:
          version: 8

      - name: Build relay-server (sidecar)
        working-directory: relay-server
        run: |
          cargo build --release --target x86_64-pc-windows-msvc
          mkdir -p ../desktop-app/src-tauri/binaries
          cp target/x86_64-pc-windows-msvc/release/relay-server.exe ../desktop-app/src-tauri/binaries/relay-server-x86_64-pc-windows-msvc.exe

      - name: Build web-ui (static export)
        working-directory: web-ui
        env:
          NEXT_BUILD_MODE: export
        run: |
          pnpm install
          pnpm build

      - name: Build Tauri Desktop App
        working-directory: desktop-app
        run: |
          pnpm install
          pnpm tauri build

      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: sankey-copier-desktop-windows
          path: desktop-app/src-tauri/target/release/bundle/nsis/*.exe
```

---

## 利点

### ユーザー視点
1. **簡単インストール**: 1つのEXEをダブルクリックするだけ
2. **依存関係ゼロ**: Node.jsやRustのインストール不要
3. **自動起動**: Desktop App起動時にrelay-serverも自動起動

### 開発者視点
1. **メンテナンス容易**: 既存のrelay-serverコードはほぼ変更なし
2. **標準機能**: Tauriの公式sidecar機能を使用
3. **クロスプラットフォーム**: Linux/macOSにも対応可能

---

## 課題と対策

### 課題1: relay-serverがすでに起動している場合

**対策**: 起動前にポート3000をチェック

```rust
fn is_port_available(port: u16) -> bool {
    TcpStream::connect(format!("127.0.0.1:{}", port)).is_err()
}

// Desktop App起動時
if is_port_available(3000) {
    start_rust_server(app)?;
} else {
    println!("⚠️ relay-server is already running on port 3000");
}
```

### 課題2: relay-server設定ファイルの場所

**対策**: sidecar起動時にカレントディレクトリを指定

```rust
let sidecar_command = app.shell()
    .sidecar("relay-server")?
    .current_dir(get_config_dir()?);  // 設定ファイルディレクトリ
```

### 課題3: バイナリサイズの肥大化

**対策**:
- relay-serverのリリースビルド最適化（LTO、strip）
- UPXなどの圧縮ツール（オプション）

---

## テスト計画

### 単体テスト

1. **relay-server sidecar起動テスト**
   ```bash
   cd desktop-app
   pnpm tauri dev
   # → relay-serverが自動起動するか確認
   ```

2. **ポート競合テスト**
   ```bash
   # 先にrelay-serverを手動起動
   cd relay-server && cargo run

   # Desktop Appを起動
   cd desktop-app && pnpm tauri dev
   # → エラーメッセージが表示されるか確認
   ```

3. **終了時のクリーンアップテスト**
   ```bash
   # Desktop Appを起動→終了
   # タスクマネージャーでrelay-serverプロセスが残っていないか確認
   ```

### 統合テスト

1. **EA接続テスト**
   - MT4/MT5のEAをアタッチ
   - Desktop App経由でトレード設定
   - コピーが正常に動作するか確認

2. **WebSocket接続テスト**
   - Desktop App起動
   - リアルタイム更新が動作するか確認

---

## ロールアウト計画

### Phase 1: プロトタイプ（1週間）
- [ ] Tauri sidecar基本実装
- [ ] ローカルビルド成功

### Phase 2: 安定化（1週間）
- [ ] エラーハンドリング
- [ ] ポート競合対策
- [ ] GitHub Actions統合

### Phase 3: リリース（1週間）
- [ ] ドキュメント更新
- [ ] インストーラー作成
- [ ] ユーザーテスト

---

## 参考資料

- [Tauri Sidecar Documentation](https://v2.tauri.app/develop/sidecar/)
- [Tauri Plugin Shell](https://v2.tauri.app/plugin/shell/)
- [Next.js Static Export](https://nextjs.org/docs/pages/building-your-application/deploying/static-exports)
