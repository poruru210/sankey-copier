import { t, type DeclarationContent } from 'intlayer';

const accountNodeHeaderContent = {
  key: 'account-node-header',
  content: {
    runtimeManualOff: t({
      en: 'Manual OFF',
      ja: '手動OFF',
    }),
    runtimeStandby: t({
      en: 'Standby',
      ja: '待機',
    }),
    runtimeStreaming: t({
      en: 'Streaming',
      ja: '配信中',
    }),
    runtimeReceiving: t({
      en: 'Receiving',
      ja: '受信中',
    }),
    runtimeUnknownState: t({
      en: 'State {code}',
      ja: '状態{code}',
    }),
    runtimeTooltip: t({
      en: 'Status Engine: {state}',
      ja: 'ステータスエンジン: {state}',
    }),
    intentSyncing: t({
      en: 'Applying change…',
      ja: '変更を反映中…',
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
} satisfies DeclarationContent;

export default accountNodeHeaderContent;
