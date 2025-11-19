import { AlertCircle } from 'lucide-react';
import type { AccountInfo } from '@/types';

interface ErrorWarningAlertProps {
  account: AccountInfo;
}

/**
 * Error or warning alert banner shown at the bottom of account cards
 */
export function ErrorWarningAlert({ account }: ErrorWarningAlertProps) {
  if (!account.hasError && !account.hasWarning) {
    return null;
  }

  return (
    <div
      className={`${account.hasError
        ? 'bg-pink-50 dark:bg-pink-900/20 border-pink-100 dark:border-pink-800'
        : 'bg-yellow-50 dark:bg-yellow-900/20 border-yellow-100 dark:border-yellow-800'
        } px-3 py-2 flex items-center gap-2 border-t`}
    >
      <AlertCircle
        className={`w-4 h-4 ${account.hasError ? 'text-red-500' : 'text-yellow-600'
          } flex-shrink-0`}
      />
      <span className="text-xs text-gray-900 dark:text-gray-300 flex-1">
        {account.errorMsg}
      </span>
    </div>
  );
}
