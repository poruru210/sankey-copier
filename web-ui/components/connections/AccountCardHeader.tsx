import { ChevronDown, Folder, Settings } from 'lucide-react';
import { Switch } from '@/components/ui/switch';
import type { AccountInfo } from '@/types';

interface AccountCardHeaderProps {
  account: AccountInfo;
  onToggle: () => void;
  onToggleEnabled?: (enabled: boolean) => void;
  settingsLabel: string;
}

/**
 * Header section of the account card with icon, name, switch, settings button, and expand toggle
 */
export function AccountCardHeader({
  account,
  onToggle,
  onToggleEnabled,
  settingsLabel,
}: AccountCardHeaderProps) {
  return (
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
      <Switch
        checked={account.isEnabled}
        onCheckedChange={(checked) => onToggleEnabled?.(checked)}
        className="mr-0.5 md:mr-1 scale-90 md:scale-100"
      />
      <button
        className="p-2 md:p-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 rounded transition-colors text-gray-600 dark:text-gray-400 min-w-[44px] min-h-[44px] md:min-w-0 md:min-h-0 flex items-center justify-center"
        title={settingsLabel}
      >
        <Settings className="w-4 h-4 md:w-4 md:h-4" />
      </button>
      <button
        onClick={onToggle}
        className="p-2 md:p-1 hover:bg-gray-100 dark:hover:bg-gray-700 rounded transition-colors text-gray-600 dark:text-gray-400 min-w-[44px] min-h-[44px] md:min-w-0 md:min-h-0 flex items-center justify-center"
      >
        <ChevronDown
          className={`w-4 h-4 transition-transform ${account.isExpanded ? 'rotate-180' : ''}`}
        />
      </button>
    </div>
  );
}
