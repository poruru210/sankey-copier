import { t, type DeclarationContent } from 'intlayer';

const pageContent = {
  key: 'home',
  content: {
    title: t({
      en: 'SANKEY Forex Copier',
      ja: 'SANKEY Forex Copier',
    }),
    description: t({
      en: 'Trade copying management dashboard',
      ja: 'トレードコピー管理ダッシュボード',
    }),
  },
} satisfies DeclarationContent;

export default pageContent;
