import { useCallback } from 'react';
import type { CopySettings, AccountInfo } from '@/types';

interface UseAccountToggleProps {
  settings: CopySettings[];
  sourceAccounts: AccountInfo[];
  receiverAccounts: AccountInfo[];
  setSourceAccounts: React.Dispatch<React.SetStateAction<AccountInfo[]>>;
  setReceiverAccounts: React.Dispatch<React.SetStateAction<AccountInfo[]>>;
  onToggle: (id: number, currentStatus: number) => void;
}

interface UseAccountToggleReturn {
  toggleSourceEnabled: (accountId: string, enabled: boolean) => void;
  toggleReceiverEnabled: (accountId: string, enabled: boolean) => void;
}

/**
 * Custom hook for managing account enable/disable toggle operations
 *
 * @param props - Settings, account states, and toggle callback
 * @returns Toggle functions for sources and receivers
 */
export function useAccountToggle({
  settings,
  sourceAccounts,
  receiverAccounts,
  setSourceAccounts,
  setReceiverAccounts,
  onToggle,
}: UseAccountToggleProps): UseAccountToggleReturn {
  const toggleSourceEnabled = useCallback(
    (accountId: string, enabled: boolean) => {
      // Update local state
      setSourceAccounts((prev) =>
        prev.map((acc) => (acc.id === accountId ? { ...acc, isEnabled: enabled } : acc))
      );

      // Find all settings for this source and toggle them
      const sourceSettings = settings.filter((s) => s.master_account === accountId);
      sourceSettings.forEach((setting) => {
        onToggle(setting.id, setting.status);
      });
    },
    [settings, setSourceAccounts, onToggle]
  );

  const toggleReceiverEnabled = useCallback(
    (accountId: string, enabled: boolean) => {
      // Update local state
      setReceiverAccounts((prev) =>
        prev.map((acc) => (acc.id === accountId ? { ...acc, isEnabled: enabled } : acc))
      );

      // Find all settings for this receiver and toggle them
      const receiverSettings = settings.filter((s) => s.slave_account === accountId);
      receiverSettings.forEach((setting) => {
        onToggle(setting.id, setting.status);
      });
    },
    [settings, setReceiverAccounts, onToggle]
  );

  return {
    toggleSourceEnabled,
    toggleReceiverEnabled,
  };
}
