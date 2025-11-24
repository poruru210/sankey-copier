import { t, type DeclarationContent } from 'intlayer';

const tradeGroupDetailPageContent = {
  key: 'trade-group-detail-page',
  content: {
    title: t({
      en: 'Master Account Settings',
      ja: 'マスターアカウント設定',
    }),
    description: t({
      en: 'Configure symbol prefix and suffix for this Master account',
      ja: 'このマスターアカウントのシンボルprefixとsuffixを設定',
    }),
    // Loading states
    loadingTradeGroup: t({
      en: 'Loading Master account settings...',
      ja: 'マスターアカウント設定を読み込み中...',
    }),
    saving: t({
      en: 'Saving...',
      ja: '保存中...',
    }),
    // Form fields
    masterAccountLabel: t({
      en: 'Master Account ID',
      ja: 'マスターアカウントID',
    }),
    symbolPrefixLabel: t({
      en: 'Symbol Prefix',
      ja: 'シンボルPrefix',
    }),
    symbolPrefixPlaceholder: t({
      en: 'e.g., pro.',
      ja: '例: pro.',
    }),
    symbolPrefixDescription: t({
      en: 'Prefix to add before symbol names when sending to Slave EAs',
      ja: 'Slave EAへ送信する際にシンボル名の前に追加するprefix',
    }),
    symbolSuffixLabel: t({
      en: 'Symbol Suffix',
      ja: 'シンボルSuffix',
    }),
    symbolSuffixPlaceholder: t({
      en: 'e.g., .m',
      ja: '例: .m',
    }),
    symbolSuffixDescription: t({
      en: 'Suffix to add after symbol names when sending to Slave EAs',
      ja: 'Slave EAへ送信する際にシンボル名の後に追加するsuffix',
    }),
    configVersionLabel: t({
      en: 'Config Version',
      ja: 'Configバージョン',
    }),
    lastUpdatedLabel: t({
      en: 'Last Updated',
      ja: '最終更新',
    }),
    // Buttons
    save: t({
      en: 'Save Changes',
      ja: '変更を保存',
    }),
    cancel: t({
      en: 'Cancel',
      ja: 'キャンセル',
    }),
    backToList: t({
      en: 'Back to List',
      ja: '一覧に戻る',
    }),
    // Success messages
    saveSuccess: t({
      en: 'Settings saved successfully',
      ja: '設定を保存しました',
    }),
    // Error messages
    saveFailed: t({
      en: 'Failed to save settings',
      ja: '設定の保存に失敗しました',
    }),
    loadFailed: t({
      en: 'Failed to load Master account settings',
      ja: 'マスターアカウント設定の読み込みに失敗しました',
    }),
    notFound: t({
      en: 'Master account not found',
      ja: 'マスターアカウントが見つかりません',
    }),
  },
} satisfies DeclarationContent;

export default tradeGroupDetailPageContent;
