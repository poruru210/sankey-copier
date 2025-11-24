import { t, type DeclarationContent } from 'intlayer';

const tradeGroupsPageContent = {
  key: 'trade-groups-page',
  content: {
    title: t({
      en: 'Master Account Settings',
      ja: 'マスターアカウント設定',
    }),
    description: t({
      en: 'Manage symbol prefix/suffix settings for Master accounts',
      ja: 'マスターアカウントのシンボルprefix/suffix設定を管理',
    }),
    // Loading states
    loadingTradeGroups: t({
      en: 'Loading Master accounts...',
      ja: 'マスターアカウントを読み込み中...',
    }),
    // No trade groups
    noTradeGroupsFound: t({
      en: 'No Master accounts found.',
      ja: 'マスターアカウントが見つかりません。',
    }),
    noTradeGroupsDescription: t({
      en: 'Master accounts will appear here once Master EAs connect to the relay server.',
      ja: 'Master EAがリレーサーバーに接続すると、マスターアカウントがここに表示されます。',
    }),
    // Table headers
    masterAccount: t({
      en: 'Master Account',
      ja: 'マスターアカウント',
    }),
    symbolPrefix: t({
      en: 'Symbol Prefix',
      ja: 'シンボルPrefix',
    }),
    symbolSuffix: t({
      en: 'Symbol Suffix',
      ja: 'シンボルSuffix',
    }),
    configVersion: t({
      en: 'Config Version',
      ja: 'Config バージョン',
    }),
    updatedAt: t({
      en: 'Last Updated',
      ja: '最終更新',
    }),
    actions: t({
      en: 'Actions',
      ja: 'アクション',
    }),
    // Actions
    edit: t({
      en: 'Edit',
      ja: '編集',
    }),
    // Placeholders
    notSet: t({
      en: '(Not set)',
      ja: '(未設定)',
    }),
  },
} satisfies DeclarationContent;

export default tradeGroupsPageContent;
