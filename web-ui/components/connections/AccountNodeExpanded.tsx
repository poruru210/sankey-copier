import type { EaConnection, CopySettings } from '@/types';
import { AccountInfoSection } from './AccountInfoSection';
import { BalanceSection } from './BalanceSection';
import { ConnectionStatusSection } from './ConnectionStatusSection';
import { CopySettingsCarousel } from './CopySettingsCarousel';

interface AccountNodeExpandedProps {
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
    // Copy Settings Carousel content
    copySettings: string;
    lotMultiplier: string;
    marginRatio: string;
    reverseTrade: string;
    symbolRules: string;
    prefix: string;
    suffix: string;
    mappings: string;
    lotFilter: string;
    min: string;
    max: string;
    noSettings: string;
  };
}

/**
 * Expanded account node content showing detailed account information, balance, and connection status
 */
export function AccountNodeExpanded({
  connection,
  accountSettings,
  type,
  content,
}: AccountNodeExpandedProps) {
  return (
    <div className="border-t border-gray-200 dark:border-gray-700 cursor-move">
      <div className="px-2 md:px-3 py-2 md:py-3 bg-gray-50 dark:bg-gray-900/30">
        <div className="space-y-2 md:space-y-3 pointer-events-none">
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

          {/* Copy Settings Carousel - only for receiver (Slave) nodes */}
          {type === 'receiver' && accountSettings.length > 0 && (
            <div className="pointer-events-auto">
              <CopySettingsCarousel
                accountSettings={accountSettings}
                content={{
                  copySettings: content.copySettings,
                  lotMultiplier: content.lotMultiplier,
                  marginRatio: content.marginRatio,
                  reverseTrade: content.reverseTrade,
                  symbolRules: content.symbolRules,
                  prefix: content.prefix,
                  suffix: content.suffix,
                  mappings: content.mappings,
                  lotFilter: content.lotFilter,
                  min: content.min,
                  max: content.max,
                  noSettings: content.noSettings,
                  pageIndicator: '{current} / {total}',
                }}
              />
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
