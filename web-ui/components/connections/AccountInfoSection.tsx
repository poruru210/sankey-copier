import type { EaConnection } from '@/types';

interface AccountInfoSectionProps {
  connection?: EaConnection;
  content: {
    accountInfo: string;
    accountNumber: string;
    platform: string;
    broker: string;
    leverage: string;
    server: string;
  };
}

/**
 * Account information section showing account number, platform, broker, leverage, and server
 */
export function AccountInfoSection({ connection, content }: AccountInfoSectionProps) {
  return (
    <div className="space-y-1.5">
      <div className="flex items-start gap-2 mb-2">
        <span className="text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wide">
          {content.accountInfo}
        </span>
      </div>
      <div className="h-px bg-gray-300 dark:bg-gray-600 -mt-1 mb-2"></div>

      <div className="grid grid-cols-2 gap-x-4 gap-y-1.5 text-xs">
        <div className="flex flex-col">
          <span className="text-gray-500 dark:text-gray-500 text-[10px] uppercase tracking-wide">
            {content.accountNumber}
          </span>
          <span className="font-medium text-gray-900 dark:text-gray-100">
            {connection?.account_number || '-'}
          </span>
        </div>
        <div className="flex flex-col">
          <span className="text-gray-500 dark:text-gray-500 text-[10px] uppercase tracking-wide">
            {content.platform}
          </span>
          <span className="font-medium text-gray-900 dark:text-gray-100">
            {connection?.platform || '-'}
          </span>
        </div>
        <div className="flex flex-col">
          <span className="text-gray-500 dark:text-gray-500 text-[10px] uppercase tracking-wide">
            {content.broker}
          </span>
          <span className="font-medium text-gray-900 dark:text-gray-100">
            {connection?.broker || '-'}
          </span>
        </div>
        <div className="flex flex-col">
          <span className="text-gray-500 dark:text-gray-500 text-[10px] uppercase tracking-wide">
            {content.leverage}
          </span>
          <span className="font-medium text-gray-900 dark:text-gray-100">
            {connection?.leverage ? `1:${connection.leverage}` : '-'}
          </span>
        </div>
      </div>

      <div className="flex flex-col text-xs pt-1">
        <span className="text-gray-500 dark:text-gray-500 text-[10px] uppercase tracking-wide">
          {content.server}
        </span>
        <span
          className="font-medium text-gray-900 dark:text-gray-100 truncate"
          title={connection?.server || '-'}
        >
          {connection?.server || '-'}
        </span>
      </div>
    </div>
  );
}
