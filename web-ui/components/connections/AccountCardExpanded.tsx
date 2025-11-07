import type { EaConnection, CopySettings } from '@/types';
import { AccountInfoSection } from './AccountInfoSection';
import { BalanceSection } from './BalanceSection';
import { ConnectionStatusSection } from './ConnectionStatusSection';

interface AccountCardExpandedProps {
  connection?: EaConnection;
  accountSettings: CopySettings[];
  type: 'source' | 'receiver';
  content: {
    accountInfo: string;
    accountNumber: string;
    platform: string;
    broker: string;
    leverage: string;
    server: string;
    balanceInfo: string;
    balance: string;
    equity: string;
    currency: string;
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
 * Expanded account card content showing detailed account information, balance, and connection status
 */
export function AccountCardExpanded({
  connection,
  accountSettings,
  type,
  content,
}: AccountCardExpandedProps) {
  return (
    <div className="border-t border-gray-200 dark:border-gray-700">
      <div className="px-2 md:px-3 py-2 md:py-3 bg-gray-50 dark:bg-gray-900/30">
        <div className="space-y-2 md:space-y-3">
          {/* Account Info Section */}
          <AccountInfoSection
            connection={connection}
            content={{
              accountInfo: content.accountInfo,
              accountNumber: content.accountNumber,
              platform: content.platform,
              broker: content.broker,
              leverage: content.leverage,
              server: content.server,
            }}
          />

          {/* Balance Section */}
          <BalanceSection
            connection={connection}
            content={{
              balanceInfo: content.balanceInfo,
              balance: content.balance,
              equity: content.equity,
              currency: content.currency,
            }}
          />

          {/* Connection Status Section */}
          <ConnectionStatusSection
            connection={connection}
            accountSettings={accountSettings}
            type={type}
            content={{
              connectionInfo: content.connectionInfo,
              status: content.status,
              online: content.online,
              offline: content.offline,
              receivers: content.receivers,
              sources: content.sources,
              lastHeartbeat: content.lastHeartbeat,
            }}
          />
        </div>
      </div>
    </div>
  );
}
