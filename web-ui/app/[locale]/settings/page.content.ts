import { t, type DeclarationContent } from 'intlayer';

// Internationalization content for the Settings page
// Provides Japanese and English translations
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
    // VictoriaLogs section
    vlogs: {
      title: t({
        en: 'VictoriaLogs',
        ja: 'VictoriaLogs',
      }),
      description: t({
        en: 'Configure logging to VictoriaLogs for centralized log management. Settings are automatically applied to all connected EAs.',
        ja: 'VictoriaLogsへのログ出力を設定します。設定は接続中のすべてのEAに自動的に適用されます。',
      }),
      enabled: t({
        en: 'Enable VictoriaLogs',
        ja: 'VictoriaLogsを有効化',
      }),
      enabledDescription: t({
        en: 'Send logs from all EAs to VictoriaLogs server',
        ja: 'すべてのEAからVictoriaLogsサーバーにログを送信',
      }),
      endpoint: t({
        en: 'Endpoint URL',
        ja: 'エンドポイントURL',
      }),
      endpointDescription: t({
        en: 'VictoriaLogs insert endpoint (e.g., http://localhost:9428/insert/jsonline)',
        ja: 'VictoriaLogsの挿入エンドポイント（例: http://localhost:9428/insert/jsonline）',
      }),
      batchSize: t({
        en: 'Batch Size',
        ja: 'バッチサイズ',
      }),
      batchSizeDescription: t({
        en: 'Number of log entries to batch before sending (1-10000)',
        ja: '送信前にバッチするログエントリ数（1-10000）',
      }),
      flushInterval: t({
        en: 'Flush Interval (seconds)',
        ja: 'フラッシュ間隔（秒）',
      }),
      flushIntervalDescription: t({
        en: 'Maximum time between log flushes (1-3600 seconds)',
        ja: 'ログフラッシュの最大間隔（1-3600秒）',
      }),
      statusActive: t({
        en: 'Logging Active',
        ja: 'ログ出力アクティブ',
      }),
      statusActiveDescription: t({
        en: 'VictoriaLogs integration is configured and will be enabled on save',
        ja: 'VictoriaLogs連携が設定されており、保存時に有効になります',
      }),
    },
    // Buttons
    buttons: {
      save: t({
        en: 'Save Changes',
        ja: '変更を保存',
      }),
      saving: t({
        en: 'Saving...',
        ja: '保存中...',
      }),
      refresh: t({
        en: 'Refresh',
        ja: '更新',
      }),
    },
    // Toast messages
    toast: {
      saveSuccess: t({
        en: 'Settings saved',
        ja: '設定を保存しました',
      }),
      saveSuccessDescription: t({
        en: 'VictoriaLogs settings have been updated and broadcast to all EAs',
        ja: 'VictoriaLogs設定が更新され、すべてのEAにブロードキャストされました',
      }),
      saveError: t({
        en: 'Failed to save',
        ja: '保存に失敗しました',
      }),
      saveErrorDescription: t({
        en: 'Could not save settings. Please try again.',
        ja: '設定を保存できませんでした。再試行してください。',
      }),
    },
  },
} satisfies DeclarationContent;

export default settingsPageContent;
