'use client';

import { useState } from 'react';
import { ChevronDown, Folder, Settings, Trash2 } from 'lucide-react';
import { Switch } from '@/components/ui/switch';
import type { AccountInfo, CopySettings } from '@/types';

interface AccountCardHeaderProps {
  account: AccountInfo;
  onToggle: () => void;
  onToggleEnabled?: (enabled: boolean) => void;
  settingsLabel: string;
  accountSettings: CopySettings[];
  onDeleteSetting?: (setting: CopySettings) => void;
}

/**
 * Header section of the account card with icon, name, switch, settings button, and expand toggle
 */
export function AccountCardHeader({
  account,
  onToggle,
  onToggleEnabled,
  settingsLabel,
  accountSettings,
  onDeleteSetting,
}: AccountCardHeaderProps) {
  const [settingsExpanded, setSettingsExpanded] = useState(false);

  return (
    <div>
      {/* Header row */}
      <div
        className={`flex items-center gap-1 md:gap-2 px-2 md:px-3 py-2 ${
          account.hasError
            ? 'bg-pink-50 dark:bg-pink-900/20'
            : account.hasWarning
            ? 'bg-yellow-50 dark:bg-yellow-900/20'
            : ''
        }`}
      >
        <div className="w-6 h-6 md:w-7 md:h-7 bg-yellow-400 rounded flex items-center justify-center flex-shrink-0">
          <Folder className="w-3 h-3 md:w-4 md:h-4 text-white" />
        </div>
        <div className="flex-1 min-w-0">
          <h3 className="font-normal text-gray-900 dark:text-gray-100 text-xs md:text-sm truncate">
            {account.name}
          </h3>
        </div>
        <div className="noDrag">
          <Switch
            checked={account.isEnabled}
            onCheckedChange={(checked) => onToggleEnabled?.(checked)}
            className="mr-0.5 md:mr-1 scale-90 md:scale-100"
          />
        </div>
        <button
          onClick={() => setSettingsExpanded(!settingsExpanded)}
          className="noDrag p-2 md:p-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 rounded transition-colors text-gray-600 dark:text-gray-400 min-w-[44px] min-h-[44px] md:min-w-0 md:min-h-0 flex items-center justify-center"
          title={settingsLabel}
        >
          <Settings className={`w-4 h-4 md:w-4 md:h-4 transition-transform ${settingsExpanded ? 'rotate-45' : ''}`} />
        </button>
        <button
          onClick={onToggle}
          className="noDrag p-2 md:p-1 hover:bg-gray-100 dark:hover:bg-gray-700 rounded transition-colors text-gray-600 dark:text-gray-400 min-w-[44px] min-h-[44px] md:min-w-0 md:min-h-0 flex items-center justify-center"
        >
          <ChevronDown
            className={`w-4 h-4 transition-transform ${account.isExpanded ? 'rotate-180' : ''}`}
          />
        </button>
      </div>

      {/* Settings list - shown when settings icon is clicked */}
      {settingsExpanded && accountSettings.length > 0 && (
        <div className="border-t border-gray-200 dark:border-gray-700">
          <div className="px-2 md:px-3 py-2 bg-gray-50 dark:bg-gray-800/50">
            <div className="text-xs font-medium text-gray-600 dark:text-gray-400 mb-2">
              {settingsLabel}
            </div>
            <div className="space-y-2">
              {accountSettings.map((setting) => (
                <div
                  key={setting.id}
                  className="flex items-center justify-between gap-2 p-2 bg-white dark:bg-gray-800 rounded border border-gray-200 dark:border-gray-700"
                >
                  <div className="flex-1 min-w-0">
                    <div className="text-xs text-gray-600 dark:text-gray-400 truncate">
                      {setting.master_account} → {setting.slave_account}
                    </div>
                  </div>
                  <button
                    onClick={() => onDeleteSetting?.(setting)}
                    className="noDrag p-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 rounded transition-colors text-red-600 dark:text-red-400"
                    title="Delete"
                  >
                    <Trash2 className="w-3.5 h-3.5" />
                  </button>
                </div>
              ))}
            </div>
          </div>
        </div>
      )}

      {/* Message when settings button clicked but no settings */}
      {settingsExpanded && accountSettings.length === 0 && (
        <div className="border-t border-gray-200 dark:border-gray-700 px-2 md:px-3 py-2 bg-gray-50 dark:bg-gray-800/50">
          <div className="text-xs text-gray-500 dark:text-gray-500">
            No settings configured
          </div>
        </div>
      )}
    </div>
  );
}
