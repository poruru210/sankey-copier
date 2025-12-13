import { t, type DeclarationContent } from 'intlayer';

const masterConfigDialogContent = {
  key: 'master-config-dialog',
  content: {
    title: t({
      en: 'Master EA Configuration',
      ja: 'マスターEA設定',
    }),
    masterAccountLabel: t({
      en: 'Master Account',
      ja: 'マスター口座',
    }),
    symbolTransformationTitle: t({
      en: 'Symbol Transformation (Global)',
      ja: 'シンボル変換（グローバル）',
    }),
    symbolTransformationDescription: t({
      en: 'These settings apply to all Slaves connected to this Master.',
      ja: 'これらの設定はこのマスターに接続しているすべてのスレーブに適用されます。',
    }),
    symbolPrefix: t({
      en: 'Symbol Prefix',
      ja: 'シンボルプレフィックス',
    }),
    symbolPrefixPlaceholder: t({
      en: "e.g. 'pro.' or 'FX.'",
      ja: "例: 'pro.' または 'FX.'",
    }),
    symbolPrefixDescription: t({
      en: 'Master will remove this prefix when broadcasting symbols (e.g., pro.EURUSD → EURUSD)',
      ja: 'ブロードキャスト時にこのプレフィックスを削除します（例: pro.EURUSD → EURUSD）',
    }),
    symbolSuffix: t({
      en: 'Symbol Suffix',
      ja: 'シンボルサフィックス',
    }),
    symbolSuffixPlaceholder: t({
      en: "e.g. '.m' or '-ECN'",
      ja: "例: '.m' または '-ECN'",
    }),
    symbolSuffixDescription: t({
      en: 'Master will remove this suffix when broadcasting symbols (e.g., EURUSD.m → EURUSD)',
      ja: 'ブロードキャスト時にこのサフィックスを削除します（例: EURUSD.m → EURUSD）',
    }),
    delete: t({
      en: 'Delete',
      ja: '削除',
    }),
    cancel: t({
      en: 'Cancel',
      ja: 'キャンセル',
    }),
    save: t({
      en: 'Save',
      ja: '保存',
    }),
    saving: t({
      en: 'Saving...',
      ja: '保存中...',
    }),
    deleting: t({
      en: 'Deleting...',
      ja: '削除中...',
    }),
    deleteConfirmTitle: t({
      en: 'Delete Master Configuration?',
      ja: 'マスター設定を削除しますか？',
    }),
    deleteConfirmDescription: t({
      en: 'This will remove the symbol transformation settings for this Master. All Slaves will use their individual symbol settings instead.',
      ja: 'このマスターのシンボル変換設定が削除されます。すべてのスレーブは個別のシンボル設定を使用するようになります。',
    }),
    loadError: t({
      en: 'Failed to load configuration',
      ja: '設定の読み込みに失敗しました',
    }),
    saveError: t({
      en: 'Failed to save configuration',
      ja: '設定の保存に失敗しました',
    }),
    deleteError: t({
      en: 'Failed to delete configuration',
      ja: '設定の削除に失敗しました',
    }),
  },
} satisfies DeclarationContent;

export default masterConfigDialogContent;
