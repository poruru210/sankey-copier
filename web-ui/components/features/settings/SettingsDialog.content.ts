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
    masterAccountLabel: t({
      en: 'Master Account (Copy From)',
      ja: 'マスター口座（コピー元）',
    }),
    masterAccountDescription: t({
      en: 'Account to copy trades from',
      ja: 'トレードをコピーする元の口座',
    }),
    slaveAccount: t({
      en: 'Slave Account ID',
      ja: 'スレーブアカウントID',
    }),
    slaveAccountLabel: t({
      en: 'Slave Account (Copy To)',
      ja: 'スレーブ口座（コピー先）',
    }),
    slaveAccountDescription: t({
      en: 'Account to copy trades to',
      ja: 'トレードをコピーする先の口座',
    }),
    copySettingsLabel: t({
      en: 'Copy Settings',
      ja: 'コピー設定',
    }),
    // Connection display (edit mode)
    connectionLabel: t({
      en: 'Connection',
      ja: '接続',
    }),
    connectionDescription: t({
      en: 'Account connection cannot be changed',
      ja: 'アカウント間の紐づけは変更できません',
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
    delete: t({
      en: 'Delete',
      ja: '削除',
    }),
    deleteConfirm: t({
      en: 'Delete this connection?',
      ja: '接続を削除しますか？',
    }),
    deleteConfirmTitle: t({
      en: 'Delete Connection',
      ja: '接続を削除',
    }),
    deleteConfirmDescription: t({
      en: 'Are you sure you want to delete this connection? This action cannot be undone.',
      ja: 'この接続を削除してもよろしいですか？この操作は取り消せません。',
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
    // Account details labels
    positionsLabel: t({
      en: 'Positions',
      ja: 'ポジション',
    }),
    lastUpdateLabel: t({
      en: 'Last update',
      ja: '最終更新',
    }),
    lastConnectionLabel: t({
      en: 'Last connection',
      ja: '最終接続',
    }),
    // Relative time labels
    timeAgoSeconds: t({
      en: '{0} sec ago',
      ja: '{0}秒前',
    }),
    timeAgoMinutes: t({
      en: '{0} min ago',
      ja: '{0}分前',
    }),
    timeAgoHours: t({
      en: '{0} hour ago',
      ja: '{0}時間前',
    }),
    timeAgoDays: t({
      en: '{0} day ago',
      ja: '{0}日前',
    }),
    // Master Settings Drawer
    masterSettingsTitle: t({
      en: 'Master Settings',
      ja: 'マスター設定',
    }),
    symbolFiltersGlobalTitle: t({
      en: 'Symbol Rules (Global)',
      ja: 'シンボルルール（グローバル）',
    }),
    symbolFiltersGlobalDescription: t({
      en: 'These settings apply to all slaves connected to this master.',
      ja: 'これらの設定はこのマスターに接続するすべてのスレーブに適用されます。',
    }),
    masterSymbolPrefixDescription: t({
      en: 'Master will remove this prefix when broadcasting symbols (e.g., pro.EURUSD → EURUSD)',
      ja: 'ブロードキャスト時にこのプレフィックスを削除します（例: pro.EURUSD → EURUSD）',
    }),
    masterSymbolSuffixDescription: t({
      en: 'Master will remove this suffix when broadcasting symbols (e.g., EURUSD.m → EURUSD)',
      ja: 'ブロードキャスト時にこのサフィックスを削除します（例: EURUSD.m → EURUSD）',
    }),
    settingsSavedSuccess: t({
      en: 'Settings saved successfully',
      ja: '設定を保存しました',
    }),
    settingsSaveFailed: t({
      en: 'Failed to save settings',
      ja: '設定の保存に失敗しました',
    }),
    saving: t({
      en: 'Saving...',
      ja: '保存中...',
    }),
    // Slave Settings Form / Symbol Rules
    symbolFiltersTitle: t({
      en: 'Symbol Rules',
      ja: 'シンボルルール',
    }),
    symbolFiltersDescription: t({
      en: 'Configure symbol name transformations for this connection.',
      ja: 'この接続のシンボル名変換を設定します。',
    }),
    symbolPrefix: t({
      en: 'Prefix',
      ja: 'プレフィックス',
    }),
    symbolPrefixDescription: t({
      en: 'Prefix to add to symbol names (e.g., EURUSD → pro.EURUSD)',
      ja: 'シンボル名に追加するプレフィックス（例: EURUSD → pro.EURUSD）',
    }),
    symbolPrefixPlaceholder: t({
      en: "e.g. 'pro.' or 'FX.'",
      ja: "例: 'pro.' または 'FX.'",
    }),
    symbolSuffix: t({
      en: 'Suffix',
      ja: 'サフィックス',
    }),
    symbolSuffixDescription: t({
      en: 'Suffix to add to symbol names (e.g., EURUSD → EURUSD.m)',
      ja: 'シンボル名に追加するサフィックス（例: EURUSD → EURUSD.m）',
    }),
    symbolSuffixPlaceholder: t({
      en: "e.g. '.m' or '-ECN'",
      ja: "例: '.m' または '-ECN'",
    }),
    symbolMappings: t({
      en: 'Mappings',
      ja: 'マッピング',
    }),
    symbolMappingsDescription: t({
      en: 'Map source symbols to target symbols for this connection.',
      ja: 'この接続のソースシンボルをターゲットシンボルにマッピングします。',
    }),
    copySettingsDescription: t({
      en: 'Configure how trades are copied.',
      ja: 'トレードのコピー方法を設定します。',
    }),
    // Lot Calculation Mode
    lotCalculationMode: t({
      en: 'Lot Calculation Mode',
      ja: 'ロット計算モード',
    }),
    lotCalculationModeDescription: t({
      en: 'How to calculate lot size for copied trades',
      ja: 'コピーされるトレードのロットサイズの計算方法',
    }),
    lotModeMultiplier: t({
      en: 'Fixed Multiplier',
      ja: '固定倍率',
    }),
    lotModeMultiplierDesc: t({
      en: 'Use fixed multiplier value',
      ja: '固定の倍率値を使用',
    }),
    lotModeMarginRatio: t({
      en: 'Margin Ratio',
      ja: '証拠金比率',
    }),
    lotModeMarginRatioDesc: t({
      en: 'Calculate based on equity ratio (slave/master)',
      ja: '証拠金比率（スレーブ/マスター）に基づいて計算',
    }),
    // Lot Filters
    lotFilterTitle: t({
      en: 'Lot Filter',
      ja: 'ロットフィルター',
    }),
    lotFilterDescription: t({
      en: 'Filter trades by source lot size. Leave empty for no filtering.',
      ja: 'コピー元のロットサイズでフィルタリング。空欄でフィルタリングなし。',
    }),
    sourceLotMin: t({
      en: 'Minimum Lot',
      ja: '最小ロット',
    }),
    sourceLotMinDescription: t({
      en: 'Skip trades with lot size smaller than this value',
      ja: 'この値より小さいロットのトレードをスキップ',
    }),
    sourceLotMinPlaceholder: t({
      en: 'e.g. 0.01',
      ja: '例: 0.01',
    }),
    sourceLotMax: t({
      en: 'Maximum Lot',
      ja: '最大ロット',
    }),
    sourceLotMaxDescription: t({
      en: 'Skip trades with lot size larger than this value',
      ja: 'この値より大きいロットのトレードをスキップ',
    }),
    sourceLotMaxPlaceholder: t({
      en: 'e.g. 10.0',
      ja: '例: 10.0',
    }),
    // Open Sync Policy
    syncPolicyTitle: t({
      en: 'Open Sync Policy',
      ja: 'オープン同期ポリシー',
    }),
    syncPolicyDescription: t({
      en: 'Configure how existing positions are synchronized when slave connects.',
      ja: 'スレーブ接続時の既存ポジション同期の設定。',
    }),
    // Sync Mode
    syncMode: t({
      en: 'Existing Position Sync',
      ja: '既存ポジション同期',
    }),
    syncModeDescription: t({
      en: 'How to handle existing master positions when slave connects',
      ja: 'スレーブ接続時のマスター既存ポジションの処理方法',
    }),
    syncModeSkip: t({
      en: "Don't Sync",
      ja: '同期しない',
    }),
    syncModeSkipDesc: t({
      en: 'Only copy new trades, ignore existing positions',
      ja: '新規トレードのみコピー、既存ポジションは無視',
    }),
    syncModeLimitOrder: t({
      en: 'Limit Order',
      ja: '指値で同期',
    }),
    syncModeLimitOrderDesc: t({
      en: "Sync at Master's open price with time limit",
      ja: 'マスターのオープン価格で指値注文（制限時間あり）',
    }),
    syncModeMarketOrder: t({
      en: 'Market Order',
      ja: '成行で同期',
    }),
    syncModeMarketOrderDesc: t({
      en: 'Sync immediately if price deviation is within limit',
      ja: '価格乖離が許容範囲内なら成行で即時同期',
    }),
    // Limit Order Expiry
    limitOrderExpiry: t({
      en: 'Limit Order Expiry (minutes)',
      ja: '指値注文の有効期限（分）',
    }),
    limitOrderExpiryDescription: t({
      en: 'Time limit for limit orders. 0 = Good Till Cancelled (GTC).',
      ja: '指値注文の有効時間。0 = 取消まで有効（GTC）。',
    }),
    limitOrderExpiryPlaceholder: t({
      en: 'e.g. 60 (0 = GTC)',
      ja: '例: 60（0 = GTC）',
    }),
    // Market Sync Max Pips
    marketSyncMaxPips: t({
      en: 'Max Price Deviation (pips)',
      ja: '最大価格乖離（pips）',
    }),
    marketSyncMaxPipsDescription: t({
      en: 'Skip sync if current price differs from open price by more than this value.',
      ja: '現在価格とオープン価格の乖離がこの値を超える場合、同期をスキップ。',
    }),
    marketSyncMaxPipsPlaceholder: t({
      en: 'e.g. 10.0',
      ja: '例: 10.0',
    }),
    // Max Slippage
    maxSlippage: t({
      en: 'Max Slippage (points)',
      ja: '最大スリッページ（ポイント）',
    }),
    maxSlippageDescription: t({
      en: 'Maximum allowed slippage when opening positions. Leave empty for default (30 points).',
      ja: 'ポジション建て時の最大許容スリッページ。空欄でデフォルト（30ポイント）。',
    }),
    maxSlippagePlaceholder: t({
      en: 'e.g. 30',
      ja: '例: 30',
    }),
    copyPendingOrders: t({
      en: 'Copy Pending Orders',
      ja: '待機注文をコピー',
    }),
    copyPendingOrdersDesc: t({
      en: 'Also copy limit and stop orders',
      ja: '指値・逆指値注文もコピーする',
    }),
    // Symbol Mapping Input
    sourceSymbol: t({
      en: 'Source',
      ja: '変換元',
    }),
    targetSymbol: t({
      en: 'Target',
      ja: '変換先',
    }),
    addMapping: t({
      en: 'Add Mapping',
      ja: 'マッピングを追加',
    }),
    removeMapping: t({
      en: 'Remove',
      ja: '削除',
    }),
    sourceSymbolPlaceholder: t({
      en: 'e.g. XAUUSD',
      ja: '例: XAUUSD',
    }),
    targetSymbolPlaceholder: t({
      en: 'e.g. GOLD',
      ja: '例: GOLD',
    }),
    // Connected Slaves Section
    connectedSlavesTitle: t({
      en: 'Connected Slaves',
      ja: '接続中のスレーブ',
    }),
    connectedSlavesDescription: t({
      en: 'Slaves currently connected to this master.',
      ja: 'このマスターに現在接続しているスレーブ。',
    }),
    noConnectedSlaves: t({
      en: 'No slaves connected to this master.',
      ja: 'このマスターに接続しているスレーブはありません。',
    }),
    // Create Connection Dialog - Steps
    stepAccounts: t({
      en: 'Accounts',
      ja: 'アカウント',
    }),
    stepAccountsDescription: t({
      en: 'Select Master & Slave',
      ja: 'マスターとスレーブを選択',
    }),
    stepMasterSettings: t({
      en: 'Master Settings',
      ja: 'マスター設定',
    }),
    stepMasterSettingsDescription: t({
      en: 'Global configuration',
      ja: 'グローバル設定',
    }),
    stepSlaveSettings: t({
      en: 'Slave Settings',
      ja: 'スレーブ設定',
    }),
    stepSlaveSettingsDescription: t({
      en: 'Copy configuration',
      ja: 'コピー設定',
    }),
    // Create Connection Dialog - Warnings/Alerts
    existingConnectionsWarningTitle: t({
      en: 'Existing Connections',
      ja: '既存の接続',
    }),
    existingConnectionsWarningDescription: t({
      en: 'This master has {count} existing slave(s). Changing these settings will affect all slaves connected to this master.',
      ja: 'このマスターには既に{count}つのスレーブが接続されています。設定を変更すると、接続されているすべてのスレーブに影響します。',
    }),
    detectedSettingsTitle: t({
      en: 'Detected Settings Available',
      ja: '推奨設定が見つかりました',
    }),
    detectedSettingsDescription: t({
      en: 'The EA detected the following symbol settings:',
      ja: 'EAが以下のシンボル設定を検出しました:',
    }),
    applyDetectedSettings: t({
      en: 'Apply Detected Settings',
      ja: '検出された設定を適用',
    }),
    applySettings: t({
      en: 'Apply Settings',
      ja: '設定を適用',
    }),
    // Common Actions
    next: t({
      en: 'Next',
      ja: '次へ',
    }),
    back: t({
      en: 'Back',
      ja: '戻る',
    }),
    loading: t({
      en: 'Loading...',
      ja: '読み込み中...',
    }),
    // Slave Settings Form - Magic Number Filter
    magicFilterTitle: t({
      en: 'Magic Number Filter',
      ja: 'マジックナンバーフィルター',
    }),
    magicFilterDescription: t({
      en: 'Filter which trades to copy based on magic number. Leave empty to copy all trades.',
      ja: 'マジックナンバーに基づいてコピートレードをフィルタリングします。空の場合はすべてのトレードをコピーします。',
    }),
    allowedMagicNumbers: t({
      en: 'Allowed Magic Numbers',
      ja: '許可するマジックナンバー',
    }),
    allowedMagicNumbersDescription: t({
      en: 'Comma-separated list of magic numbers to copy. Only trades with these magic numbers will be copied.',
      ja: 'コピーするマジックナンバーのカンマ区切りリスト。指定されたマジックナンバーのトレードのみコピーされます。',
    }),
    allowedMagicNumbersPlaceholder: t({
      en: 'e.g. 12345, 67890',
      ja: '例: 12345, 67890',
    }),
    // Slave Settings Form - Trade Execution
    tradeExecutionTitle: t({
      en: 'Trade Execution',
      ja: 'トレード実行',
    }),
    tradeExecutionDescription: t({
      en: 'Configure signal processing and order execution behavior.',
      ja: 'シグナル処理と注文実行の動作を設定します。',
    }),
    maxRetries: t({
      en: 'Max Retries',
      ja: '最大再試行回数',
    }),
    maxRetriesDescription: t({
      en: 'Maximum number of order retry attempts on failure.',
      ja: '注文失敗時の最大再試行回数。',
    }),
    maxSignalDelay: t({
      en: 'Max Signal Delay (ms)',
      ja: '最大シグナル遅延 (ms)',
    }),
    maxSignalDelayDescription: t({
      en: 'Maximum allowed signal delay in milliseconds. Signals older than this are skipped or handled based on the setting below.',
      ja: '許容される最大シグナル遅延（ミリ秒）。これより古いシグナルはスキップされるか、以下の設定に基づいて処理されます。',
    }),
    usePendingOrderForDelayed: t({
      en: 'Use Pending Order for Delayed Signals',
      ja: '遅延シグナルに待機注文を使用',
    }),
    usePendingOrderForDelayedDesc: t({
      en: 'Place limit order at original price instead of skipping',
      ja: 'スキップする代わりに元の価格で指値注文を出す',
    }),
    // Symbol Mapping Input
    mappingCheck: t({
      en: 'Mapping: {mapping}',
      ja: 'マッピング: {mapping}',
    }),
    prefix: t({
      en: 'Prefix',
      ja: 'プレフィックス',
    }),
    suffix: t({
      en: 'Suffix',
      ja: 'サフィックス',
    }),
  },
} satisfies DeclarationContent;

export default settingsDialogContent;
