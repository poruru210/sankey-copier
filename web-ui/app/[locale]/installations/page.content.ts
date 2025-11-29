import { t, type DeclarationContent } from 'intlayer';

const installationsPageContent = {
  key: 'installations-page',
  content: {
    title: t({
      en: 'Installation Manager',
      ja: 'インストール管理',
    }),
    description: t({
      en: 'Detect and install SANKEY Copier components to your MT4/MT5 platforms',
      ja: 'MT4/MT5プラットフォームへSANKEY Copierコンポーネントを検出してインストール',
    }),
    refreshDetection: t({
      en: 'Refresh Detection',
      ja: '検出を更新',
    }),
    installToSelected: t({
      en: 'Install to Selected',
      ja: '選択項目にインストール',
    }),
    installing: t({
      en: 'Installing',
      ja: 'インストール中',
    }),
    installationsCount: t({
      en: 'installation(s)',
      ja: '個のインストール',
    }),
    // Loading states
    loadingInstallations: t({
      en: 'Loading installations...',
      ja: 'インストールを読み込み中...',
    }),
    // No installations
    noInstallationsDetected: t({
      en: 'No MT4/MT5 installations detected.',
      ja: 'MT4/MT5インストールが検出されませんでした。',
    }),
    clickRefreshToScan: t({
      en: 'Click "Refresh Detection" to scan for installations',
      ja: '「検出を更新」をクリックしてインストールをスキャン',
    }),
    // Table headers
    name: t({
      en: 'Name',
      ja: '名前',
    }),
    type: t({
      en: 'Type',
      ja: 'タイプ',
    }),
    installationPath: t({
      en: 'Installation Path',
      ja: 'インストールパス',
    }),
    version: t({
      en: 'Version',
      ja: 'バージョン',
    }),
    components: t({
      en: 'Components',
      ja: 'コンポーネント',
    }),
    actions: t({
      en: 'Actions',
      ja: 'アクション',
    }),
    // Component names
    dll: t({
      en: 'DLL',
      ja: 'DLL',
    }),
    master: t({
      en: 'Master',
      ja: 'マスター',
    }),
    slave: t({
      en: 'Slave',
      ja: 'スレーブ',
    }),
    // Buttons
    install: t({
      en: 'Install',
      ja: 'インストール',
    }),
    reinstall: t({
      en: 'Reinstall',
      ja: '再インストール',
    }),
    // Success messages
    installationCompleted: t({
      en: 'Installation completed successfully',
      ja: 'インストールが正常に完了しました',
    }),
    successfullyInstalled: t({
      en: 'Successfully installed components to {count} installation(s)',
      ja: '{count}個のインストールにコンポーネントを正常にインストールしました',
    }),
    // Error messages
    installationFailed: t({
      en: 'Installation failed',
      ja: 'インストールに失敗しました',
    }),
    failedToInstall: t({
      en: 'Failed to install components to all {count} installation(s)',
      ja: '{count}個すべてのインストールへのコンポーネントのインストールに失敗しました',
    }),
    completedWithErrors: t({
      en: 'Completed with {successCount} success and {failCount} failure(s)',
      ja: '{successCount}個成功、{failCount}個失敗で完了しました',
    }),
    // Port status
    ports: t({
      en: 'Ports',
      ja: 'ポート',
    }),
    portMismatch: t({
      en: 'Mismatch',
      ja: '不一致',
    }),
    portMismatchTitle: t({
      en: 'Port configuration mismatch',
      ja: 'ポート設定の不一致',
    }),
    portMismatchDescription: t({
      en: 'EA ports do not match server config. Click "Install" to update.',
      ja: 'EAのポート設定がサーバーと一致しません。「インストール」で更新してください。',
    }),
  },
} satisfies DeclarationContent;

export default installationsPageContent;
