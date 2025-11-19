import { t, type DeclarationContent } from 'intlayer';

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
      en: 'Receiver',
      ja: 'レシーバー',
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
      en: 'Source Accounts',
      ja: 'ソース・アカウント',
    }),
    receiverAccounts: t({
      en: 'Receiver Accounts',
      ja: 'レシーバー・アカウント',
    }),
    addConnection: t({
      en: 'Add Connection',
      ja: '接続の追加',
    }),
    connectedReceivers: t({
      en: 'Connected Receivers',
      ja: '接続中のレシーバー',
    }),
    connectedSources: t({
      en: 'Connected Sources',
      ja: '接続元ソース',
    }),
    settings: t({
      en: 'Settings',
      ja: '設定',
    }),
    lotLevel: t({
      en: 'Log Level',
      ja: 'ログレベル',
    }),
    allLeverage: t({
      en: 'All Leverage Operations',
      ja: 'すべてのレバレッジ操作',
    }),
    prefix: t({
      en: 'Prefix',
      ja: '前置き',
    }),
    addSource: t({
      en: 'Add Source',
      ja: 'ソースの追加',
    }),
    addReceiver: t({
      en: 'Add Receiver',
      ja: 'レシーバーの追加',
    }),
    hideSettings: t({
      en: 'Hide Settings',
      ja: '設定を非表示',
    }),
    showSettings: t({
      en: 'Show Settings',
      ja: '設定を表示',
    }),
    connectAllSources: t({
      en: 'Connect All Sources',
      ja: 'すべてのソースを接続',
    }),
    disableAllSources: t({
      en: 'Disable All Sources',
      ja: 'すべてのソースを無効化',
    }),
    allSourcesInactive: t({
      en: 'All sources are inactive',
      ja: 'すべてのソースが非アクティブです',
    }),
    someSourcesInactive: t({
      en: 'Some sources are inactive',
      ja: '一部のソースが非アクティブです',
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
      en: 'Receivers',
      ja: 'レシーバー数',
    }),
    sources: t({
      en: 'Sources',
      ja: 'ソース数',
    }),
    selectSource: t({
      en: 'Select Source',
      ja: 'ソースを選択',
    }),
    selectSourcePlaceholder: t({
      en: '-- Please select --',
      ja: '-- 選択してください --',
    }),
    tradingConnections: t({
      en: 'Trading Connections',
      ja: '取引接続',
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
  },
} satisfies DeclarationContent;

export default connectionsViewContent;
