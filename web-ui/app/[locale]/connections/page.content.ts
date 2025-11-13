import { t, type DeclarationContent } from 'intlayer';

/**
 * Internationalization content for the Connections home page
 * Provides Japanese and English translations
 */
const connectionsPageContent = {
  key: 'connections-page',
  content: {
    title: t({
      en: 'Trading Connections',
      ja: '取引接続',
    }),
    description: t({
      en: 'Manage and monitor your master-slave trading connections in real-time',
      ja: 'マスター・スレーブの取引接続をリアルタイムで管理・監視',
    }),
    loading: t({
      en: 'Loading...',
      ja: '読み込み中...',
    }),
  },
} satisfies DeclarationContent;

export default connectionsPageContent;
