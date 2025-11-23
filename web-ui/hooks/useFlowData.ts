import { useMemo, useCallback } from 'react';
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
  disabledSourceIdsAtom,
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
  isAccountHighlighted: (accountId: string, type: 'source' | 'receiver') => boolean;
  isMobile: boolean;
  content: any;
  onToggle: (id: number, currentStatus: number) => Promise<void>;
}

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
  isAccountHighlighted,
  isMobile,
  content,
  onToggle,
}: UseFlowDataProps): { nodes: Node[]; edges: Edge[] } {
  const hoveredSourceId = useAtomValue(hoveredSourceIdAtom);
  const hoveredReceiverId = useAtomValue(hoveredReceiverIdAtom);
  const selectedSourceId = useAtomValue(selectedSourceIdAtom);

  const [expandedSourceIds, setExpandedSourceIds] = useAtom(expandedSourceIdsAtom);
  const [expandedReceiverIds, setExpandedReceiverIds] = useAtom(expandedReceiverIdsAtom);
  const [disabledSourceIds, setDisabledSourceIds] = useAtom(disabledSourceIdsAtom);
  const [disabledReceiverIds, setDisabledReceiverIds] = useAtom(disabledReceiverIdsAtom);

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
    // Update local state (disabledSourceIds)
    setDisabledSourceIds((prev) => {
      if (enabled) {
        return prev.filter((id) => id !== accountId);
      } else {
        return prev.includes(accountId) ? prev : [...prev, accountId];
      }
    });

    // Find all settings for this source and toggle them based on logic
    const sourceSettings = settings.filter((s) => s.master_account === accountId);
    sourceSettings.forEach((setting) => {
      const isCurrentlyEnabled = setting.status !== 0;

      if (enabled) {
        // Master is being enabled
        // Only enable connection if Slave is ALSO enabled
        // Note: We use the current disabledReceiverIds state
        const isSlaveEnabled = !disabledReceiverIds.includes(setting.slave_account);
        if (isSlaveEnabled && !isCurrentlyEnabled) {
          onToggle(setting.id, setting.status);
        }
      } else {
        // Master is being disabled -> Always disable connection
        if (isCurrentlyEnabled) {
          onToggle(setting.id, setting.status);
        }
      }
    });
  }, [settings, onToggle, setDisabledSourceIds, disabledReceiverIds]);

  const toggleReceiverEnabled = useCallback((accountId: string, enabled: boolean) => {
    // Update local state (disabledReceiverIds)
    setDisabledReceiverIds((prev) => {
      if (enabled) {
        return prev.filter((id) => id !== accountId);
      } else {
        return prev.includes(accountId) ? prev : [...prev, accountId];
      }
    });

    // Receiver enabled state is derived from settings, so we just need to update settings
    const receiverSettings = settings.filter((s) => s.slave_account === accountId);
    receiverSettings.forEach((setting) => {
      const isCurrentlyEnabled = setting.status !== 0;

      if (enabled) {
        // Slave is being enabled
        // Only enable connection if Master is ALSO enabled
        const isMasterEnabled = !disabledSourceIds.includes(setting.master_account);
        if (isMasterEnabled && !isCurrentlyEnabled) {
          onToggle(setting.id, setting.status);
        }
      } else {
        // Slave is being disabled -> Always disable connection
        if (isCurrentlyEnabled) {
          onToggle(setting.id, setting.status);
        }
      }
    });
  }, [settings, onToggle, setDisabledReceiverIds, disabledSourceIds]);

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
          type: 'source' as const,
          isHighlighted,
          hoveredSourceId,
          hoveredReceiverId,
          selectedSourceId,
          isMobile,
          content,
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
    hoveredSourceId,
    hoveredReceiverId,
    selectedSourceId,
    isAccountHighlighted,
    isMobile,
    content,
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

      // Edge is active only if both source and receiver are active (green)
      // Both accounts must be online, trade allowed, enabled, and have no errors/warnings
      const isActive =
        setting.status !== 0 &&
        sourceAccount.isActive &&
        receiverAccount.isActive;

      // Build label text with copy settings
      const labelParts: string[] = [];

      // Lot multiplier
      if (setting.lot_multiplier !== null) {
        labelParts.push(`×${setting.lot_multiplier}`);
      }

      // Reverse trade indicator
      if (setting.reverse_trade) {
        labelParts.push('⇄');
      }

      const labelText = labelParts.length > 0 ? labelParts.join(' ') : '';

      // Direct edge from source to receiver
      edgeList.push({
        id: `edge-${setting.id}`,
        source: `source-${setting.master_account}`,
        target: `receiver-${setting.slave_account}`,
        animated: isActive,
        style: {
          stroke: isActive ? '#22c55e' : '#d1d5db',
          strokeWidth: 2,
          strokeDasharray: isActive ? undefined : '5,5',
          cursor: 'pointer',
        },
        label: labelText,
        labelStyle: {
          fill: isActive ? '#16a34a' : '#6b7280',
          fontWeight: 600,
          fontSize: 12,
        },
        labelBgStyle: {
          fill: '#ffffff',
          fillOpacity: 0.9,
        },
        labelBgPadding: [8, 4] as [number, number],
        data: { setting },
      });
    });

    return edgeList;
  }, [settings, sourceAccounts, receiverAccounts]);

  return { nodes, edges };
}
