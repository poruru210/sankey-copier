import { useMemo } from 'react';
import { Node, Edge } from '@xyflow/react';
import type { AccountInfo, CopySettings, EaConnection } from '@/types';
import type { AccountNodeData } from '@/components/flow-nodes';

interface UseFlowDataProps {
  sourceAccounts: AccountInfo[];
  receiverAccounts: AccountInfo[];
  settings: CopySettings[];
  getAccountConnection: (accountId: string) => EaConnection | undefined;
  getAccountSettings: (accountId: string, type: 'source' | 'receiver') => CopySettings[];
  toggleSourceExpand: (id: string) => void;
  toggleReceiverExpand: (id: string) => void;
  toggleSourceEnabled: (id: string, enabled: boolean) => void;
  toggleReceiverEnabled: (id: string, enabled: boolean) => void;
  handleEditSetting: (setting: CopySettings) => void;
  handleDeleteSetting: (setting: CopySettings) => void;
  hoveredSourceId: string | null;
  hoveredReceiverId: string | null;
  selectedSourceId: string | null;
  isAccountHighlighted: (accountId: string, type: 'source' | 'receiver') => boolean;
  isMobile: boolean;
  content: any;
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
  toggleSourceExpand,
  toggleReceiverExpand,
  toggleSourceEnabled,
  toggleReceiverEnabled,
  handleEditSetting,
  handleDeleteSetting,
  hoveredSourceId,
  hoveredReceiverId,
  selectedSourceId,
  isAccountHighlighted,
  isMobile,
  content,
}: UseFlowDataProps): { nodes: Node[]; edges: Edge[] } {
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
        } as AccountNodeData,
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
        } as AccountNodeData,
      });
    });

    return nodeList;
  }, [
    sourceAccounts,
    receiverAccounts,
    getAccountConnection,
    getAccountSettings,
    toggleSourceExpand,
    toggleReceiverExpand,
    toggleSourceEnabled,
    toggleReceiverEnabled,
    handleEditSetting,
    handleDeleteSetting,
    hoveredSourceId,
    hoveredReceiverId,
    selectedSourceId,
    isAccountHighlighted,
    isMobile,
    content,
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

      const isActive =
        setting.enabled &&
        !sourceAccount.hasError &&
        !receiverAccount.hasError &&
        !receiverAccount.hasWarning;

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
        id: `${setting.master_account}-${setting.slave_account}`,
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
