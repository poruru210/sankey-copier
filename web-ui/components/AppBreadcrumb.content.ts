import { t, type DeclarationContent } from 'intlayer';

// Breadcrumb content for navigation labels
const breadcrumbContent = {
  key: 'breadcrumb',
  content: {
    home: t({
      en: 'Home',
      ja: 'ホーム',
    }),
    connections: t({
      en: 'Connections',
      ja: '接続',
    }),
    installations: t({
      en: 'Installations',
      ja: 'インストール',
    }),
    sites: t({
      en: 'Sites',
      ja: 'サイト',
    }),
    tradeGroups: t({
      en: 'Trade Groups',
      ja: 'トレードグループ',
    }),
    settings: t({
      en: 'Settings',
      ja: '設定',
    }),
  },
} satisfies DeclarationContent;

export default breadcrumbContent;
