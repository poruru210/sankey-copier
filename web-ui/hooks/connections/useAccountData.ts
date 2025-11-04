import { useState, useEffect, useMemo } from 'react';
import type { EaConnection, CopySettings, AccountInfo } from '@/types';

interface UseAccountDataProps {
  connections: EaConnection[];
  settings: CopySettings[];
  content: {
    allSourcesInactive: string;
    someSourcesInactive: string;
  };
}

interface UseAccountDataReturn {
  sourceAccounts: AccountInfo[];
  receiverAccounts: AccountInfo[];
  setSourceAccounts: React.Dispatch<React.SetStateAction<AccountInfo[]>>;
  setReceiverAccounts: React.Dispatch<React.SetStateAction<AccountInfo[]>>;
  getConnectionStatus: (accountId: string) => boolean;
  getAccountConnection: (accountId: string) => EaConnection | undefined;
  getAccountSettings: (accountId: string, type: 'source' | 'receiver') => CopySettings[];
  toggleSourceExpand: (accountId: string) => void;
  toggleReceiverExpand: (accountId: string) => void;
}

/**
 * Custom hook for managing account data, including status checks and error/warning states
 *
 * @param connections - List of EA connections
 * @param settings - List of copy settings
 * @param content - Internationalized content for error messages
 * @returns Account data and helper functions
 */
export function useAccountData({
  connections,
  settings,
  content,
}: UseAccountDataProps): UseAccountDataReturn {
  const [sourceAccounts, setSourceAccounts] = useState<AccountInfo[]>([]);
  const [receiverAccounts, setReceiverAccounts] = useState<AccountInfo[]>([]);

  // Helper function to check connection status
  const getConnectionStatus = (accountId: string): boolean => {
    const conn = connections.find((c) => c.account_id === accountId);
    // Support both old and new formats
    return conn?.is_online ?? (conn?.status === 'Online') ?? false;
  };

  // Helper function to get account connection data
  const getAccountConnection = (accountId: string): EaConnection | undefined => {
    return connections.find((c) => c.account_id === accountId);
  };

  // Helper function to get settings for an account
  const getAccountSettings = (accountId: string, type: 'source' | 'receiver'): CopySettings[] => {
    if (type === 'source') {
      return settings.filter((s) => s.master_account === accountId);
    } else {
      return settings.filter((s) => s.slave_account === accountId);
    }
  };

  // Build account lists from settings
  useEffect(() => {
    const sourceMap = new Map<string, AccountInfo>();
    const receiverMap = new Map<string, AccountInfo>();

    // Get previous account states for preserving isEnabled
    const prevSourceMap = new Map(sourceAccounts.map(acc => [acc.id, acc]));
    const prevReceiverMap = new Map(receiverAccounts.map(acc => [acc.id, acc]));

    settings.forEach((setting, index) => {
      // Add source
      if (!sourceMap.has(setting.master_account)) {
        const existingSource = prevSourceMap.get(setting.master_account);
        const isOnline = getConnectionStatus(setting.master_account);
        // For demo: show error on first account if offline
        const hasError = index === 0 && !isOnline;
        sourceMap.set(setting.master_account, {
          id: setting.master_account,
          name: setting.master_account,
          isOnline,
          isEnabled: existingSource?.isEnabled ?? true,
          hasError,
          errorMsg: hasError ? 'エラー! チャート上のEAの問題' : '',
          hasWarning: false,
          isExpanded: existingSource?.isExpanded ?? false,
        });
      }

      // Add receiver
      if (!receiverMap.has(setting.slave_account)) {
        const existingReceiver = prevReceiverMap.get(setting.slave_account);
        const isOnline = getConnectionStatus(setting.slave_account);

        receiverMap.set(setting.slave_account, {
          id: setting.slave_account,
          name: setting.slave_account,
          isOnline,
          isEnabled: existingReceiver?.isEnabled ?? setting.enabled,
          hasError: false,
          hasWarning: false,
          errorMsg: '',
          isExpanded: existingReceiver?.isExpanded ?? false,
        });
      }
    });

    // Convert to arrays
    const newSourceAccounts = Array.from(sourceMap.values());
    const newReceiverAccounts = Array.from(receiverMap.values());

    // Update receiver errors/warnings based on connected source status
    newReceiverAccounts.forEach((receiver) => {
      // Get all connected sources for this receiver
      const connectedSources = settings
        .filter((s) => s.slave_account === receiver.id)
        .map((s) => s.master_account);

      // Count active and inactive sources
      let activeCount = 0;
      let inactiveCount = 0;

      connectedSources.forEach((sourceId) => {
        const source = newSourceAccounts.find((acc) => acc.id === sourceId);
        if (source) {
          if (source.isEnabled && !source.hasError) {
            activeCount++;
          } else {
            inactiveCount++;
          }
        }
      });

      // Determine receiver state based on source states
      if (inactiveCount > 0 && activeCount === 0) {
        // All sources are inactive - ERROR
        receiver.hasError = true;
        receiver.hasWarning = false;
        receiver.errorMsg = content.allSourcesInactive;
      } else if (inactiveCount > 0 && activeCount > 0) {
        // Some sources are inactive - WARNING
        receiver.hasError = false;
        receiver.hasWarning = true;
        receiver.errorMsg = content.someSourcesInactive;
      } else {
        // All sources are active - NORMAL
        receiver.hasError = false;
        receiver.hasWarning = false;
        receiver.errorMsg = '';
      }
    });

    setSourceAccounts(newSourceAccounts);
    setReceiverAccounts(newReceiverAccounts);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [settings, connections, content.allSourcesInactive, content.someSourcesInactive]);

  // Toggle expand state for source accounts
  const toggleSourceExpand = (accountId: string) => {
    setSourceAccounts((prev) =>
      prev.map((acc) => (acc.id === accountId ? { ...acc, isExpanded: !acc.isExpanded } : acc))
    );
  };

  // Toggle expand state for receiver accounts
  const toggleReceiverExpand = (accountId: string) => {
    setReceiverAccounts((prev) =>
      prev.map((acc) => (acc.id === accountId ? { ...acc, isExpanded: !acc.isExpanded } : acc))
    );
  };

  return {
    sourceAccounts,
    receiverAccounts,
    setSourceAccounts,
    setReceiverAccounts,
    getConnectionStatus,
    getAccountConnection,
    getAccountSettings,
    toggleSourceExpand,
    toggleReceiverExpand,
  };
}
