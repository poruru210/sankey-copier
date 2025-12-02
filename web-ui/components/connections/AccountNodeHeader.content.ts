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
    runtimeUnknownState: t({
      en: 'State {code}',
      ja: '状態{code}',
    }),
    runtimeTooltip: t({
      en: 'Status Engine: {state}',
      ja: 'ステータスエンジン: {state}',
    }),
    masterIntentOn: t({
      en: 'Intent:ON',
      ja: '意図:ON',
    }),
    masterIntentOff: t({
      en: 'Intent:OFF',
      ja: '意図:OFF',
    }),
    masterIntentTooltip: t({
      en: 'Web UI toggle intent',
      ja: 'Web UI トグルの意図',
    }),
    slaveIntentTooltip: t({
      en: 'Receiver intent',
      ja: 'スレーブ側のユーザー意図',
    }),
    slaveIntentOn: t({
      en: 'Intent:ON',
      ja: '意図:ON',
    }),
    slaveIntentOff: t({
      en: 'Intent:OFF',
      ja: '意図:OFF',
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
