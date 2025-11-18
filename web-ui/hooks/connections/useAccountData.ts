import { useState, useEffect, useMemo } from 'react';
import type { EaConnection, CopySettings, AccountInfo } from '@/types';

interface UseAccountDataProps {
  connections: EaConnection[];
  settings: CopySettings[];
  content: {
    allSourcesInactive: string;
    someSourcesInactive: string;
    autoTradingDisabled: string;
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
    return conn?.is_online ?? (conn?.status === 'Online' || false);
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
      // Add source (Master)
      if (!sourceMap.has(setting.master_account)) {
        const existingSource = prevSourceMap.get(setting.master_account);
        const isOnline = getConnectionStatus(setting.master_account);
        const connection = getAccountConnection(setting.master_account);
        const isTradeAllowed = connection?.is_trade_allowed ?? true;
        const isEnabled = existingSource?.isEnabled ?? true;

        // Calculate active state: Master is active if online && trade_allowed && enabled
        const isActive = isOnline && isTradeAllowed && isEnabled;

        // Warnings/errors: only show if online but trade not allowed
        const hasWarning = isOnline && !isTradeAllowed;
        const hasError = index === 0 && !isOnline;
        let errorMsg = '';
        if (hasError) {
          errorMsg = 'エラー! チャート上のEAの問題';
        } else if (hasWarning) {
          errorMsg = content.autoTradingDisabled;
        }

        sourceMap.set(setting.master_account, {
          id: setting.master_account,
          name: setting.master_account,
          isOnline,
          isEnabled,
          isActive,
          hasError,
          hasWarning,
          errorMsg,
          isExpanded: existingSource?.isExpanded ?? false,
        });
      }

      // Add receiver (Slave)
      if (!receiverMap.has(setting.slave_account)) {
        const existingReceiver = prevReceiverMap.get(setting.slave_account);
        const isOnline = getConnectionStatus(setting.slave_account);
        const connection = getAccountConnection(setting.slave_account);
        const isTradeAllowed = connection?.is_trade_allowed ?? true;
        const isEnabled = existingReceiver?.isEnabled ?? (setting.status !== 0);

        // Active state calculation will be done after all masters are processed
        // For now, set to false - will be updated in the next section
        const isActive = false;

        // Check for MT auto-trading disabled warning
        const hasWarning = isOnline && !isTradeAllowed;

        receiverMap.set(setting.slave_account, {
          id: setting.slave_account,
          name: setting.slave_account,
          isOnline,
          isEnabled,
          isActive, // Will be updated below based on connected masters
          hasError: false,
          hasWarning,
          errorMsg: hasWarning ? content.autoTradingDisabled : '',
          isExpanded: existingReceiver?.isExpanded ?? false,
        });
      }
    });

    // Convert to arrays
    const newSourceAccounts = Array.from(sourceMap.values());
    const newReceiverAccounts = Array.from(receiverMap.values());

    // Update receiver active state based on connected masters
    newReceiverAccounts.forEach((receiver) => {
      // Get all connected masters for this receiver
      const connectedMasters = settings
        .filter((s) => s.slave_account === receiver.id)
        .map((s) => s.master_account);

      // Check if ALL connected masters are active
      const allMastersActive = connectedMasters.every((masterId) => {
        const master = newSourceAccounts.find((acc) => acc.id === masterId);
        return master?.isActive === true;
      });

      // Slave active = isOnline && isTradeAllowed && isEnabled && allMastersActive
      const connection = getAccountConnection(receiver.id);
      const isTradeAllowed = connection?.is_trade_allowed ?? true;
      receiver.isActive = receiver.isOnline && isTradeAllowed && receiver.isEnabled && allMastersActive;

      // Simplify warnings: only show auto-trading warning if relevant
      if (receiver.isOnline && !isTradeAllowed) {
        receiver.hasWarning = true;
        receiver.errorMsg = content.autoTradingDisabled;
      } else {
        // No warnings/errors based on master states
        receiver.hasWarning = false;
        receiver.hasError = false;
        receiver.errorMsg = '';
      }
    });

    setSourceAccounts(newSourceAccounts);
    setReceiverAccounts(newReceiverAccounts);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [settings, connections, content.allSourcesInactive, content.someSourcesInactive, content.autoTradingDisabled]);

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
