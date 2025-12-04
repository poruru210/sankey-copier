import { t, type DeclarationContent } from 'intlayer';

const settingsEdgeContent = {
  key: 'settings-edge',
  content: {
    connectionSettingsTitle: t({
      en: 'Connection Settings',
      ja: '接続設定',
    }),
  },
} satisfies DeclarationContent;

export default settingsEdgeContent;
