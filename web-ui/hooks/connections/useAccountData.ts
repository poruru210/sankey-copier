import { useMemo, useCallback, useEffect, useState } from 'react';
import { useAtom } from 'jotai';
import type { EaConnection, CopySettings, AccountInfo, TradeGroup } from '@/types';
import {
  expandedSourceIdsAtom,
  expandedReceiverIdsAtom,
  disabledReceiverIdsAtom,
} from '@/lib/atoms/ui';

interface UseAccountDataProps {
  connections: EaConnection[];
  settings: CopySettings[];
  tradeGroups: TradeGroup[];
  content: {
    allSourcesInactive: string;
    someSourcesInactive: string;
    autoTradingDisabled: string;
  };
}

interface UseAccountDataReturn {
  sourceAccounts: AccountInfo[];
  receiverAccounts: AccountInfo[];
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
 * @param tradeGroups - List of trade groups (for Master enabled state)
 * @param content - Internationalized content for error messages
 * @returns Account data and helper functions
 */
export function useAccountData({
  connections,
  settings,
  tradeGroups,
  content,
}: UseAccountDataProps): UseAccountDataReturn {
  const [expandedSourceIds, setExpandedSourceIds] = useAtom(expandedSourceIdsAtom);
  const [expandedReceiverIds, setExpandedReceiverIds] = useAtom(expandedReceiverIdsAtom);
  const [disabledReceiverIds, setDisabledReceiverIds] = useAtom(disabledReceiverIdsAtom);
  const [initialized, setInitialized] = useState(false);

  // Helper to get Master enabled state from TradeGroups
  const isMasterEnabled = useCallback((masterAccount: string): boolean => {
    const tradeGroup = tradeGroups.find((tg) => tg.id === masterAccount);
    return tradeGroup?.master_settings.enabled ?? true;
  }, [tradeGroups]);

  // Initialize disabled receiver list from settings on first load
  useEffect(() => {
    if (!initialized && settings.length > 0) {
      const newDisabledReceivers: string[] = [];

      settings.forEach((setting) => {
        if (setting.status === 0) {
          if (!disabledReceiverIds.includes(setting.slave_account) && !newDisabledReceivers.includes(setting.slave_account)) {
            newDisabledReceivers.push(setting.slave_account);
          }
        }
      });

      if (newDisabledReceivers.length > 0) {
        setDisabledReceiverIds((prev) => [...prev, ...newDisabledReceivers]);
      }

      setInitialized(true);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [settings, initialized]);

  // Helper function to check connection status
  const getConnectionStatus = useCallback((accountId: string): boolean => {
    const conn = connections.find((c) => c.account_id === accountId);
    // Support both old and new formats
    return conn?.is_online ?? (conn?.status === 'Online' || false);
  }, [connections]);

  // Helper function to get account connection data
  const getAccountConnection = useCallback((accountId: string): EaConnection | undefined => {
    return connections.find((c) => c.account_id === accountId);
  }, [connections]);

  // Helper function to get settings for an account
  const getAccountSettings = useCallback((accountId: string, type: 'source' | 'receiver'): CopySettings[] => {
    if (type === 'source') {
      return settings.filter((s) => s.master_account === accountId);
    } else {
      return settings.filter((s) => s.slave_account === accountId);
    }
  }, [settings]);

  // Build account lists from settings
  const { sourceAccounts, receiverAccounts } = useMemo(() => {
    const sourceMap = new Map<string, AccountInfo>();
    const receiverMap = new Map<string, AccountInfo>();

    settings.forEach((setting, index) => {
      // Add source (Master)
      if (!sourceMap.has(setting.master_account)) {
        const isOnline = getConnectionStatus(setting.master_account);
        const connection = getAccountConnection(setting.master_account);
        const isTradeAllowed = connection?.is_trade_allowed ?? true;

        // Master enabled state comes from TradeGroup.master_settings.enabled
        const isEnabled = isMasterEnabled(setting.master_account);
        const isExpanded = expandedSourceIds.includes(setting.master_account);

        // Calculate active state: Master is active if online && trade_allowed && enabled
        const isActive = isOnline && isTradeAllowed && isEnabled;

        // Warnings: only show if online but trade not allowed
        const hasWarning = isOnline && !isTradeAllowed;
        const hasError = false;
        const errorMsg = hasWarning ? content.autoTradingDisabled : '';

        sourceMap.set(setting.master_account, {
          id: setting.master_account,
          name: setting.master_account,
          platform: connection?.platform,
          isOnline,
          isEnabled,
          isActive,
          hasError,
          hasWarning,
          errorMsg,
          isExpanded,
        });
      }

      // Add receiver (Slave)
      if (!receiverMap.has(setting.slave_account)) {
        const isOnline = getConnectionStatus(setting.slave_account);
        const connection = getAccountConnection(setting.slave_account);
        const isTradeAllowed = connection?.is_trade_allowed ?? true;

        // Receiver is enabled if not in disabled list (independent of settings status)
        const isEnabled = !disabledReceiverIds.includes(setting.slave_account);
        const isExpanded = expandedReceiverIds.includes(setting.slave_account);

        // Active state calculation will be done after all masters are processed
        // For now, set to false - will be updated in the next section
        const isActive = false;

        // Check for MT auto-trading disabled warning
        const hasWarning = isOnline && !isTradeAllowed;

        receiverMap.set(setting.slave_account, {
          id: setting.slave_account,
          name: setting.slave_account,
          platform: connection?.platform,
          isOnline,
          isEnabled,
          isActive, // Will be updated below based on connected masters
          hasError: false,
          hasWarning,
          errorMsg: hasWarning ? content.autoTradingDisabled : '',
          isExpanded,
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

    return { sourceAccounts: newSourceAccounts, receiverAccounts: newReceiverAccounts };
  }, [
    settings,
    content.autoTradingDisabled,
    expandedSourceIds,
    expandedReceiverIds,
    isMasterEnabled,
    disabledReceiverIds,
    getAccountConnection,
    getConnectionStatus,
  ]);

  // Toggle expand state for source accounts
  const toggleSourceExpand = (accountId: string) => {
    setExpandedSourceIds((prev) =>
      prev.includes(accountId)
        ? prev.filter((id) => id !== accountId)
        : [...prev, accountId]
    );
  };

  // Toggle expand state for receiver accounts
  const toggleReceiverExpand = (accountId: string) => {
    setExpandedReceiverIds((prev) =>
      prev.includes(accountId)
        ? prev.filter((id) => id !== accountId)
        : [...prev, accountId]
    );
  };

  return {
    sourceAccounts,
    receiverAccounts,
    getConnectionStatus,
    getAccountConnection,
    getAccountSettings,
    toggleSourceExpand,
    toggleReceiverExpand,
  };
}
