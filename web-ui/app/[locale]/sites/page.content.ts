import { t, type DeclarationContent } from 'intlayer';

const sitesPageContent = {
  key: 'sites-page',
  content: {
    title: t({
      en: 'Site Management',
      ja: 'サイト管理',
    }),
    description: t({
      en: 'Manage your SANKEY Copier server connections',
      ja: 'SANKEY Copierサーバーの接続を管理',
    }),
    registeredSites: t({
      en: 'Registered Sites',
      ja: '登録済みサイト',
    }),
    addButton: t({
      en: 'Add Site',
      ja: 'サイトを追加',
    }),
    siteName: t({
      en: 'Site Name',
      ja: 'サイト名',
    }),
    siteUrl: t({
      en: 'Site URL',
      ja: 'サイトURL',
    }),
    siteNamePlaceholder: t({
      en: 'e.g., Local Server',
      ja: '例: ローカルサーバー',
    }),
    siteUrlPlaceholder: t({
      en: 'e.g., http://localhost:3000',
      ja: '例: http://localhost:3000',
    }),
    save: t({
      en: 'Save',
      ja: '保存',
    }),
    cancel: t({
      en: 'Cancel',
      ja: 'キャンセル',
    }),
    delete: t({
      en: 'Delete',
      ja: '削除',
    }),
    edit: t({
      en: 'Edit',
      ja: '編集',
    }),
    add: t({
      en: 'Add',
      ja: '追加',
    }),
    selected: t({
      en: 'Selected',
      ja: '選択中',
    }),
    addNewSite: t({
      en: 'Add New Site',
      ja: '新しいサイトを追加',
    }),
    // Error messages
    errorSiteNameRequired: t({
      en: 'Site name is required',
      ja: 'サイト名を入力してください',
    }),
    errorSiteUrlRequired: t({
      en: 'Site URL is required',
      ja: 'URLを入力してください',
    }),
    errorInvalidUrl: t({
      en: 'Please enter a valid URL (e.g., http://localhost:3000)',
      ja: '有効なURLを入力してください（例: http://localhost:3000）',
    }),
    errorCannotDeleteLast: t({
      en: 'Cannot delete the last site',
      ja: '最後のサイトは削除できません',
    }),
    confirmDelete: t({
      en: 'Are you sure you want to delete "{siteName}"?',
      ja: '「{siteName}」を削除しますか？',
    }),
    // Info message
    infoMessage: t({
      en: '💡 You can register and switch between multiple SANKEY Copier servers. Settings are saved in your browser\'s localStorage.',
      ja: '💡 複数のSANKEY Copierサーバーを登録して切り替えることができます。設定はブラウザのlocalStorageに保存されます。',
    }),
  },
} satisfies DeclarationContent;

export default sitesPageContent;
