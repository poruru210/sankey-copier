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
    saveAndEnable: t({
      en: 'Save and Enable',
      ja: '保存して有効化',
    }),
    backToSelector: t({
      en: 'Back to Selector',
      ja: '選択に戻る',
    }),
    // Validation messages
    errorTitle: t({
      en: 'Error',
      ja: 'エラー',
    }),
    warningTitle: t({
      en: 'Warning',
      ja: '警告',
    }),
    // Account selector
    selectMasterAccount: t({
      en: 'Select Master Account',
      ja: 'マスター口座を選択',
    }),
    selectSlaveAccount: t({
      en: 'Select Slave Account',
      ja: 'スレーブ口座を選択',
    }),
    connectedMasterAccounts: t({
      en: 'Connected Master Accounts',
      ja: '接続中のマスター口座',
    }),
    connectedSlaveAccounts: t({
      en: 'Connected Slave Accounts',
      ja: '接続中のスレーブ口座',
    }),
    timeoutAccounts: t({
      en: 'Timeout Accounts',
      ja: 'タイムアウト中の口座',
    }),
    offlineAccounts: t({
      en: 'Offline Accounts',
      ja: 'オフラインの口座',
    }),
    manualInput: t({
      en: '📝 Manual Input...',
      ja: '📝 手動入力...',
    }),
    noConnectedAccounts: t({
      en: 'No connected accounts. Please start EA and connect.',
      ja: '接続中の口座がありません。EAを起動して接続してください。',
    }),
    noConnectedMasterAccounts: t({
      en: 'No connected master accounts. Please start EA and connect.',
      ja: '接続中のマスター口座がありません。EAを起動して接続してください。',
    }),
    noConnectedSlaveAccounts: t({
      en: 'No connected slave accounts. Please start EA and connect.',
      ja: '接続中のスレーブ口座がありません。EAを起動して接続してください。',
    }),
    // Lot multiplier description
    lotMultiplierDescription: t({
      en: 'Enter 0.5 to copy with 0.5 times the lot of master',
      ja: 'マスターの0.5倍のロットでコピーする場合は0.5を入力',
    }),
    // Validation messages
    validationSelectMasterAccount: t({
      en: 'Please select master account',
      ja: 'マスター口座を選択してください',
    }),
    validationSelectSlaveAccount: t({
      en: 'Please select slave account',
      ja: 'スレーブ口座を選択してください',
    }),
    validationSameAccountError: t({
      en: 'Cannot select the same account for both master and slave',
      ja: 'マスターとスレーブに同じ口座は選択できません',
    }),
    validationLotMultiplierPositive: t({
      en: 'Lot multiplier must be greater than 0',
      ja: 'ロット倍率は0より大きい値を指定してください',
    }),
    validationLotMultiplierTooSmall: t({
      en: 'Lot multiplier is very small (recommended: 0.01 or higher)',
      ja: 'ロット倍率が非常に小さいです（推奨: 0.01以上）',
    }),
    validationLotMultiplierTooLarge: t({
      en: 'Lot multiplier is very large (recommended: 100 or lower)',
      ja: 'ロット倍率が非常に大きいです（推奨: 100以下）',
    }),
    validationDuplicateSettings: t({
      en: 'This combination already exists (Setting ID: {id}, {status})',
      ja: 'この組み合わせは既に存在します（設定ID: {id}、{status}）',
    }),
    validationStatusEnabled: t({
      en: 'enabled',
      ja: '有効',
    }),
    validationStatusDisabled: t({
      en: 'disabled',
      ja: '無効',
    }),
    validationAccountOffline: t({
      en: '{account} is currently offline. Trades will not be copied until EA connects.',
      ja: '{account}は現在オフラインです。EAが接続するまでトレードはコピーされません。',
    }),
    validationAccountTimeout: t({
      en: '{account} response is delayed. Please check the connection status.',
      ja: '{account}の応答が遅延しています。接続状態を確認してください。',
    }),
    validationAccountNotInList: t({
      en: '{account} is not found in the connection list. Please start EA.',
      ja: '{account}は接続リストに見つかりません。EAを起動してください。',
    }),
    validationCircularReference: t({
      en: 'Potential circular reference: Connection {slave} → {master} already exists (not recommended)',
      ja: '循環参照の可能性があります: {slave} → {master}の接続が既に存在します（推奨されません）',
    }),
  },
} satisfies DeclarationContent;

export default settingsDialogContent;
