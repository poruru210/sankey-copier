import { t, type DeclarationContent } from 'intlayer';

/**
 * Internationalization content for the ConnectionsView component
 * Provides Japanese and English translations for all UI strings
 */
const connectionsViewContent = {
  key: 'connections-view',
  content: {
    title: t({
      en: 'Copy Connections',
      ja: 'コピー接続',
    }),
    newConnection: t({
      en: 'New Connection',
      ja: '新規接続',
    }),
    master: t({
      en: 'Master',
      ja: 'マスター',
    }),
    slave: t({
      en: 'Slave',
      ja: 'スレーブ',
    }),
    online: t({
      en: 'Online',
      ja: 'オンライン',
    }),
    offline: t({
      en: 'Offline',
      ja: 'オフライン',
    }),
    enabled: t({
      en: 'Enabled',
      ja: '有効',
    }),
    disabled: t({
      en: 'Disabled',
      ja: '無効',
    }),
    lotMultiplier: t({
      en: 'Lot Multiplier',
      ja: 'ロット倍率',
    }),
    reverseTrade: t({
      en: 'Reverse Trade',
      ja: '売買反転',
    }),
    edit: t({
      en: 'Edit',
      ja: '編集',
    }),
    delete: t({
      en: 'Delete',
      ja: '削除',
    }),
    deleteConfirm: t({
      en: 'Are you sure you want to delete this connection?',
      ja: 'この接続設定を削除しますか？',
    }),
    cancel: t({
      en: 'Cancel',
      ja: 'キャンセル',
    }),
    confirm: t({
      en: 'Confirm',
      ja: '確認',
    }),
    sourceAccounts: t({
      en: 'Master Accounts',
      ja: 'マスター・アカウント',
    }),
    receiverAccounts: t({
      en: 'Slave Accounts',
      ja: 'スレーブ・アカウント',
    }),
    addConnection: t({
      en: 'Add Connection',
      ja: '接続の追加',
    }),
    connectedReceivers: t({
      en: 'Connected Slaves',
      ja: '接続中のスレーブ',
    }),
    connectedSources: t({
      en: 'Connected Masters',
      ja: '接続元マスター',
    }),
    settings: t({
      en: 'Settings',
      ja: '設定',
    }),
    allSourcesInactive: t({
      en: 'All masters are inactive',
      ja: 'すべてのマスターが非アクティブです',
    }),
    someSourcesInactive: t({
      en: 'Some masters are inactive',
      ja: '一部のマスターが非アクティブです',
    }),
    status: t({
      en: 'Status',
      ja: 'ステータス',
    }),
    role: t({
      en: 'Role',
      ja: 'ロール',
    }),
    lastHeartbeat: t({
      en: 'Last Heartbeat',
      ja: '最終ハートビート',
    }),
    accountNumber: t({
      en: 'Account',
      ja: '口座番号',
    }),
    balance: t({
      en: 'Balance',
      ja: '残高',
    }),
    equity: t({
      en: 'Equity',
      ja: '証拠金',
    }),
    broker: t({
      en: 'Broker',
      ja: 'ブローカー',
    }),
    server: t({
      en: 'Server',
      ja: 'サーバー',
    }),
    leverage: t({
      en: 'Leverage',
      ja: 'レバレッジ',
    }),
    noConnectionData: t({
      en: 'No connection data available. EA not connected.',
      ja: '接続データがありません。EAが接続されていません。',
    }),
    accountInfo: t({
      en: 'Account Info',
      ja: '口座情報',
    }),
    balanceInfo: t({
      en: 'Balance',
      ja: '残高情報',
    }),
    connectionInfo: t({
      en: 'Connection',
      ja: '接続情報',
    }),
    platform: t({
      en: 'Platform',
      ja: 'プラットフォーム',
    }),
    currency: t({
      en: 'Currency',
      ja: '通貨',
    }),
    receivers: t({
      en: 'Slaves',
      ja: 'スレーブ数',
    }),
    sources: t({
      en: 'Masters',
      ja: 'マスター数',
    }),
    refresh: t({
      en: 'Refresh',
      ja: '更新',
    }),
    createNewLink: t({
      en: 'Create New Link',
      ja: '新しい紐づけを作成',
    }),
    settingsCreated: t({
      en: 'Link created successfully',
      ja: '紐づけを作成しました',
    }),
    settingsUpdated: t({
      en: 'Settings updated successfully',
      ja: '設定を更新しました',
    }),
    saveFailed: t({
      en: 'Failed to save',
      ja: '保存に失敗しました',
    }),
    unknownError: t({
      en: 'An unknown error occurred',
      ja: '不明なエラーが発生しました',
    }),
    autoTradingDisabled: t({
      en: 'MT Auto-Trading is disabled. Enable it in MT terminal.',
      ja: 'MTの自動売買がOFFです。MTターミナルで有効にしてください。',
    }),
    deleteFailed: t({
      en: 'Failed to delete',
      ja: '削除に失敗しました',
    }),
    createFailed: t({
      en: 'Failed to create',
      ja: '作成に失敗しました',
    }),
    fixError: t({
      en: 'Fix Error',
      ja: 'エラーを修正',
    }),
    // Copy Settings Carousel content
    copySettings: t({
      en: 'Copy Settings',
      ja: 'コピー設定',
    }),
    marginRatio: t({
      en: 'Margin Ratio',
      ja: '証拠金比率',
    }),
    symbolRules: t({
      en: 'Symbol Rules',
      ja: 'シンボルルール',
    }),
    prefix: t({
      en: 'Prefix',
      ja: 'プレフィックス',
    }),
    suffix: t({
      en: 'Suffix',
      ja: 'サフィックス',
    }),
    mappings: t({
      en: 'Mappings',
      ja: 'マッピング',
    }),
    lotFilter: t({
      en: 'Lot Filter',
      ja: 'ロットフィルター',
    }),
    min: t({
      en: 'Min',
      ja: '最小',
    }),
    max: t({
      en: 'Max',
      ja: '最大',
    }),
    noSettings: t({
      en: 'No settings configured',
      ja: '設定がありません',
    }),
  },
} satisfies DeclarationContent;

export default connectionsViewContent;
