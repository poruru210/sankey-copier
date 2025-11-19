import React from 'react';
import type { AccountInfo, EaConnection, CopySettings } from '@/types';
import { StatusIndicatorBar } from './StatusIndicatorBar';
import { AccountCardHeader } from './AccountCardHeader';
import { AccountCardExpanded } from './AccountCardExpanded';
import { ErrorWarningAlert } from './ErrorWarningAlert';

interface AccountCardProps {
  account: AccountInfo;
  connection?: EaConnection;
  accountSettings: CopySettings[];
  onToggle: () => void;
  onToggleEnabled?: (enabled: boolean) => void;
  onEditSetting?: (setting: CopySettings) => void;
  onDeleteSetting?: (setting: CopySettings) => void;
  type: 'source' | 'receiver';
  onMouseEnter?: () => void;
  onMouseLeave?: () => void;
  isHighlighted?: boolean;
  hoveredSourceId: string | null;
  hoveredReceiverId: string | null;
  selectedSourceId: string | null;
  isMobile: boolean;
  content: {
    settings: string;
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
 * Account card component showing account information with expandable details
 */
export const AccountCard = React.memo(
  ({
    account,
    connection,
    accountSettings,
    onToggle,
    onToggleEnabled,
    onEditSetting,
    onDeleteSetting,
    type,
    onMouseEnter,
    onMouseLeave,
    isHighlighted,
    hoveredSourceId,
    hoveredReceiverId,
    selectedSourceId,
    isMobile,
    content,
  }: AccountCardProps) => {
    // Determine visibility based on mobile/desktop and selection/hover state
    let visibilityClass = '';

    if (isMobile && selectedSourceId) {
      // Mobile: Hide unconnected accounts when a source is selected
      visibilityClass = isHighlighted ? '' : 'hidden';
    } else if (!isMobile && (hoveredSourceId || hoveredReceiverId)) {
      // Desktop: Dim unconnected accounts when hovering
      visibilityClass = isHighlighted ? 'opacity-100' : 'opacity-30';
    }

    return (
      <div
        className={`bg-white dark:bg-gray-800 rounded-lg overflow-hidden shadow-lg ${isMobile ? 'flex flex-col' : 'flex'
          } transition-all w-full text-sm md:text-base ${visibilityClass}`}
        onMouseEnter={onMouseEnter}
        onMouseLeave={onMouseLeave}
      >
        {/* Status Bar - Top for receiver on mobile, left for receiver on desktop */}
        {type === 'receiver' && <StatusIndicatorBar account={account} type={type} isMobile={isMobile} />}

        {/* Card Content */}
        <div className="flex-1">
          {/* Card Header */}
          <AccountCardHeader
            account={account}
            onToggle={onToggle}
            onToggleEnabled={onToggleEnabled}
          />

          {/* Card Body - Expands on click */}
          {account.isExpanded && (
            <AccountCardExpanded
              connection={connection}
              accountSettings={accountSettings}
              type={type}
              content={content}
            />
          )}

          {/* Error/Warning Alert */}
          <ErrorWarningAlert account={account} />
        </div>

        {/* Status Bar - Bottom for source on mobile, right for source on desktop */}
        {type === 'source' && <StatusIndicatorBar account={account} type={type} isMobile={isMobile} />}
      </div>
    );
  }
);

AccountCard.displayName = 'AccountCard';
