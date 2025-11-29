# Windows Installer UX Design Specification

## 概要

SANKEY CopierのWindowsインストーラーのUX設計書です。特にMT4/MT5へのDLL・EAデプロイメント機能に焦点を当てています。

## 基本方針

### 初回インストールの範囲

Windowsインストーラーでは、以下のコンポーネントのみをインストールします：

1. **Rust Server** (Windows Service)
   - サービス名: `SankeyCopierServer`
   - バイナリ: `sankey-copier-server.exe`
   - 自動起動設定

2. **Web UI** (Windows Service or 静的ファイル配信)
   - Next.jsアプリケーション
   - サービス名: `SankeyCopierWebUI` (スタンドアロンの場合)
   - または、Rust Serverから静的ファイルとして配信

3. **コンポーネントファイル** (未デプロイ状態)
   - DLL (32bit/64bit版)
   - EA files (MT4/MT5 Master/Slave)
   - Include files (.mqh)
   - インストール先: `C:\Program Files\Sankey Copier\components\`

### MT4/MT5へのデプロイは初回インストール後に実施

**理由：**
- ユーザーが複数のブローカーのMT4/MT5を使用する可能性がある
- MT4/MT5を後からインストールする場合がある
- 一部のMT4/MT5にのみインストールしたい場合がある
- MT4/MT5のアップデートや再インストール時に再デプロイが必要
- インストール時にMT4/MT5が起動中の場合、ファイルロックの問題が発生する

---

## MT4/MT5デプロイメントのUX設計

### アプローチ: Web UIベース管理システム

MT4/MT5へのDLL・EAデプロイは、Web UIから管理する方式を採用します。

#### メリット

- ✅ 既存のWeb UIに統合可能
- ✅ リモートからも管理可能（同一LAN内）
- ✅ インストール状況を視覚的に確認できる
- ✅ 複数のMT4/MT5インストールを一元管理
- ✅ 必要なときに必要なMT4/MT5にデプロイ可能
- ✅ バージョン管理と更新が容易

#### セキュリティ考慮事項

- ローカルホスト（127.0.0.1）またはプライベートネットワークからのみアクセス可能
- ファイル操作は事前定義されたパス（Program Files配下のMetaTraderフォルダ）のみ
- 管理者権限が必要な場合は、Windowsサービスが既に管理者として実行されることを利用

---

## UI設計

### 新規ページ: "Installation Manager"

Web UIにMT4/MT5インストール管理用の新しいページを追加します。

#### ページレイアウト

```
┌────────────────────────────────────────────────────────────┐
│ SANKEY Copier - Installation Manager                      │
├────────────────────────────────────────────────────────────┤
│                                                            │
│ MT4/MT5 Installations                     🔄 Auto-detected │
│                                                            │
│ ┌──────────────────────────────────────────────────────┐   │
│ │ ✅ MetaTrader 4 - IC Markets            🟢 Running    │   │
│ │ Path: D:\Trading\IC Markets MT4\                     │   │
│ │ Type: MT4 (32-bit)                                   │   │
│ │ Status: Installed (v1.2.3)                           │   │
│ │ Detection: Process-based                             │   │
│ │ Last Updated: 2024-01-15 10:30                       │   │
│ │                                                       │   │
│ │ Installed Components:                                │   │
│ │   • DLL: sankey_copier_zmq.dll ✓                     │   │
│ │   • Master EA: SankeyCopierMaster.mq4 ✓              │   │
│ │   • Slave EA: SankeyCopierSlave.mq4 ✓                │   │
│ │   • Include files: SankeyCopier/*.mqh ✓              │   │
│ │                                                       │   │
│ │ [Update] [Reinstall] [Uninstall] [View Details]     │   │
│ └──────────────────────────────────────────────────────┘   │
│                                                            │
│ ┌──────────────────────────────────────────────────────┐   │
│ │ ⚪ MetaTrader 5 - XM                     🟢 Running    │   │
│ │ Path: C:\Program Files\XM MetaTrader 5\              │   │
│ │ Type: MT5 (64-bit)                                   │   │
│ │ Status: Not installed                                │   │
│ │ Detection: Process-based                             │   │
│ │                                                       │   │
│ │ [Install Now] [Skip]                                 │   │
│ └──────────────────────────────────────────────────────┘   │
│                                                            │
│ ┌──────────────────────────────────────────────────────┐   │
│ │ ⚠️ MetaTrader 5 - FXGT                   ⚫ Stopped   │   │
│ │ Path: C:\Users\John\MT5\FXGT\                        │   │
│ │ Type: MT5 (32-bit)                                   │   │
│ │ Status: Version mismatch (installed: v1.1.0)         │   │
│ │ Available: v1.2.3                                    │   │
│ │ Detection: Manual                                    │   │
│ │                                                       │   │
│ │ [Update Now] [View Changes] [Remove]                │   │
│ └──────────────────────────────────────────────────────┘   │
│                                                            │
│ [Refresh] [Install to All] [Add Manually]                 │
│                                                            │
│ Detection Status:                                          │
│   🔍 Running Processes: 3 found                           │
│   👤 Manually Added: 0                                     │
│                                                            │
│ 💡 Tip: Start MT4/MT5 to auto-detect, or use [Add         │
│    Manually] to add installations.                        │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

### UI要素の詳細

#### 1. インストール状態のバッジ

- ✅ **Installed**: すべてのコンポーネントが正しくインストールされている
- ⚪ **Not installed**: まだインストールされていない
- ⚠️ **Version mismatch**: 古いバージョンがインストールされている
- ❌ **Error**: インストールエラーまたは不完全なインストール
- 🔄 **Installing**: インストール中

#### 2. 起動状態インジケーター

- 🟢 **Running**: MT4/MT5が起動中（プロセス検出）
- ⚫ **Stopped**: MT4/MT5が停止中
- ❓ **Unknown**: 起動状態を判定できない（権限不足など）

**注意事項:**
- 起動中のMT4/MT5へのインストールは、ファイルロックにより失敗する可能性があります
- 可能であれば、インストール前にMT4/MT5を終了することを推奨
- UIで警告を表示: "⚠️ MT4/MT5 is running. Please close it before installation."

#### 3. 検出方法の表示

各MT4/MT5に検出方法を表示:
- **Process-based**: 起動中プロセスから自動検出（リアルタイム）
- **Manual**: ユーザーが手動で追加

#### 4. アクションボタン

- **Install Now**: 新規インストールを実行
- **Update**: 新しいバージョンに更新
- **Reinstall**: 既存のインストールを再実行（トラブルシューティング用）
- **Uninstall**: コンポーネントを削除
- **View Details**: インストール詳細やログを表示

#### 5. 一括操作ボタン

- **Refresh**: MT4/MT5インストールを再スキャン（起動中プロセスを再検出）
- **Install to All**: 検出されたすべてのMT4/MT5にインストール
- **Add Manually**: ファイルブラウザでterminal.exeを手動選択して追加

---

## インストールフロー

### 1. MT4/MT5検出

```
ユーザーがページを開く
    ↓
Rust Serverが自動的にMT4/MT5をスキャン
    ↓
検出されたインストールを一覧表示
```

**検出方法（2つのシンプルなアプローチ）:**

#### 方法1: 起動中プロセスからの自動検出

**利点:**
- インストール場所に依存しない（任意のディレクトリに対応）
- ポータブル版やカスタムディレクトリにも対応
- 実際に使用中のMT4/MT5のみを検出
- 確実にパスを取得可能
- 権限問題がほとんどない

**実装:**
```rust
// 1. Windowsプロセス一覧を取得（sysinfo crateなどを使用）
// 2. "terminal.exe" または "terminal64.exe" を検索
// 3. プロセスの実行ファイルフルパスを取得
//    例: D:\Trading\IC Markets MT4\terminal.exe
//    → ベースパス: D:\Trading\IC Markets MT4\
// 4. MQL4/MQL5フォルダの存在確認でMT4/MT5を判別
// 5. terminal64.exe なら 64-bit、terminal.exe なら 32-bit
```

**取得可能な情報:**
- プロセスID
- 実行ファイルフルパス
- ビット数（実行ファイル名から判定）
- MT4/MT5の種別（MQL4/MQL5フォルダの存在確認）
- 起動状態（常にRunning）

**ユーザーへのガイダンス:**
```
検出されたMT4/MT5がない場合:

💡 Tip: MT4/MT5を起動すると自動的に検出されます
または、下記の「Add Manually」ボタンから手動で追加できます
```

#### 方法2: ユーザー手動指定

**UI:**
```
[Add MT4/MT5 Manually]
  ↓
ファイル選択ダイアログ
  ↓
terminal.exe または terminal64.exe を選択
  ↓
自動的にベースパスとMT4/MT5を判定して追加
```

**利点:**
- 確実に任意の場所のMT4/MT5を追加可能
- MT4/MT5が起動していない場合でも追加可能
- シンプルで分かりやすい

**検証ロジック:**
```rust
// 選択されたファイルが terminal.exe / terminal64.exe かチェック
// 親ディレクトリに MQL4 または MQL5 フォルダがあるかチェック
// なければエラー: "選択されたファイルは有効なMT4/MT5実行ファイルではありません"
```

**判定ロジック:**
```rust
// ビット数判定
if executable_name == "terminal64.exe" {
    platform = "64-bit"
} else if executable_name == "terminal.exe" {
    platform = "32-bit"
}

// MT4/MT5判定
if exists(base_path + "/MQL5") {
    type = "MT5"
} else if exists(base_path + "/MQL4") {
    type = "MT4"
}
```

### 2. インストール実行

```
ユーザーが「Install Now」をクリック
    ↓
確認ダイアログ表示
    ↓
Rust Serverにインストールリクエスト送信
    ↓
サーバー側でファイルコピー実行
    ↓
結果をWeb UIに表示
```

**インストールされるファイル:**

MT4の場合:
```
[MT4 Installation Path]\
├── MQL4\
│   ├── Experts\
│   │   ├── SankeyCopierMaster.mq4
│   │   └── SankeyCopierSlave.mq4
│   ├── Libraries\
│   │   └── sankey_copier_zmq.dll (32-bit)
│   └── Include\
│       └── SankeyCopier\
│           ├── SankeyCopierCommon.mqh
│           ├── SankeyCopierMessages.mqh
│           └── SankeyCopierTrade.mqh
```

MT5の場合:
```
[MT5 Installation Path]\
├── MQL5\
│   ├── Experts\
│   │   ├── SankeyCopierMaster.mq5
│   │   └── SankeyCopierSlave.mq5
│   ├── Libraries\
│   │   └── sankey_copier_zmq.dll (32-bit or 64-bit)
│   └── Include\
│       └── SankeyCopier\
│           ├── SankeyCopierCommon.mqh
│           ├── SankeyCopierMessages.mqh
│           └── SankeyCopierTrade.mqh
```

### 3. インストール確認

インストール後、以下をチェック:
- すべてのファイルが正しくコピーされたか
- ファイルサイズが一致するか
- DLLのビット数が正しいか（32-bit MT5に64-bit DLLは不可）

### 4. ユーザーへの案内

インストール成功後、以下を案内:

```
✅ Installation completed successfully!

Next steps:
1. Restart MetaTrader
2. Go to Tools > Options > Expert Advisors
   ☑ Allow automated trading
   ☑ Allow DLL imports
3. Attach SankeyCopierMaster EA to a chart on Master account
4. Attach SankeyCopierSlave EA to a chart on Slave account
5. Configure copy settings from the main page

[Open Setup Guide] [Go to Settings]
```

---

## API設計

### 新規APIエンドポイント

Rust Serverに以下のエンドポイントを追加:

#### 1. MT4/MT5インストール検出

```
GET /api/mt-installations
```

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "id": "mt4-ic-markets-d-trading",
      "name": "IC Markets MetaTrader 4",
      "type": "MT4",
      "platform": "32-bit",
      "path": "D:\\Trading\\IC Markets MT4",
      "executable": "D:\\Trading\\IC Markets MT4\\terminal.exe",
      "version": "4.00 build 1380",
      "is_running": true,
      "process_id": 12345,
      "detection_method": "process",
      "is_installed": true,
      "installed_version": "1.2.3",
      "available_version": "1.2.3",
      "components": {
        "dll": true,
        "master_ea": true,
        "slave_ea": true,
        "includes": true
      },
      "last_updated": "2024-01-15T10:30:00Z"
    },
    {
      "id": "mt5-xm-64",
      "name": "XM MetaTrader 5",
      "type": "MT5",
      "platform": "64-bit",
      "path": "C:\\Program Files\\XM MetaTrader 5",
      "executable": "C:\\Program Files\\XM MetaTrader 5\\terminal64.exe",
      "version": "5.00 build 3802",
      "is_running": true,
      "process_id": 23456,
      "detection_method": "process",
      "is_installed": false,
      "installed_version": null,
      "available_version": "1.2.3",
      "components": {
        "dll": false,
        "master_ea": false,
        "slave_ea": false,
        "includes": false
      },
      "last_updated": null
    },
    {
      "id": "mt5-fxgt-32",
      "name": "FXGT MetaTrader 5",
      "type": "MT5",
      "platform": "32-bit",
      "path": "C:\\Users\\John\\MT5\\FXGT",
      "executable": "C:\\Users\\John\\MT5\\FXGT\\terminal.exe",
      "version": "5.00 build 3750",
      "is_running": false,
      "process_id": null,
      "detection_method": "manual",
      "is_installed": true,
      "installed_version": "1.1.0",
      "available_version": "1.2.3",
      "components": {
        "dll": true,
        "master_ea": true,
        "slave_ea": false,
        "includes": true
      },
      "last_updated": "2024-01-10T08:15:00Z"
    }
  ],
  "detection_summary": {
    "total_found": 3,
    "by_method": {
      "process": 2,
      "manual": 1
    },
    "running": 2,
    "stopped": 1
  }
}
```

#### 2. インストール実行

```
POST /api/mt-installations/{id}/install
```

**Request Body:**
```json
{
  "components": ["dll", "master_ea", "slave_ea", "includes"],
  "force_reinstall": false
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "installation_id": "mt4-ic-markets",
    "installed_components": ["dll", "master_ea", "slave_ea", "includes"],
    "version": "1.2.3",
    "timestamp": "2024-01-15T10:35:00Z"
  }
}
```

#### 3. アンインストール

```
DELETE /api/mt-installations/{id}/uninstall
```

**Response:**
```json
{
  "success": true,
  "message": "Successfully uninstalled all components"
}
```

#### 4. インストール状態の更新（再スキャン）

```
POST /api/mt-installations/refresh
```

**Response:**
```json
{
  "success": true,
  "data": {
    "detected_count": 3,
    "installed_count": 1
  }
}
```

---

## 代替案: ワンクリックインストーラー

Web UIからのインストールが技術的に困難な場合の代替案として、ローカルで実行するインストーラースクリプトを提供します。

### PowerShellスクリプト版

```
[Installation Manager] ページから
    ↓
「Download Installer」ボタンをクリック
    ↓
PowerShellスクリプト (.ps1) をダウンロード
    ↓
右クリック > 「PowerShellで実行」
    ↓
対話型インストーラーが起動
```

**スクリプトの機能:**
1. MT4/MT5を自動検出
2. インストール先を選択（複数選択可）
3. コンポーネントを自動コピー
4. インストール結果を表示

**メリット:**
- 権限問題を回避しやすい
- ユーザーが直接実行するため信頼性が高い

**デメリット:**
- Web UIから直接操作できない
- 自動化が難しい

---

## 追加機能の検討

### 1. バージョン管理

- インストールされたコンポーネントのバージョンを記録
- 新しいバージョンが利用可能になったら通知
- 変更履歴の表示

### 2. トラブルシューティング

- インストールログの表示
- 不完全なインストールの検出と修復
- DLLロードエラーの診断

### 3. 一括管理

- 複数のMT4/MT5に一括インストール
- 設定のエクスポート/インポート

### 4. ロールバック

- 以前のバージョンに戻す機能
- インストール前のバックアップ

---

## 実装の優先順位

### Phase 1: MVP (Minimum Viable Product)

1. MT4/MT5自動検出API
2. インストール実行API
3. 基本的なWeb UIページ
4. インストール状態の表示

### Phase 2: 改善

1. アンインストール機能
2. 再インストール機能
3. 詳細なエラーハンドリング
4. インストールログの表示

### Phase 3: 拡張機能

1. バージョン管理と更新通知
2. 一括インストール
3. PowerShellスクリプト生成
4. トラブルシューティングツール

---

## まとめ

**推奨アプローチ:**
- Windowsインストーラーでは、Rust ServerとWeb UIのみをWindows Serviceとしてインストール
- MT4/MT5へのDLL・EAデプロイは、Web UIから管理する「Installation Manager」ページを追加
- ユーザーが必要なときに必要なMT4/MT5にデプロイできる柔軟な設計

**主な利点:**
- 初回インストールがシンプル
- 複数のMT4/MT5インストールに対応
- 後から追加されるMT4/MT5にも柔軟に対応
- バージョン管理と更新が容易
- トラブルシューティングがしやすい
