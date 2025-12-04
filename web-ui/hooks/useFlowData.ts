import { useMemo, useCallback, useState } from 'react';
import { useAtom, useAtomValue } from 'jotai';
import { Node, Edge } from '@xyflow/react';
import type { AccountInfo, CopySettings, EaConnection } from '@/types';
import type { AccountNodeData } from '@/components/flow-nodes';
import {
  hoveredSourceIdAtom,
  hoveredReceiverIdAtom,
  selectedSourceIdAtom,
  expandedSourceIdsAtom,
  expandedReceiverIdsAtom,
  disabledReceiverIdsAtom,
} from '@/lib/atoms/ui';

interface UseFlowDataProps {
  sourceAccounts: AccountInfo[];
  receiverAccounts: AccountInfo[];
  settings: CopySettings[];
  getAccountConnection: (accountId: string) => EaConnection | undefined;
  getAccountSettings: (accountId: string, type: 'source' | 'receiver') => CopySettings[];
  handleEditSetting: (setting: CopySettings) => void;
  handleDeleteSetting: (setting: CopySettings) => void;
  handleEditMasterSettings?: (masterAccount: string) => void;
  isAccountHighlighted: (accountId: string, type: 'source' | 'receiver') => boolean;
  isMobile: boolean;
  content: any;
  onToggle: (id: number, enabled: boolean) => Promise<void>;
  onToggleMaster: (masterAccount: string, enabled: boolean) => Promise<void>;
}

const MIN_PENDING_DURATION_MS = 800;

// Layout constants - Desktop (horizontal)
const NODE_WIDTH = 380;
const NODE_HEIGHT = 120;
const VERTICAL_SPACING = 200;
const SOURCE_X = 0;
const RECEIVER_X = 600; // Moved closer since no relay server

// Layout constants - Mobile (vertical)
const MOBILE_X = 0;
const MOBILE_SOURCE_START_Y = 0;
const MOBILE_VERTICAL_SPACING = 200;
const MOBILE_SECTION_GAP = 120; // Gap between source and receiver sections

/**
 * Custom hook to convert account data to React Flow nodes and edges
 */
export function useFlowData({
  sourceAccounts,
  receiverAccounts,
  settings,
  getAccountConnection,
  getAccountSettings,
  handleEditSetting,
  handleDeleteSetting,
  handleEditMasterSettings,
  isAccountHighlighted,
  isMobile,
  content,
  onToggle,
  onToggleMaster,
}: UseFlowDataProps): { nodes: Node[]; edges: Edge[] } {
  const hoveredSourceId = useAtomValue(hoveredSourceIdAtom);
  const hoveredReceiverId = useAtomValue(hoveredReceiverIdAtom);
  const selectedSourceId = useAtomValue(selectedSourceIdAtom);

  const [expandedSourceIds, setExpandedSourceIds] = useAtom(expandedSourceIdsAtom);
  const [expandedReceiverIds, setExpandedReceiverIds] = useAtom(expandedReceiverIdsAtom);
  const [disabledReceiverIds, setDisabledReceiverIds] = useAtom(disabledReceiverIdsAtom);
  const [pendingAccountIds, setPendingAccountIds] = useState<Set<string>>(new Set());

  const setAccountPending = useCallback((accountId: string, isPending: boolean) => {
    setPendingAccountIds((prev) => {
      const next = new Set(prev);
      if (isPending) {
        next.add(accountId);
      } else {
        next.delete(accountId);
      }
      return next;
    });
  }, []);

  const runWithPending = useCallback((accountId: string, task: () => Promise<void> | void) => {
    setAccountPending(accountId, true);

    const execute = async () => {
      const start = Date.now();
      try {
        await task();
      } catch (error) {
        console.error(`[Connections] Failed to toggle account ${accountId}`, error);
      } finally {
        const elapsed = Date.now() - start;
        const remaining = Math.max(0, MIN_PENDING_DURATION_MS - elapsed);
        setTimeout(() => setAccountPending(accountId, false), remaining);
      }
    };

    void execute();
  }, [setAccountPending]);

  const toggleSourceExpand = useCallback((accountId: string) => {
    setExpandedSourceIds((prev) =>
      prev.includes(accountId)
        ? prev.filter((id) => id !== accountId)
        : [...prev, accountId]
    );
  }, [setExpandedSourceIds]);

  const toggleReceiverExpand = useCallback((accountId: string) => {
    setExpandedReceiverIds((prev) =>
      prev.includes(accountId)
        ? prev.filter((id) => id !== accountId)
        : [...prev, accountId]
    );
  }, [setExpandedReceiverIds]);

  const toggleSourceEnabled = useCallback((accountId: string, enabled: boolean) => {
    runWithPending(accountId, () => onToggleMaster(accountId, enabled));
  }, [onToggleMaster, runWithPending]);

  const toggleReceiverEnabled = useCallback((accountId: string, enabled: boolean) => {
    // Update local state (disabledReceiverIds)
    setDisabledReceiverIds((prev) => {
      if (enabled) {
        return prev.filter((id) => id !== accountId);
      }
      return prev.includes(accountId) ? prev : [...prev, accountId];
    });

    // Receiver enabled state is derived from settings, so we just need to update settings
    const receiverSettings = settings.filter((s) => s.slave_account === accountId);
    const mutations = receiverSettings
      .filter((setting) => {
        const intentEnabled = setting.enabled_flag ?? (setting.status !== 0);
        return intentEnabled !== enabled;
      })
      .map((setting) => onToggle(setting.id, enabled));

    if (mutations.length === 0) {
      return;
    }

    runWithPending(accountId, async () => {
      await Promise.allSettled(mutations);
    });
  }, [settings, onToggle, setDisabledReceiverIds, runWithPending]);

  const nodes = useMemo(() => {
    const nodeList: Node[] = [];

    // Create source account nodes
    sourceAccounts.forEach((account, index) => {
      const accountSettings = getAccountSettings(account.id, 'source');
      const connection = getAccountConnection(account.id);
      const isHighlighted = isAccountHighlighted(account.id, 'source');

      // Mobile: vertical layout, Desktop: horizontal layout
      const position = isMobile
        ? { x: MOBILE_X, y: MOBILE_SOURCE_START_Y + index * MOBILE_VERTICAL_SPACING }
        : { x: SOURCE_X, y: index * VERTICAL_SPACING };

      nodeList.push({
        id: `source-${account.id}`,
        type: 'accountNode',
        position,
        data: {
          account,
          connection,
          accountSettings,
          onToggle: () => toggleSourceExpand(account.id),
          onToggleEnabled: (enabled: boolean) => toggleSourceEnabled(account.id, enabled),
          onEditSetting: handleEditSetting,
          onDeleteSetting: handleDeleteSetting,
          onEditMasterSettings: handleEditMasterSettings ? () => handleEditMasterSettings(account.id) : undefined,
          type: 'source' as const,
          isHighlighted,
          hoveredSourceId,
          hoveredReceiverId,
          selectedSourceId,
          isMobile,
          content,
          isTogglePending: pendingAccountIds.has(account.id),
        } as AccountNodeData & Record<string, unknown>,
      });
    });

    // Create receiver account nodes
    receiverAccounts.forEach((account, index) => {
      const accountSettings = getAccountSettings(account.id, 'receiver');
      const connection = getAccountConnection(account.id);
      const isHighlighted = isAccountHighlighted(account.id, 'receiver');

      // Mobile: vertical layout below source accounts, Desktop: horizontal layout
      const position = isMobile
        ? {
          x: MOBILE_X,
          y: MOBILE_SOURCE_START_Y +
            sourceAccounts.length * MOBILE_VERTICAL_SPACING +
            MOBILE_SECTION_GAP +
            index * MOBILE_VERTICAL_SPACING,
        }
        : { x: RECEIVER_X, y: index * VERTICAL_SPACING };

      nodeList.push({
        id: `receiver-${account.id}`,
        type: 'accountNode',
        position,
        data: {
          account,
          connection,
          accountSettings,
          onToggle: () => toggleReceiverExpand(account.id),
          onToggleEnabled: (enabled: boolean) => toggleReceiverEnabled(account.id, enabled),
          onEditSetting: handleEditSetting,
          onDeleteSetting: handleDeleteSetting,
          type: 'receiver' as const,
          isHighlighted,
          hoveredSourceId,
          hoveredReceiverId,
          selectedSourceId,
          isMobile,
          content,
          isTogglePending: pendingAccountIds.has(account.id),
        } as AccountNodeData & Record<string, unknown>,
      });
    });

    return nodeList;
  }, [
    sourceAccounts,
    receiverAccounts,
    getAccountConnection,
    getAccountSettings,
    handleEditSetting,
    handleDeleteSetting,
    handleEditMasterSettings,
    hoveredSourceId,
    hoveredReceiverId,
    selectedSourceId,
    isAccountHighlighted,
    isMobile,
    content,
    pendingAccountIds,
    toggleSourceExpand,
    toggleReceiverExpand,
    toggleSourceEnabled,
    toggleReceiverEnabled,
  ]);

  const edges = useMemo(() => {
    const edgeList: Edge[] = [];

    // Create direct edges from source accounts to receiver accounts
    settings.forEach((setting) => {
      const sourceAccount = sourceAccounts.find(
        (acc) => acc.id === setting.master_account
      );
      const receiverAccount = receiverAccounts.find(
        (acc) => acc.id === setting.slave_account
      );

      if (!sourceAccount || !receiverAccount) return;

      // Edge animation and color is based solely on this specific connection's runtime_status
      // A connection is "active" when runtime_status === 2 (CONNECTED)
      // This means the Master is online and actively sending signals to this Slave
      const runtimeStatus = setting.runtime_status ?? setting.status ?? 0;
      const isConnected = runtimeStatus === 2;

      // Direct edge from source to receiver with settings button
      edgeList.push({
        id: `edge-${setting.id}`,
        source: `source-${setting.master_account}`,
        target: `receiver-${setting.slave_account}`,
        type: 'settingsEdge',
        animated: isConnected,
        style: {
          stroke: isConnected ? '#22c55e' : '#d1d5db',
          strokeWidth: 2,
          strokeDasharray: isConnected ? undefined : '5,5',
        },
        data: {
          setting,
          onEditSetting: handleEditSetting,
        },
      });
    });

    return edgeList;
  }, [settings, sourceAccounts, receiverAccounts, handleEditSetting]);

  return { nodes, edges };
}
