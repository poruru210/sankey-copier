import type { EaConnection, CopySettings } from '@/types';

interface ConnectionStatusSectionProps {
  connection?: EaConnection;
  accountSettings: CopySettings[];
  type: 'source' | 'receiver';
  content: {
    connectionInfo: string;
    status: string;
    online: string;
    offline: string;
    receivers: string;
    sources: string;
    lastHeartbeat: string;
  };
}

/**
 * Connection status section showing online status, connected accounts count, and last heartbeat
 */
export function ConnectionStatusSection({
  connection,
  accountSettings,
  type,
  content,
}: ConnectionStatusSectionProps) {
  return (
    <div className="space-y-1.5">
      <div className="flex items-start gap-2 mb-2">
        <span className="text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wide">
          {content.connectionInfo}
        </span>
      </div>
      <div className="h-px bg-gray-300 dark:bg-gray-600 -mt-1 mb-2"></div>

      <div className="grid grid-cols-2 gap-x-2 md:gap-x-3 lg:gap-x-4 gap-y-1.5 text-xs">
        <div className="flex flex-col min-w-0">
          <span className="text-gray-500 dark:text-gray-500 text-[10px] uppercase tracking-wide">
            {content.status}
          </span>
          <span
            className={`font-semibold truncate ${
              connection?.is_online ?? connection?.status === 'Online'
                ? 'text-green-600 dark:text-green-400'
                : 'text-gray-500 dark:text-gray-500'
            }`}
          >
            {connection?.is_online ?? connection?.status === 'Online'
              ? content.online
              : content.offline}
          </span>
        </div>
        <div className="flex flex-col min-w-0">
          <span className="text-gray-500 dark:text-gray-500 text-[10px] uppercase tracking-wide">
            {type === 'source' ? content.receivers : content.sources}
          </span>
          <span className="font-semibold text-gray-900 dark:text-gray-100 truncate">
            {accountSettings.length}
          </span>
        </div>
      </div>

      {connection?.last_heartbeat && (
        <div className="flex flex-col text-xs pt-1 min-w-0">
          <span className="text-gray-500 dark:text-gray-500 text-[10px] uppercase tracking-wide">
            {content.lastHeartbeat}
          </span>
          <span className="font-medium text-gray-900 dark:text-gray-100 text-[10px] truncate" title={new Date(connection.last_heartbeat).toLocaleString()}>
            {new Date(connection.last_heartbeat).toLocaleString()}
          </span>
        </div>
      )}
    </div>
  );
}
