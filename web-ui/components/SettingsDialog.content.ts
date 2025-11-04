import { t, type DeclarationContent } from 'intlayer';

const settingsDialogContent = {
  key: 'settings-dialog',
  content: {
    createTitle: t({
      en: 'Create Connection',
      ja: '接続を作成',
    }),
    editTitle: t({
      en: 'Edit Connection',
      ja: '接続を編集',
    }),
    masterAccount: t({
      en: 'Master Account ID',
      ja: 'マスターアカウントID',
    }),
    slaveAccount: t({
      en: 'Receiver Account ID',
      ja: 'レシーバーアカウントID',
    }),
    lotMultiplier: t({
      en: 'Lot Multiplier',
      ja: 'ロット倍率',
    }),
    reverseTrade: t({
      en: 'Reverse Trade',
      ja: '売買反転',
    }),
    reverseDescription: t({
      en: 'Reverse buy/sell orders',
      ja: '売買を反転する',
    }),
    cancel: t({
      en: 'Cancel',
      ja: 'キャンセル',
    }),
    create: t({
      en: 'Create',
      ja: '作成',
    }),
    save: t({
      en: 'Save',
      ja: '保存',
    }),
  },
} satisfies DeclarationContent;

export default settingsDialogContent;
