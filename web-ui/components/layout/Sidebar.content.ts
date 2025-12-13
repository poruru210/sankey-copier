import { t, type DeclarationContent } from 'intlayer';

const sidebarContent = {
  key: 'sidebar',
  content: {
    // Navigation items
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
    settings: t({
      en: 'Settings',
      ja: '設定',
    }),
    // Mobile menu
    menu: t({
      en: 'Menu',
      ja: 'メニュー',
    }),
    // Accessibility labels
    openMenu: t({
      en: 'Open menu',
      ja: 'メニューを開く',
    }),
    collapseSidebar: t({
      en: 'Collapse sidebar',
      ja: 'サイドバーを折りたたむ',
    }),
    expandSidebar: t({
      en: 'Expand sidebar',
      ja: 'サイドバーを展開',
    }),
  },
} satisfies DeclarationContent;

export default sidebarContent;
