import { t, type DeclarationContent } from 'intlayer';

// Internationalization content for the Settings page
// VictoriaLogs config is read-only from config.toml, only enabled toggle is available
const settingsPageContent = {
  key: 'settings-page',
  content: {
    title: t({
      en: 'Settings',
      ja: '設定',
    }),
    description: t({
      en: 'Configure global system settings and manage sites',
      ja: 'グローバルシステム設定とサイト管理',
    }),
    loading: t({
      en: 'Loading...',
      ja: '読み込み中...',
    }),
    errorTitle: t({
      en: 'Error',
      ja: 'エラー',
    }),
    // Sites section (Migrated from sites/page.content.ts)
    sites: {
      title: t({
        en: 'Site Management',
        ja: 'サイト管理',
      }),
      description: t({
        en: 'Manage your SANKEY Copier server connections',
        ja: 'SANKEY Copierサーバーの接続を管理',
      }),
      activeSite: t({
        en: 'Active Site',
        ja: 'アクティブなサイト',
      }),
      activeSiteDescription: t({
        en: 'Select the server you want to control',
        ja: '操作するサーバーを選択してください',
      }),
      sitesTitle: t({
        en: 'Sites',
        ja: 'サイト',
      }),
      addButton: t({
        en: 'Add Site',
        ja: 'サイトを追加',
      }),
      siteName: t({
        en: 'Site Name',
        ja: 'サイト名',
      }),
      siteUrl: t({
        en: 'Site URL',
        ja: 'サイトURL',
      }),
      siteNamePlaceholder: t({
        en: 'e.g., Local Server',
        ja: '例: ローカルサーバー',
      }),
      siteUrlPlaceholder: t({
        en: 'e.g., http://localhost:3000',
        ja: '例: http://localhost:3000',
      }),
      save: t({
        en: 'Save',
        ja: '保存',
      }),
      cancel: t({
        en: 'Cancel',
        ja: 'キャンセル',
      }),
      delete: t({
        en: 'Delete',
        ja: '削除',
      }),
      edit: t({
        en: 'Edit',
        ja: '編集',
      }),
      add: t({
        en: 'Add',
        ja: '追加',
      }),
      selected: t({
        en: 'Selected',
        ja: '選択中',
      }),
      connect: t({
        en: 'Connect',
        ja: '接続',
      }),
      manageSites: t({
        en: 'Edit List',
        ja: 'リストを編集',
      }),
      done: t({
        en: 'Done',
        ja: '完了',
      }),
      addNewSite: t({
        en: 'Add New Site',
        ja: '新しいサイトを追加',
      }),
      // Error messages
      errorSiteNameRequired: t({
        en: 'Site name is required',
        ja: 'サイト名を入力してください',
      }),
      errorSiteUrlRequired: t({
        en: 'Site URL is required',
        ja: 'URLを入力してください',
      }),
      errorInvalidUrl: t({
        en: 'Please enter a valid URL (e.g., http://localhost:3000)',
        ja: '有効なURLを入力してください（例: http://localhost:3000）',
      }),
      errorCannotDeleteLast: t({
        en: 'Cannot delete the last site',
        ja: '最後のサイトは削除できません',
      }),
      confirmDelete: t({
        en: 'Are you sure you want to delete "{siteName}"?',
        ja: '「{siteName}」を削除しますか？',
      }),
      // Info message
      infoMessage: t({
        en: '💡 You can register and switch between multiple SANKEY Copier servers. Settings are saved in your browser\'s localStorage.',
        ja: '💡 複数のSANKEY Copierサーバーを登録して切り替えることができます。設定はブラウザのlocalStorageに保存されます。',
      }),
    },
    // Not configured state
    notConfigured: {
      title: t({
        en: 'VictoriaLogs Not Configured',
        ja: 'VictoriaLogs未設定',
      }),
      description: t({
        en: 'VictoriaLogs is not configured in config.toml. Add the [victoria_logs] section to enable this feature.',
        ja: 'VictoriaLogsはconfig.tomlで設定されていません。この機能を有効にするには[victoria_logs]セクションを追加してください。',
      }),
      hint: t({
        en: 'Add the following section to your config.toml file:',
        ja: '以下のセクションをconfig.tomlファイルに追加してください:',
      }),
    },
    // VictoriaLogs section
    vlogs: {
      title: t({
        en: 'VictoriaLogs',
        ja: 'VictoriaLogs',
      }),
      description: t({
        en: 'VictoriaLogs configuration for centralized log management. Toggle enabled state to control log shipping.',
        ja: 'VictoriaLogsによる集中ログ管理の設定。有効状態を切り替えてログ送信を制御できます。',
      }),
      enabled: t({
        en: 'Enable VictoriaLogs',
        ja: 'VictoriaLogsを有効化',
      }),
      enabledDescription: t({
        en: 'Send logs from relay server and all EAs to VictoriaLogs',
        ja: 'リレーサーバーとすべてのEAからVictoriaLogsにログを送信',
      }),
      readOnlyTitle: t({
        en: 'Configuration from config.toml',
        ja: 'config.tomlからの設定',
      }),
      readOnlyDescription: t({
        en: 'The following settings are read from config.toml and cannot be changed here. To modify these values, edit config.toml and restart the server.',
        ja: '以下の設定はconfig.tomlから読み込まれ、ここでは変更できません。値を変更するにはconfig.tomlを編集してサーバーを再起動してください。',
      }),
      host: t({
        en: 'Host URL',
        ja: 'ホストURL',
      }),
      hostDescription: t({
        en: 'VictoriaLogs server URL (configured in config.toml)',
        ja: 'VictoriaLogsサーバーURL（config.tomlで設定）',
      }),
      batchSize: t({
        en: 'Batch Size',
        ja: 'バッチサイズ',
      }),
      batchSizeDescription: t({
        en: 'Number of log entries to batch before sending (configured in config.toml)',
        ja: '送信前にバッチするログエントリ数（config.tomlで設定）',
      }),
      flushInterval: t({
        en: 'Flush Interval (seconds)',
        ja: 'フラッシュ間隔（秒）',
      }),
      flushIntervalDescription: t({
        en: 'Maximum time between log flushes (configured in config.toml)',
        ja: 'ログフラッシュの最大間隔（config.tomlで設定）',
      }),
      source: t({
        en: 'Source',
        ja: 'ソース',
      }),
      sourceDescription: t({
        en: 'Log source identifier (configured in config.toml)',
        ja: 'ログソース識別子（config.tomlで設定）',
      }),
      statusActive: t({
        en: 'Logging Active',
        ja: 'ログ出力アクティブ',
      }),
      statusActiveDescription: t({
        en: 'VictoriaLogs integration is enabled. Logs are being sent to the configured endpoint.',
        ja: 'VictoriaLogs連携が有効です。ログは設定されたエンドポイントに送信されています。',
      }),
    },
    // Buttons
    buttons: {
      refresh: t({
        en: 'Refresh',
        ja: '更新',
      }),
    },
    // Toast messages
    toast: {
      toggleSuccess: t({
        en: 'Settings updated',
        ja: '設定を更新しました',
      }),
      enabledDescription: t({
        en: 'VictoriaLogs has been enabled. Logs will be sent to the configured endpoint.',
        ja: 'VictoriaLogsが有効になりました。ログは設定されたエンドポイントに送信されます。',
      }),
      disabledDescription: t({
        en: 'VictoriaLogs has been disabled. Logs will not be sent.',
        ja: 'VictoriaLogsが無効になりました。ログは送信されません。',
      }),
      toggleError: t({
        en: 'Failed to update',
        ja: '更新に失敗しました',
      }),
      toggleErrorDescription: t({
        en: 'Could not update VictoriaLogs settings. Please try again.',
        ja: 'VictoriaLogs設定を更新できませんでした。再試行してください。',
      }),
    },
    // ZeroMQ section
    zeromq: {
      title: t({
        en: 'ZeroMQ Ports',
        ja: 'ZeroMQポート',
      }),
      description: t({
        en: 'ZeroMQ port configuration for EA communication. Ports are read-only and managed by the server.',
        ja: 'EA通信用のZeroMQポート設定。ポートは読み取り専用でサーバーによって管理されます。',
      }),
      receiverPort: t({
        en: 'Receiver Port (PULL)',
        ja: 'レシーバーポート (PULL)',
      }),
      receiverPortDescription: t({
        en: 'Port for receiving messages from EAs (EA → Server)',
        ja: 'EAからのメッセージ受信用ポート（EA → Server）',
      }),
      senderPort: t({
        en: 'Sender Port (PUB)',
        ja: 'センダーポート (PUB)',
      }),
      senderPortDescription: t({
        en: 'Port for publishing trade signals and configuration to EAs (unified)',
        ja: 'EAへのトレードシグナルおよび設定配信用ポート（統合）',
      }),
      isDynamic: t({
        en: 'Dynamic Ports',
        ja: '動的ポート',
      }),
      isDynamicDescription: t({
        en: 'Ports are dynamically assigned by the server at startup',
        ja: 'ポートはサーバー起動時に動的に割り当てられます',
      }),
      isFixed: t({
        en: 'Fixed Ports',
        ja: '固定ポート',
      }),
      isFixedDescription: t({
        en: 'Ports are configured in config.toml',
        ja: 'ポートはconfig.tomlで設定されています',
      }),
      generatedAt: t({
        en: 'Generated At',
        ja: '生成日時',
      }),
      readOnlyTitle: t({
        en: 'Port Configuration',
        ja: 'ポート設定',
      }),
      readOnlyDescription: t({
        en: 'Ports are configured by the server and cannot be changed from the web UI. To use fixed ports, configure them in config.toml.',
        ja: 'ポートはサーバーによって設定され、Web UIからは変更できません。固定ポートを使用するにはconfig.tomlで設定してください。',
      }),
    },
  },
} satisfies DeclarationContent;

export default settingsPageContent;
