# SANKEY Copier統合インストーラー

このディレクトリには、SANKEY Copier統合インストーラーのビルドスクリプトが含まれています。

## 概要

1つのインストーラーで以下をインストール：
- **relay-server** - Windowsサービスとして24/7稼働
- **Desktop App** - 設定変更用（必要時のみ起動）
- **MT4/MT5コンポーネント** - EA、DLL（オプション）

## 必要要件

### ビルド環境

- Windows 10/11 (64-bit)
- [Rust](https://rustup.rs/) 1.70以上
- [Node.js](https://nodejs.org/) 20以上
- [pnpm](https://pnpm.io/) または npm
- [Inno Setup 6](https://jrsoftware.org/isdl.php)

### インストール（開発者向け）

```powershell
# Chocolateyでまとめてインストール
choco install rust nodejs pnpm innosetup -y

# または手動でダウンロード・インストール
```

## ビルド方法

### 方法1: フルビルド（推奨）

すべてのコンポーネントを自動的にビルドしてインストーラーを作成：

```powershell
cd installer
.\build-installer.ps1
```

**実行内容:**
1. relay-serverをビルド（Cargo）
2. web-uiを静的エクスポート（Next.js）
3. Desktop Appをビルド（Tauri）
4. MT4/MT5コンポーネントをビルド（オプション）
5. Inno Setupでインストーラーを作成

**出力:**
```
installer/output/SankeyCopierSetup-1.0.0.exe
```

### 方法2: 既存バイナリを使用

既にビルド済みのバイナリがある場合、インストーラーのみ作成：

```powershell
cd installer
.\build-installer.ps1 -SkipBuild
```

### 方法3: MQLコンポーネントをスキップ

MT4/MT5コンポーネントなしでビルド：

```powershell
cd installer
.\build-installer.ps1 -SkipMQL
```

## GitHub Actions

### 自動ビルド

GitHub Actionsで自動的にビルドされます：

- **mainブランチへのpush**: インストーラーをビルドしてArtifactとして保存
- **タグpush（`v*`）**: リリースを作成してインストーラーを添付

### 手動トリガー

GitHub UIから手動でビルドを実行できます：

1. リポジトリの**Actions**タブ
2. **Build Unified Installer**を選択
3. **Run workflow**をクリック

## インストーラーの内容

### ビルド成果物（dist/構造）

GitHub Actionsまたはローカルビルドで生成される`dist/`ディレクトリの構造：

```
dist/
├── sankey-copier-server.exe    # relay-server
├── sankey-copier-desktop.exe   # Desktop App（web-ui内蔵）
├── sankey-copier-tray.exe      # トレイアプリ
├── config.toml                 # 設定ファイル
├── app.ico                     # アイコン
├── nssm.exe                    # Windowsサービス管理ツール
└── mt-advisors/                # MT4/MT5コンポーネント
    ├── MT4/
    │   ├── Experts/
    │   │   ├── SankeyCopierMaster.ex4
    │   │   └── SankeyCopierSlave.ex4
    │   └── Libraries/
    │       └── sankey_copier_zmq.dll
    └── MT5/
        ├── Experts/
        │   ├── SankeyCopierMaster.ex5
        │   └── SankeyCopierSlave.ex5
        └── Libraries/
            └── sankey_copier_zmq.dll
```

### インストールされるファイル

```
C:\Program Files\SANKEY Copier\
├── sankey-copier-desktop.exe   # Desktop App（web-ui内蔵）
├── sankey-copier-server.exe    # relay-server（Windowsサービス）
├── sankey-copier-tray.exe      # トレイアプリ
├── config.toml                 # relay-server設定ファイル
├── app.ico                     # アイコン
├── nssm.exe                    # サービス管理ツール
└── mt-advisors/                # MT4/MT5コンポーネント（オプション）
    ├── MT4/
    │   ├── Experts/
    │   │   ├── SankeyCopierMaster.ex4
    │   │   └── SankeyCopierSlave.ex4
    │   └── Libraries/
    │       └── sankey_copier_zmq.dll
    └── MT5/
        ├── Experts/
        │   ├── SankeyCopierMaster.ex5
        │   └── SankeyCopierSlave.ex5
        └── Libraries/
            └── sankey_copier_zmq.dll
```

### Windowsサービス

**サービス名:** `SankeyCopierServer`
**表示名:** SANKEY Copier Server
**起動タイプ:** 自動（システム起動時に自動起動）

### デスクトップショートカット

- デスクトップ: `SANKEY Copier`
- スタートメニュー: `SANKEY Copier`

## インストーラーのカスタマイズ

### バージョン変更

`setup.iss`の先頭部分を編集：

```iss
#define MyAppVersion "1.0.0"  // ← ここを変更
```

### アイコン変更

```iss
SetupIconFile=..\app.ico  // ← アイコンファイルのパス
```

### 言語追加

```iss
[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"
; 追加の言語はここに
```

Inno Setupの`Languages`ディレクトリにある`.isl`ファイルを指定できます。

## トラブルシューティング

### エラー: "Inno Setup not found"

**原因:** Inno Setup 6がインストールされていない

**解決方法:**
```powershell
choco install innosetup -y
```

または [公式サイト](https://jrsoftware.org/isdl.php)からダウンロード

### エラー: "relay-server build failed"

**原因:** Rustツールチェーンが正しくインストールされていない

**解決方法:**
```powershell
# Rustのインストール
rustup-init

# 再試行
.\build-installer.ps1
```

### エラー: "web-ui build failed"

**原因:** Node.jsまたはpnpmがインストールされていない

**解決方法:**
```powershell
# Node.jsとpnpmのインストール
choco install nodejs pnpm -y

# 再試行
.\build-installer.ps1
```

### インストーラーが起動しない

**原因:** 管理者権限が必要

**解決方法:**
- インストーラーを右クリック → **管理者として実行**

### サービスが起動しない

**原因:** ポート3000が既に使用されている

**解決方法:**
```powershell
# ポート3000を使用しているプロセスを確認
netstat -ano | findstr :3000

# プロセスを終了（PIDを確認して）
taskkill /PID <PID> /F
```

## 開発者向け情報

### ディレクトリ構成

```
installer/
├── setup.iss                   # Inno Setupスクリプト
├── build-installer.ps1         # ビルドスクリプト
├── README.md                   # このファイル
└── output/                     # ビルド成果物（生成される）
    └── SankeyCopierSetup-1.0.0.exe
```

### Inno Setupスクリプトの構成

- **[Setup]**: インストーラーの基本設定
- **[Languages]**: サポート言語
- **[Tasks]**: インストール時のタスク（デスクトップアイコン等）
- **[Files]**: インストールするファイル一覧
- **[Icons]**: ショートカット作成
- **[Run]**: インストール後に実行するコマンド（サービス登録）
- **[UninstallRun]**: アンインストール時のクリーンアップ
- **[Code]**: カスタムロジック（Pascal Script）

### デバッグモード

Inno Setupをデバッグモードで実行：

```powershell
& "C:\Program Files (x86)\Inno Setup 6\Compil32.exe" /cc "setup.iss"
```

これによりInno Setup IDEが開き、対話的にビルドできます。

## 参考資料

- [Inno Setup Documentation](https://jrsoftware.org/ishelp/)
- [Inno Setup FAQ](https://jrsoftware.org/isfaq.php)
- [Tauri Documentation](https://tauri.app/)
- [統合インストーラー実装計画](../docs/UNIFIED_INSTALLER_PLAN.md)

## ライセンス

メインプロジェクトと同じライセンスが適用されます。
