import { useCallback } from 'react';
import { useAtom } from 'jotai';
import type { CopySettings } from '@/types';
import { disabledSourceIdsAtom } from '@/lib/atoms/ui';

interface UseAccountToggleProps {
  settings: CopySettings[];
  onToggle: (id: number, enabled: boolean) => Promise<void>;
}

interface UseAccountToggleReturn {
  toggleSourceEnabled: (accountId: string, enabled: boolean) => void;
  toggleReceiverEnabled: (accountId: string, enabled: boolean) => void;
}

/**
 * Custom hook for managing account enable/disable toggle operations
 *
 * @param props - Settings and toggle callback
 * @returns Toggle functions for sources and receivers
 */
export function useAccountToggle({
  settings,
  onToggle,
}: UseAccountToggleProps): UseAccountToggleReturn {
  const [disabledSourceIds, setDisabledSourceIds] = useAtom(disabledSourceIdsAtom);

  const toggleSourceEnabled = useCallback(
    (accountId: string, enabled: boolean) => {
      // Update local state (disabledSourceIds)
      setDisabledSourceIds((prev) => {
        if (enabled) {
          return prev.filter((id) => id !== accountId);
        } else {
          return prev.includes(accountId) ? prev : [...prev, accountId];
        }
      });

      // Find all settings for this source and toggle them
      const sourceSettings = settings.filter((s) => s.master_account === accountId);
      sourceSettings.forEach((setting) => {
        // Only toggle if the target state matches the desired enabled state
        // If we want to ENABLE (enabled=true), we should toggle if currently DISABLED (status=0)
        // If we want to DISABLE (enabled=false), we should toggle if currently ENABLED (status!=0)
        const intentEnabled = setting.enabled_flag ?? (setting.status !== 0);
        if (intentEnabled !== enabled) {
          onToggle(setting.id, enabled);
        }
      });
    },
    [settings, setDisabledSourceIds, onToggle]
  );

  const toggleReceiverEnabled = useCallback(
    (accountId: string, enabled: boolean) => {
      // Receiver enabled state is derived from settings, so we just need to update settings

      // Find all settings for this receiver and toggle them
      const receiverSettings = settings.filter((s) => s.slave_account === accountId);
      receiverSettings.forEach((setting) => {
        const intentEnabled = setting.enabled_flag ?? (setting.status !== 0);
        if (intentEnabled !== enabled) {
          onToggle(setting.id, enabled);
        }
      });
    },
    [settings, onToggle]
  );

  return {
    toggleSourceEnabled,
    toggleReceiverEnabled,
  };
}
