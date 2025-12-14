import { t, type Dictionary } from 'intlayer';

const accountNodeHeaderContent = {
  key: 'account-node-header',
  content: {
    runtimeManualOff: t({
      en: 'MANUAL OFF',
      ja: '手動停止',
    }),
    runtimeStandby: t({
      en: 'STANDBY',
      ja: '待機中',
    }),
    runtimeReceiving: t({
      en: 'RECEIVING',
      ja: '受信中',
    }),
    runtimeStreaming: t({
      en: 'STREAMING',
      ja: '配信中',
    }),
    runtimeUnknownState: t({
      en: 'STATE: {code}',
      ja: '状態: {code}',
    }),
    runtimeTooltip: t({
      en: 'Runtime State: {state}',
      ja: '実行状態: {state}',
    }),
    intentSyncing: t({
      en: 'Syncing...',
      ja: '同期中...',
    }),
    masterSettingsTitle: t({
      en: 'Master Settings',
      ja: 'マスター設定',
    }),
    connectionSettingsTitle: t({
      en: 'Connection Settings',
      ja: '接続設定',
    }),
  },
} satisfies Dictionary;

export default accountNodeHeaderContent;
