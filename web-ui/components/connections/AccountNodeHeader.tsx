'use client';

// AccountNodeHeader - Header section of the account node
// Shows broker icon, account name, enable/disable switch, settings button, and expand toggle
// For Master (source) nodes: shows Master settings button
// For Slave (receiver) nodes: shows connection settings button

import { ChevronDown, Settings } from 'lucide-react';
import { useIntlayer } from 'next-intlayer';
import { Switch } from '@/components/ui/switch';
import { Badge } from '@/components/ui/badge';
import { BrokerIcon } from '@/components/BrokerIcon';
import type { AccountInfo } from '@/types';
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
  TooltipProvider,
} from '@/components/ui/tooltip';

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
  const content = useIntlayer('account-node-header');

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

  const renderStatusBadges = () => {
    if (account.masterRuntimeStatus !== undefined) {
      const runtime = account.masterRuntimeStatus;
      const runtimeLabels: Record<number, string> = {
        0: content.runtimeManualOff,
        1: content.runtimeStandby,
        2: content.runtimeStreaming,
      };
      const runtimeLabel =
        runtimeLabels[runtime] ??
        content.runtimeUnknownState.replace('{code}', runtime.toString());
      const runtimeColors: Record<number, string> = {
        0: 'bg-gray-200 text-gray-700 dark:bg-gray-700 dark:text-gray-200',
        1: 'bg-amber-100 text-amber-700 dark:bg-amber-900/40 dark:text-amber-200',
        2: 'bg-emerald-500 text-white',
      };
      return (
        <div className="flex flex-wrap gap-1">
          <TooltipProvider delayDuration={100}>
            <Tooltip>
              <TooltipTrigger asChild>
                <Badge className={`text-[10px] px-1.5 py-0 ${runtimeColors[runtime] ?? 'bg-gray-200'}`}>
                  {runtimeLabel}
                </Badge>
              </TooltipTrigger>
              <TooltipContent>
                <p className="text-xs">
                  {content.runtimeTooltip.replace('{state}', runtimeLabel)}
                </p>
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
          {typeof account.masterIntentEnabled === 'boolean' && (
            <TooltipProvider delayDuration={100}>
              <Tooltip>
                <TooltipTrigger asChild>
                  <Badge
                    variant={account.masterIntentEnabled ? 'default' : 'secondary'}
                    className="text-[10px] px-1.5 py-0"
                  >
                    {account.masterIntentEnabled ? content.masterIntentOn : content.masterIntentOff}
                  </Badge>
                </TooltipTrigger>
                <TooltipContent>
                  <p className="text-xs">{content.masterIntentTooltip}</p>
                </TooltipContent>
              </Tooltip>
            </TooltipProvider>
          )}
        </div>
      );
    }

    if (typeof account.slaveIntentEnabled === 'boolean') {
      return (
        <TooltipProvider delayDuration={100}>
          <Tooltip>
            <TooltipTrigger asChild>
              <Badge
                variant={account.slaveIntentEnabled ? 'default' : 'secondary'}
                className="text-[10px] px-1.5 py-0"
              >
                {account.slaveIntentEnabled ? content.slaveIntentOn : content.slaveIntentOff}
              </Badge>
            </TooltipTrigger>
            <TooltipContent>
              <p className="text-xs">{content.slaveIntentTooltip}</p>
            </TooltipContent>
          </Tooltip>
        </TooltipProvider>
      );
    }

    return null;
  };

  const statusBadges = renderStatusBadges();

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
        <BrokerIcon brokerName={brokerName} size="lg" />
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
          {statusBadges && (
            <div className="mt-1 flex flex-wrap gap-1">
              {statusBadges}
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
            title={onEditMasterSettings ? content.masterSettingsTitle : content.connectionSettingsTitle}
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
