import { t, type DeclarationContent } from 'intlayer';

const activityLogContent = {
  key: 'activity-log',
  content: {
    title: t({
      en: 'Recent Activity',
      ja: '最近のアクティビティ',
    }),
    noActivity: t({
      en: 'No activity yet',
      ja: 'まだアクティビティがありません',
    }),
  },
} satisfies DeclarationContent;

export default activityLogContent;
