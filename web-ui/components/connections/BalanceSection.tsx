import type { EaConnection } from '@/types';

interface BalanceSectionProps {
  connection?: EaConnection;
  content: {
    balanceInfo: string;
    balance: string;
    equity: string;
    currency: string;
  };
}

/**
 * Balance information section showing balance, equity, and currency
 */
export function BalanceSection({ connection, content }: BalanceSectionProps) {
  return (
    <div className="space-y-1.5">
      <div className="flex items-start gap-2 mb-2">
        <span className="text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wide">
          {content.balanceInfo}
        </span>
      </div>
      <div className="h-px bg-gray-300 dark:bg-gray-600 -mt-1 mb-2"></div>

      <div className="grid grid-cols-2 gap-x-4 gap-y-1.5">
        <div className="flex flex-col">
          <span className="text-gray-500 dark:text-gray-500 text-[10px] uppercase tracking-wide">
            {content.balance}
          </span>
          <span
            className={`font-bold text-sm ${
              connection?.balance !== undefined
                ? 'text-blue-600 dark:text-blue-400'
                : 'text-gray-400 dark:text-gray-500'
            }`}
          >
            {connection?.balance !== undefined
              ? `${connection.balance.toLocaleString(undefined, {
                  minimumFractionDigits: 2,
                  maximumFractionDigits: 2,
                })}`
              : '-'}
          </span>
        </div>
        <div className="flex flex-col">
          <span className="text-gray-500 dark:text-gray-500 text-[10px] uppercase tracking-wide">
            {content.equity}
          </span>
          <span
            className={`font-bold text-sm ${
              connection?.equity !== undefined
                ? 'text-green-600 dark:text-green-400'
                : 'text-gray-400 dark:text-gray-500'
            }`}
          >
            {connection?.equity !== undefined
              ? `${connection.equity.toLocaleString(undefined, {
                  minimumFractionDigits: 2,
                  maximumFractionDigits: 2,
                })}`
              : '-'}
          </span>
        </div>
      </div>
      {connection?.currency && (
        <div className="text-[10px] text-gray-500 dark:text-gray-500 text-right">
          {content.currency}: {connection.currency}
        </div>
      )}
    </div>
  );
}
