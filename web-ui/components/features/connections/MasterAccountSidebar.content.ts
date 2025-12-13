import { t, type DeclarationContent } from 'intlayer';

const masterAccountSidebarContent = {
  key: 'master-account-sidebar',
  content: {
    filterAccounts: t({
      en: 'Master Accounts',
      ja: 'マスターアカウント',
    }),
    allAccounts: t({
      en: 'All Accounts',
      ja: 'すべてのアカウント',
    }),
    connections: t({
      en: 'connections',
      ja: '件の接続',
    }),
    connection: t({
      en: 'connection',
      ja: '件の接続',
    }),
    link: t({
      en: 'link',
      ja: '紐づけ',
    }),
    links: t({
      en: 'links',
      ja: '紐づけ',
    }),
    noMasterAccounts: t({
      en: 'No master accounts',
      ja: 'マスターアカウントがありません',
    }),
    online: t({
      en: 'Online',
      ja: 'オンライン',
    }),
    offline: t({
      en: 'Offline',
      ja: 'オフライン',
    }),
    viewingAccount: t({
      en: 'Viewing',
      ja: '表示中',
    }),
    clearFilter: t({
      en: 'Clear Filter',
      ja: 'フィルターをクリア',
    }),
    showingAll: t({
      en: 'Showing all connections',
      ja: 'すべての接続を表示中',
    }),
  },
} satisfies DeclarationContent;

export default masterAccountSidebarContent;
