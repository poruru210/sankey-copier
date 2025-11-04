import { AlertCircle, Edit2 } from 'lucide-react';
import type { AccountInfo } from '@/types';

interface ErrorWarningAlertProps {
  account: AccountInfo;
  fixErrorLabel: string;
}

/**
 * Error or warning alert banner shown at the bottom of account cards
 */
export function ErrorWarningAlert({ account, fixErrorLabel }: ErrorWarningAlertProps) {
  if (!account.hasError && !account.hasWarning) {
    return null;
  }

  return (
    <div
      className={`${
        account.hasError
          ? 'bg-pink-50 dark:bg-pink-900/20 border-pink-100 dark:border-pink-800'
          : 'bg-yellow-50 dark:bg-yellow-900/20 border-yellow-100 dark:border-yellow-800'
      } px-3 py-2 flex items-center gap-2 border-t`}
    >
      <AlertCircle
        className={`w-4 h-4 ${
          account.hasError ? 'text-red-500' : 'text-yellow-600'
        } flex-shrink-0`}
      />
      <span className="text-xs text-gray-900 dark:text-gray-300 flex-1">
        {account.errorMsg}
      </span>
      <button
        className={`${
          account.hasError
            ? 'bg-red-500 hover:bg-red-600'
            : 'bg-yellow-500 hover:bg-yellow-600'
        } text-white px-3 py-1.5 rounded text-xs font-medium flex items-center gap-1.5 transition-colors whitespace-nowrap shadow-sm`}
      >
        <Edit2 className="w-3 h-3" />
        {fixErrorLabel}
      </button>
    </div>
  );
}
