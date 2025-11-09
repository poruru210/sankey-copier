import { t, type DeclarationContent } from 'intlayer';

const pageContent = {
  key: 'home',
  content: {
    title: t({
      en: 'SANKEY SANKEY Copier',
      ja: 'SANKEY SANKEY Copier',
    }),
    description: t({
      en: 'Trade copying management dashboard',
      ja: 'トレードコピー管理ダッシュボード',
    }),
  },
} satisfies DeclarationContent;

export default pageContent;
