# MQL Build Issues - 既知の問題と対応案

## 現状

GitHub ActionsでMQL4/MQL5ファイルのコンパイルが**現在無効化**されています。

### 無効化の理由

```yaml
# Temporarily disabled due to download.mql5.com being blocked (451 error)
# TODO: Implement alternative compilation method or use pre-compiled binaries
if: ${{ github.event.inputs.build_target == 'mql' && false }}
```

## 問題の詳細

### 1. HTTP 451 エラー - Unavailable For Legal Reasons

**症状:**
```
Error downloading MT5: HTTP 451 - Unavailable For Legal Reasons
URL: https://download.mql5.com/cdn/web/metaquotes.software.corp/mt5/mt5setup.exe
```

**原因:**
- `download.mql5.com` ドメインがGitHub Actionsのランナー環境からアクセスできない
- 地域的な制限またはネットワークポリシーによりブロックされている
- GitHub Actionsの複数の地域（特にUS）でこの問題が発生

**影響範囲:**
- `fx31337/mql-compile-action@v1` および `@v2` の両方で発生
- MT4とMT5の両方でインストールに失敗

### 2. MQL Compile Action バージョンの問題

#### v2 で試行した結果
```
Error: Unexpected input(s) 'mt-version'
```
- v2では自動的にMT4/MT5をインストールする機能が追加されたが、パラメータ仕様が変更されていた
- ドキュメント不足により正しいパラメータ設定が不明

#### v1 で試行した結果
- パラメータは正しく認識されたが、MT4/MT5のダウンロードでHTTP 451エラー
- インストールステップで完全に失敗

## 現在のワークフロー設定

### 対象ファイル

**MT4:**
- `mql/MT4/Master/ForexCopierMaster.mq4`
- `mql/MT4/Slave/ForexCopierSlave.mq4`

**MT5:**
- `mql/MT5/Master/ForexCopierMaster.mq5`
- `mql/MT5/Slave/ForexCopierSlave.mq5`

### 期待される出力
- `ForexCopierMaster.ex4` / `ForexCopierMaster.ex5`
- `ForexCopierSlave.ex4` / `ForexCopierSlave.ex5`

## 解決策の選択肢

### オプション 1: Self-Hosted GitHub Actions Runners（推奨度: ★★★★☆）

**概要:**
- 自社サーバーでGitHub Actions Runnerを設定
- MT4/MT5を事前にインストールしておく

**メリット:**
- 完全な制御が可能
- download.mql5.comへのアクセス制限を回避できる
- カスタムビルド環境を構築可能

**デメリット:**
- インフラストラクチャの維持コスト
- セキュリティ管理が必要
- 初期セットアップの手間

**実装手順:**
```yaml
compile-mql:
  name: Compile MQL4/MQL5 Files
  runs-on: self-hosted  # Self-hosted runnerを使用
  tags: [windows, mt4, mt5]  # 適切なタグを設定
```

### オプション 2: Pre-compiled Binaries（推奨度: ★★★★★）

**概要:**
- コンパイル済みの.ex4/.ex5ファイルをリポジトリにコミット
- ソースコード変更時にローカルで再コンパイル

**メリット:**
- 最もシンプルで確実
- ビルド時間の短縮
- GitHub Actionsの複雑性を回避

**デメリット:**
- バイナリファイルのバージョン管理
- ソースとバイナリの同期管理が必要
- コード変更時の手動コンパイルが必要

**実装案:**
```
mql/
  MT4/
    Master/
      ForexCopierMaster.mq4  (ソース)
      ForexCopierMaster.ex4  (コンパイル済み)
    Slave/
      ForexCopierSlave.mq4   (ソース)
      ForexCopierSlave.ex4   (コンパイル済み)
  MT5/
    Master/
      ForexCopierMaster.mq5  (ソース)
      ForexCopierMaster.ex5  (コンパイル済み)
    Slave/
      ForexCopierSlave.mq5   (ソース)
      ForexCopierSlave.ex5   (コンパイル済み)
```

ワークフロー更新:
```yaml
- name: Copy pre-compiled MQL files
  run: |
    mkdir -p artifacts/mql/MT4
    mkdir -p artifacts/mql/MT5
    copy mql\MT4\Master\*.ex4 artifacts\mql\MT4\
    copy mql\MT4\Slave\*.ex4 artifacts\mql\MT4\
    copy mql\MT5\Master\*.ex5 artifacts\mql\MT5\
    copy mql\MT5\Slave\*.ex5 artifacts\mql\MT5\
```

### オプション 3: Alternative MT4/MT5 Installation Source（推奨度: ★☆☆☆☆）

**概要:**
- 別のミラーサイトやCDNからMT4/MT5をダウンロード
- カスタムインストーラーを作成

**メリット:**
- 既存のワークフローを大幅に変更せずに済む可能性

**デメリット:**
- 信頼できる代替ソースの特定が困難
- セキュリティリスク（非公式ソース）
- 長期的なメンテナンス性に懸念

**実現可能性:** 低い

### オプション 4: Docker Container with MT4/MT5（推奨度: ★★☆☆☆）

**概要:**
- MT4/MT5がインストール済みのDockerイメージを作成
- GitHub Container Registryで管理

**メリット:**
- 再現可能なビルド環境
- バージョン管理が容易

**デメリット:**
- Windows Containerのサイズが大きい
- ライセンス問題の可能性
- 初期セットアップが複雑

### オプション 5: On-Demand Manual Compilation（推奨度: ★★★☆☆）

**概要:**
- MQLファイルの変更がある場合のみ手動でコンパイル
- リリース前に手動でビルドとテスト

**メリット:**
- シンプル
- インフラストラクチャ不要

**デメリット:**
- CI/CDパイプラインが不完全
- 人為的エラーのリスク
- 自動化の恩恵を受けられない

## 採用済みソリューション（2025-11-09実装）

### ✅ オプション 6: Custom PowerShell Installation + Automated Compilation

**実装内容:**
- PowerShellで直接MT4/MT5インストーラーをダウンロード
- `actions/cache@v4`でインストーラーをキャッシュ（MT4: 22.6MB, MT5: 22.7MB）
- サイレントインストール（`/auto`フラグ）
- MetaEditorのコマンドラインインターフェースでコンパイル
- コンパイル済み.ex4/.ex5ファイルの検証とアップロード

**メリット:**
- ✅ GitHub Actions上で完全自動化
- ✅ インストーラーキャッシュにより2回目以降はダウンロード不要
- ✅ MT4とMT5を並列ビルド（matrixストラテジー）
- ✅ コンパイルエラーの自動検出とログ出力
- ✅ `fx31337/mql-compile-action`のHTTP 451エラーを回避

**キャッシュ仕様:**
```yaml
key: ${{ matrix.platform }}-installer-v1
```
- キャッシュキーに`v1`を含めることでバージョン管理可能
- インストーラー更新時はキーの数字を増やすだけ

**コンパイルコマンド:**
```powershell
metaeditor.exe /compile:"path/to/file.mq4" /log /inc:"include/path"
```

**実装日:** 2025-11-09
**ステータス:** 本番環境デプロイ済み

---

## その他の解決策（参考）

### 短期的な代替案

**オプション 2: Pre-compiled Binaries**

1. ローカル環境でMQL4/MQL5ファイルをコンパイル
2. .ex4/.ex5ファイルをリポジトリにコミット
3. .gitignoreから.ex4/.ex5を除外
4. ワークフローでバイナリファイルを直接コピー

**実装の優先度:** 低（オプション6で解決済み）
**推定作業時間:** 30分

### 長期的な代替案

**オプション 1: Self-Hosted Runners**

プロジェクトが成熟し、頻繁なMQL変更が発生する場合:
1. Self-hosted runnerのセットアップ
2. MT4/MT5の事前インストール
3. 自動コンパイルパイプラインの構築

**実装の優先度:** 低（オプション6で十分）
**推定作業時間:** 4-8時間

## ~~現在の回避策~~（解決済み）

### ~~リリースパッケージへの影響~~

~~現在のワークフロー（`create-release-package`）は、MQLコンパイルジョブの完了を**待たない**ように設定されています:~~

**2025-11-09更新: 問題解決**

現在のワークフロー（`create-release-package`）は、MQLコンパイルジョブを含むように修正されました:

```yaml
needs: [build-rust-dll, build-rust-server, build-web-ui, compile-mql]
```

**結果:**
- ✅ Rust DLL、Rust Server、Web UIはビルドされる
- ✅ MQLファイル（.ex4/.ex5）が自動コンパイルされる
- ✅ リリースパッケージに全てのコンポーネントが含まれる
- ✅ ユーザーは手動コンパイル不要

### ~~ユーザーへの影響~~

**~~必要な手動ステップ:~~（不要になりました）**
1. ~~MT4/MT5を開く~~
2. ~~MetaEditorで.mq4/.mq5ファイルを開く~~
3. ~~手動でコンパイル~~
4. ~~生成された.ex4/.ex5ファイルを配置~~

**現在:**
GitHub Actionsが自動的に全てのビルドを実行します。ユーザーは成果物をダウンロードするだけで使用可能です。

## ~~次のステップ~~（完了）

### ~~即座に実施すべきこと~~

~~1. **Pre-compiled Binariesの追加**~~
~~2. **ワークフローの更新**~~
~~3. **ドキュメントの更新**~~

**2025-11-09更新: 全て実装完了**

実施済み:
1. ✅ カスタムPowerShellインストール＋自動コンパイルを実装
2. ✅ `compile-mql`ジョブを有効化し、完全自動化
3. ✅ インストーラーキャッシュ機構を実装
4. ✅ リリースパッケージにMQLファイルを含めるよう修正
5. ✅ このドキュメントを更新

### 検討事項

- [x] プロジェクトの規模とMQL変更頻度を評価 → 自動コンパイルで対応
- [x] ~~Self-hosted runnerの必要性を判断~~ → 不要（PowerShellダウンロードで解決）
- [ ] ライセンス要件の確認（バイナリ配布） → 要確認
- [x] ユーザーへの影響を文書化 → 完了

## 関連リソース

- GitHub Actions Self-hosted Runners: https://docs.github.com/en/actions/hosting-your-own-runners
- fx31337/mql-compile-action: https://github.com/fx31337/mql-compile-action
- MetaTrader Installation Issues: https://www.mql5.com/en/forum

---

**最終更新:** 2025-11-09
**ステータス:** ✅ 解決済み - 自動コンパイル実装完了
**優先度:** 完了（全コンポーネントが自動ビルドされる）

**実装概要:**
- PowerShellで直接MT4/MT5インストーラーをダウンロード
- GitHub Actions Cacheでインストーラーを再利用（2回目以降はダウンロード不要）
- MetaEditorコマンドラインで自動コンパイル
- MT4とMT5を並列ビルド（matrixストラテジー）
