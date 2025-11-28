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
      en: 'Configure global system settings',
      ja: 'グローバルシステム設定を構成',
    }),
    loading: t({
      en: 'Loading...',
      ja: '読み込み中...',
    }),
    errorTitle: t({
      en: 'Error',
      ja: 'エラー',
    }),
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
  },
} satisfies DeclarationContent;

export default settingsPageContent;
