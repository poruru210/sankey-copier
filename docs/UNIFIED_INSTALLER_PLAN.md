# 統合インストーラー実装計画

## 目標

rust-server + Desktop App + MT4/MT5コンポーネントを**1つのインストーラー**で配布し、利用者の利便性を向上させる。

## 要件

### 機能要件

1. **1つのインストーラーEXE**で以下をインストール:
   - rust-server.exe
   - Desktop App（sankey-copier-desktop.exe）
   - MT4/MT5用DLL/EA

2. **Windowsサービス登録**:
   - rust-serverをWindowsサービスとして登録
   - システム起動時に自動起動

3. **デスクトップショートカット**:
   - Desktop App用のショートカット
   - スタートメニューにも登録

4. **アンインストール機能**:
   - Windowsの「プログラムと機能」から削除可能
   - サービス削除も含む

### 非機能要件

- Windows 10/11 (64-bit)
- 管理者権限で実行
- インストールサイズ: ~50MB

---

## アーキテクチャ

### インストール後のディレクトリ構成

```
C:\Program Files\SANKEY Copier\
├── rust-server.exe              # サーバー本体（24/7稼働）
├── sankey-copier-desktop.exe    # Desktop App（必要時のみ起動）
├── config.toml                  # rust-server設定ファイル
├── mql/                         # MT4/MT5コンポーネント
│   ├── mt4/
│   │   ├── Experts/
│   │   │   ├── SankeyCopierMaster.ex4
│   │   │   └── SankeyCopierSlave.ex4
│   │   └── Libraries/
│   │       └── zmq.dll
│   └── mt5/
│       ├── Experts/
│       │   ├── SankeyCopierMaster.ex5
│       │   └── SankeyCopierSlave.ex5
│       └── Libraries/
│           └── zmq.dll
└── unins000.exe                 # アンインストーラー
```

### Windowsサービス

- **サービス名**: `SankeyCopierServer`
- **表示名**: `SANKEY Copier Server`
- **起動タイプ**: 自動
- **実行ファイル**: `C:\Program Files\SANKEY Copier\rust-server.exe`

### デスクトップショートカット

- **名前**: `SANKEY Copier`
- **ターゲット**: `C:\Program Files\SANKEY Copier\sankey-copier-desktop.exe`
- **アイコン**: 同梱のapp.ico

---

## 実装方法: Inno Setup

### Inno Setupの選定理由

✅ **メリット**:
1. 無料・オープンソース
2. Windowsインストーラーの業界標準
3. Windowsサービス登録をサポート
4. スクリプトが読みやすい
5. コード署名対応

❌ **他の選択肢を却下した理由**:
- **WiX Toolset**: XMLが複雑、学習コストが高い
- **NSIS**: Inno Setupより機能が少ない
- **Windows Installer (MSI)**: 自作は非常に複雑

---

## Inno Setupスクリプト

### setup.iss（概要）

```iss
; SANKEY Copier Unified Installer
; Installs rust-server (Windows Service) + Desktop App

#define MyAppName "SANKEY Copier"
#define MyAppVersion "1.0.0"
#define MyAppPublisher "SANKEY Copier Team"
#define MyAppURL "https://github.com/yourorg/sankey-copier"

[Setup]
AppId={{YOUR-GUID-HERE}}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
DefaultDirName={autopf}\SANKEY Copier
DefaultGroupName={#MyAppName}
OutputDir=output
OutputBaseFilename=SankeyCopierSetup-{#MyAppVersion}
Compression=lzma2/max
SolidCompression=yes
PrivilegesRequired=admin
ArchitecturesAllowed=x64
ArchitecturesInstallIn64BitMode=x64
WizardStyle=modern

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"

[Tasks]
Name: "desktopicon"; Description: "デスクトップアイコンを作成"; GroupDescription: "追加アイコン:"
Name: "startservice"; Description: "インストール後にrust-serverサービスを開始"; GroupDescription: "サービス:"

[Files]
; rust-server
Source: "..\rust-server\target\release\rust-server.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\rust-server\config.toml"; DestDir: "{app}"; Flags: ignoreversion onlyifdoesntexist

; Desktop App
Source: "..\desktop-app\src-tauri\target\release\sankey-copier-desktop.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\desktop-app\src-tauri\target\release\webview2\*"; DestDir: "{app}\webview2"; Flags: ignoreversion recursesubdirs

; MT4/MT5 Components
Source: "..\mql\build\mt4\Experts\*.ex4"; DestDir: "{app}\mql\mt4\Experts"; Flags: ignoreversion
Source: "..\mql\build\mt4\Libraries\*.dll"; DestDir: "{app}\mql\mt4\Libraries"; Flags: ignoreversion
Source: "..\mql\build\mt5\Experts\*.ex5"; DestDir: "{app}\mql\mt5\Experts"; Flags: ignoreversion
Source: "..\mql\build\mt5\Libraries\*.dll"; DestDir: "{app}\mql\mt5\Libraries"; Flags: ignoreversion

; Icon
Source: "..\app.ico"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
; Desktop App shortcut
Name: "{group}\SANKEY Copier"; Filename: "{app}\sankey-copier-desktop.exe"; IconFilename: "{app}\app.ico"
Name: "{autodesktop}\SANKEY Copier"; Filename: "{app}\sankey-copier-desktop.exe"; IconFilename: "{app}\app.ico"; Tasks: desktopicon

[Run]
; Install rust-server as Windows Service
Filename: "sc.exe"; Parameters: "create SankeyCopierServer binPath= ""{app}\rust-server.exe"" DisplayName= ""SANKEY Copier Server"" start= auto"; Flags: runhidden
Filename: "sc.exe"; Parameters: "description SankeyCopierServer ""Trade copier server for MT4/MT5"""; Flags: runhidden
Filename: "sc.exe"; Parameters: "start SankeyCopierServer"; Flags: runhidden; Tasks: startservice

[UninstallRun]
; Stop and remove service
Filename: "sc.exe"; Parameters: "stop SankeyCopierServer"; Flags: runhidden
Filename: "sc.exe"; Parameters: "delete SankeyCopierServer"; Flags: runhidden

[Code]
// Custom installation logic (if needed)

function InitializeSetup(): Boolean;
begin
  Result := True;
end;
```

### 主要セクション説明

#### `[Setup]`
- インストーラーの基本設定
- 管理者権限要求（`PrivilegesRequired=admin`）
- 64-bit専用（`ArchitecturesAllowed=x64`）

#### `[Files]`
- インストールするファイル一覧
- `rust-server.exe`, `sankey-copier-desktop.exe`, MT4/MT5コンポーネント

#### `[Icons]`
- スタートメニュー、デスクトップショートカット

#### `[Run]`
- インストール後に実行するコマンド
- `sc.exe`でWindowsサービス登録・起動

#### `[UninstallRun]`
- アンインストール時にサービス停止・削除

---

## ビルドプロセス

### ビルドスクリプト: `installer/build-installer.ps1`

```powershell
# SANKEY Copier Unified Installer Build Script

param(
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"

Write-Host "=====================================" -ForegroundColor Cyan
Write-Host "Building SANKEY Copier Installer" -ForegroundColor Cyan
Write-Host "=====================================" -ForegroundColor Cyan

$PROJECT_ROOT = (Get-Item $PSScriptRoot).Parent.FullName

if (-not $SkipBuild) {
    # 1. Build rust-server
    Write-Host "`n[1/4] Building rust-server..." -ForegroundColor Yellow
    Push-Location "$PROJECT_ROOT\rust-server"
    cargo build --release
    if ($LASTEXITCODE -ne 0) { throw "rust-server build failed" }
    Pop-Location

    # 2. Build web-ui (static export for Desktop App)
    Write-Host "`n[2/4] Building web-ui (static export)..." -ForegroundColor Yellow
    Push-Location "$PROJECT_ROOT\web-ui"
    $env:NEXT_BUILD_MODE = "export"
    npm install
    npm run build
    if ($LASTEXITCODE -ne 0) { throw "web-ui build failed" }
    Pop-Location

    # 3. Build Desktop App (Tauri)
    Write-Host "`n[3/4] Building Desktop App..." -ForegroundColor Yellow
    Push-Location "$PROJECT_ROOT\desktop-app"
    npm install
    npm run tauri build
    if ($LASTEXITCODE -ne 0) { throw "Desktop App build failed" }
    Pop-Location
} else {
    Write-Host "Skipping builds (using existing binaries)..." -ForegroundColor Yellow
}

# 4. Build installer with Inno Setup
Write-Host "`n[4/4] Building installer..." -ForegroundColor Yellow

# Check if Inno Setup is installed
$InnoSetupPath = "C:\Program Files (x86)\Inno Setup 6\ISCC.exe"
if (-not (Test-Path $InnoSetupPath)) {
    Write-Host "❌ Inno Setup 6 not found!" -ForegroundColor Red
    Write-Host "Download from: https://jrsoftware.org/isdl.php" -ForegroundColor Yellow
    exit 1
}

# Compile installer
Push-Location "$PROJECT_ROOT\installer"
& $InnoSetupPath "setup.iss"
if ($LASTEXITCODE -ne 0) { throw "Installer build failed" }
Pop-Location

Write-Host "`n✅ Installer build completed!" -ForegroundColor Green
Write-Host "Output: installer\output\SankeyCopierSetup-1.0.0.exe" -ForegroundColor Green
```

### 使用方法

```powershell
# フルビルド（すべてのコンポーネントをビルド）
.\installer\build-installer.ps1

# インストーラーのみビルド（既存のバイナリを使用）
.\installer\build-installer.ps1 -SkipBuild
```

---

## GitHub Actions統合

### `.github/workflows/build-installer.yml`

```yaml
name: Build Unified Installer

on:
  push:
    tags:
      - 'v*'
  workflow_dispatch:

jobs:
  build-installer:
    runs-on: windows-latest

    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'

      - name: Install Inno Setup
        run: |
          choco install innosetup -y

      - name: Build rust-server
        working-directory: rust-server
        run: cargo build --release

      - name: Build web-ui (static export)
        working-directory: web-ui
        env:
          NEXT_BUILD_MODE: export
        run: |
          npm install
          npm run build

      - name: Build Desktop App
        working-directory: desktop-app
        run: |
          npm install
          npm run tauri build

      - name: Build installer
        working-directory: installer
        run: |
          & "C:\Program Files (x86)\Inno Setup 6\ISCC.exe" setup.iss

      - name: Upload installer
        uses: actions/upload-artifact@v4
        with:
          name: sankey-copier-installer
          path: installer/output/*.exe

      - name: Create Release (on tag)
        if: startsWith(github.ref, 'refs/tags/')
        uses: softprops/action-gh-release@v1
        with:
          files: installer/output/*.exe
```

---

## インストール手順（ユーザー視点）

### 1. インストーラーのダウンロード

```
https://github.com/yourorg/sankey-copier/releases/latest
→ SankeyCopierSetup-1.0.0.exe をダウンロード
```

### 2. インストール実行

1. `SankeyCopierSetup-1.0.0.exe`をダブルクリック
2. UAC（管理者権限）の確認ダイアログで「はい」
3. セットアップウィザードに従う:
   - インストール先選択（デフォルト: `C:\Program Files\SANKEY Copier`）
   - タスク選択:
     - ✅ デスクトップアイコンを作成
     - ✅ インストール後にrust-serverサービスを開始
4. 「インストール」をクリック

### 3. インストール完了後

- **rust-serverサービス**: 自動起動（バックグラウンド）
- **Desktop App**: デスクトップアイコンから起動可能

### 4. MT4/MT5コンポーネントのインストール

Desktop Appを起動:
1. サイドバーから「MT4/MT5 Installations」を開く
2. 自動検出されたMT4/MT5を確認
3. 「Install」ボタンでDLL/EAをインストール

---

## アンインストール手順

### 方法1: Windowsの設定から

1. **設定** → **アプリ** → **アプリと機能**
2. 「SANKEY Copier」を検索
3. **アンインストール**をクリック
4. 確認ダイアログで「はい」

### 方法2: コントロールパネルから

1. **コントロールパネル** → **プログラムと機能**
2. 「SANKEY Copier」を選択
3. **アンインストール**をクリック

### アンインストール時の動作

- rust-serverサービスを停止・削除
- インストールディレクトリを削除
- デスクトップショートカット削除
- スタートメニュー項目削除

**注意**: `config.toml`やログファイルは削除されません（ユーザーデータ保護）。

---

## サービス管理（上級ユーザー向け）

### サービスの状態確認

```powershell
sc query SankeyCopierServer
```

### サービスの手動起動/停止

```powershell
# 起動
sc start SankeyCopierServer

# 停止
sc stop SankeyCopierServer

# 再起動
sc stop SankeyCopierServer
sc start SankeyCopierServer
```

### サービスログの確認

```
C:\Program Files\SANKEY Copier\logs\sankey-copier-YYYY-MM-DD.log
```

---

## 利点

### ユーザー視点

1. **簡単インストール**: 1つのEXEで完了
2. **自動起動**: rust-serverがシステム起動時に自動起動
3. **独立動作**: Desktop Appとrust-serverは別プロセス
4. **簡単アンインストール**: Windowsの標準機能で削除可能

### 開発者視点

1. **単一配布物**: ビルド・リリースが簡単
2. **GitHub Actions統合**: タグpush時に自動ビルド
3. **メンテナンス容易**: Inno Setupスクリプトは読みやすい
4. **コード署名対応**: SignTool連携可能

---

## ロードマップ

### Phase 1: 基本実装（1週間）
- [ ] Inno Setupスクリプト作成
- [ ] ローカルビルド成功
- [ ] 手動テスト

### Phase 2: 自動化（1週間）
- [ ] ビルドスクリプト作成
- [ ] GitHub Actions統合
- [ ] 自動リリース

### Phase 3: 品質向上（1週間）
- [ ] 多言語対応（日本語UI）
- [ ] カスタムセットアップ画面
- [ ] コード署名対応

---

## 参考資料

- [Inno Setup Documentation](https://jrsoftware.org/ishelp/)
- [Windowsサービス作成](https://learn.microsoft.com/en-us/windows/win32/services/services)
- [sc.exe コマンドリファレンス](https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/sc-create)
