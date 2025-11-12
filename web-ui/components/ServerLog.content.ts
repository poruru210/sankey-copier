import { t, type DeclarationContent } from 'intlayer';

const serverLogContent = {
  key: 'server-log',
  content: {
    title: t({
      en: 'Server Logs',
      ja: 'サーバーログ',
    }),
    noLogs: t({
      en: 'No logs available',
      ja: 'ログがありません',
    }),
    refreshButton: t({
      en: 'Refresh',
      ja: '更新',
    }),
    loading: t({
      en: 'Loading logs...',
      ja: 'ログを読み込み中...',
    }),
    error: t({
      en: 'Error',
      ja: 'エラー',
    }),
    toggleLabel: t({
      en: 'Expand',
      ja: '展開',
    }),
    closeLabel: t({
      en: 'Close',
      ja: '閉じる',
    }),
  },
} satisfies DeclarationContent;

export default serverLogContent;
