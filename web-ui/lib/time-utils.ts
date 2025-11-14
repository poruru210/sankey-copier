/**
 * Time utility functions for displaying relative time
 */

export interface RelativeTimeResult {
  value: number;
  unit: 'seconds' | 'minutes' | 'hours' | 'days';
}

/**
 * Calculate relative time from a date string
 * @param dateString ISO 8601 date string
 * @returns Relative time result
 */
export function getRelativeTime(dateString: string): RelativeTimeResult {
  const now = new Date();
  const past = new Date(dateString);
  const diffInSeconds = Math.floor((now.getTime() - past.getTime()) / 1000);

  if (diffInSeconds < 60) {
    return { value: diffInSeconds, unit: 'seconds' };
  }

  const diffInMinutes = Math.floor(diffInSeconds / 60);
  if (diffInMinutes < 60) {
    return { value: diffInMinutes, unit: 'minutes' };
  }

  const diffInHours = Math.floor(diffInMinutes / 60);
  if (diffInHours < 24) {
    return { value: diffInHours, unit: 'hours' };
  }

  const diffInDays = Math.floor(diffInHours / 24);
  return { value: diffInDays, unit: 'days' };
}

/**
 * Format relative time for display
 * @param dateString ISO 8601 date string
 * @param labels Localized labels for time units
 * @returns Formatted relative time string (e.g., "2秒前", "5 minutes ago")
 */
export function formatRelativeTime(
  dateString: string,
  labels: {
    secondsAgo: string;
    minutesAgo: string;
    hoursAgo: string;
    daysAgo: string;
  }
): string {
  const { value, unit } = getRelativeTime(dateString);

  switch (unit) {
    case 'seconds':
      return labels.secondsAgo.replace('{0}', value.toString());
    case 'minutes':
      return labels.minutesAgo.replace('{0}', value.toString());
    case 'hours':
      return labels.hoursAgo.replace('{0}', value.toString());
    case 'days':
      return labels.daysAgo.replace('{0}', value.toString());
    default:
      return '';
  }
}
