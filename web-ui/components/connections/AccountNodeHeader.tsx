'use client';

// AccountNodeHeader - Header section of the account node
// Shows broker icon, account name, enable/disable switch, settings button, and expand toggle
// For Master (source) nodes: shows Master settings button
// For Slave (receiver) nodes: shows connection settings button

import { ChevronDown, Settings } from 'lucide-react';
import { Switch } from '@/components/ui/switch';
import { Badge } from '@/components/ui/badge';
import { BrokerIcon } from '@/components/BrokerIcon';
import type { AccountInfo } from '@/types';

interface AccountNodeHeaderProps {
  account: AccountInfo;
  onToggle: () => void;
  onToggleEnabled?: (enabled: boolean) => void;
  onEditMasterSettings?: () => void;
  onOpenSettingsDrawer?: () => void;
}

export function AccountNodeHeader({
  account,
  onToggle,
  onToggleEnabled,
  onEditMasterSettings,
  onOpenSettingsDrawer,
}: AccountNodeHeaderProps) {

  // Split account name into broker name and account number
  // Format: "Broker_Name_AccountNumber"
  const splitAccountName = () => {
    const lastUnderscoreIndex = account.name.lastIndexOf('_');
    if (lastUnderscoreIndex === -1) {
      return { brokerName: account.name, accountNumber: '' };
    }
    return {
      brokerName: account.name.substring(0, lastUnderscoreIndex).replace(/_/g, ' '),
      accountNumber: account.name.substring(lastUnderscoreIndex + 1),
    };
  };

  const { brokerName, accountNumber } = splitAccountName();

  // Determine which settings button to show
  const hasSettingsButton = onEditMasterSettings || onOpenSettingsDrawer;
  const handleSettingsClick = onEditMasterSettings || onOpenSettingsDrawer;

  return (
    <div>
      {/* Header row - Draggable area */}
      <div
        className={`flex items-center gap-2 md:gap-3 px-3 md:px-4 py-3 cursor-move drag-handle ${
          account.hasError
            ? 'bg-pink-50 dark:bg-pink-900/20'
            : account.hasWarning
            ? 'bg-yellow-50 dark:bg-yellow-900/20'
            : ''
        }`}
      >
        <BrokerIcon brokerName={brokerName} size="md" />
        <div className="flex-1 min-w-0">
          <div className="font-normal text-gray-900 dark:text-gray-100 text-xs md:text-sm truncate">
            {brokerName}
          </div>
          {accountNumber && (
            <div className="flex items-center gap-1.5 mt-1 text-[10px] md:text-xs text-gray-600 dark:text-gray-400">
              {account.platform && (
                <Badge
                  className={`text-[8px] md:text-[9px] px-1.5 py-0 h-4 md:h-[18px] font-medium ${
                    account.platform === 'MT4'
                      ? 'bg-blue-500 text-white hover:bg-blue-500'
                      : 'bg-purple-500 text-white hover:bg-purple-500'
                  }`}
                >
                  {account.platform}
                </Badge>
              )}
              <span className="truncate">{accountNumber}</span>
            </div>
          )}
        </div>
        {/* Switch - smaller size, vertically centered */}
        <div className="noDrag flex items-center">
          <Switch
            checked={account.isEnabled}
            onCheckedChange={(checked) => onToggleEnabled?.(checked)}
            className="scale-75 md:scale-80"
          />
        </div>
        {/* Settings button - shown for both Master and Slave accounts */}
        {hasSettingsButton && (
          <button
            onClick={(e) => {
              e.stopPropagation();
              handleSettingsClick?.();
            }}
            className="noDrag p-2 md:p-1 hover:bg-blue-100 dark:hover:bg-blue-900/30 rounded transition-colors text-blue-600 dark:text-blue-400 min-w-[44px] min-h-[44px] md:min-w-0 md:min-h-0 flex items-center justify-center"
            title={onEditMasterSettings ? "Master Settings" : "Connection Settings"}
          >
            <Settings className="w-4 h-4" />
          </button>
        )}
        <button
          onClick={onToggle}
          className="noDrag p-2 md:p-1 hover:bg-gray-100 dark:hover:bg-gray-700 rounded transition-colors text-gray-600 dark:text-gray-400 min-w-[44px] min-h-[44px] md:min-w-0 md:min-h-0 flex items-center justify-center"
        >
          <ChevronDown
            className={`w-4 h-4 transition-transform ${account.isExpanded ? 'rotate-180' : ''}`}
          />
        </button>
      </div>
    </div>
  );
}
