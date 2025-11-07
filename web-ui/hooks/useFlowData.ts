import { useMemo } from 'react';
import { Node, Edge } from 'reactflow';
import type { AccountInfo, CopySettings, EaConnection } from '@/types';
import type { AccountNodeData, RelayServerNodeData } from '@/components/flow-nodes';

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
const RELAY_X = 450;
const RECEIVER_X = 900;

// Layout constants - Mobile (vertical)
const MOBILE_X = 0;
const MOBILE_SOURCE_START_Y = 0;
const MOBILE_VERTICAL_SPACING = 200;
const MOBILE_RELAY_OFFSET = 120; // Gap between sections

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
        draggable: true,
      });
    });

    // Calculate relay server position
    let relayPosition;
    if (isMobile) {
      // Mobile: position between source and receiver sections
      const sourceEndY = MOBILE_SOURCE_START_Y + (sourceAccounts.length - 1) * MOBILE_VERTICAL_SPACING;
      relayPosition = {
        x: MOBILE_X,
        y: sourceEndY + MOBILE_RELAY_OFFSET,
      };
    } else {
      // Desktop: center of all accounts
      const totalAccounts = sourceAccounts.length + receiverAccounts.length;
      const relayY = totalAccounts > 0
        ? ((sourceAccounts.length - 1) * VERTICAL_SPACING +
           (receiverAccounts.length - 1) * VERTICAL_SPACING) / 2
        : 0;
      relayPosition = { x: RELAY_X, y: relayY };
    }

    // Create relay server node
    nodeList.push({
      id: 'relay-server',
      type: 'relayServerNode',
      position: relayPosition,
      data: {
        label: 'Relay Server',
        isMobile,
      } as RelayServerNodeData,
      draggable: false,
    });

    // Create receiver account nodes
    receiverAccounts.forEach((account, index) => {
      const accountSettings = getAccountSettings(account.id, 'receiver');
      const connection = getAccountConnection(account.id);
      const isHighlighted = isAccountHighlighted(account.id, 'receiver');

      // Mobile: vertical layout, Desktop: horizontal layout
      const position = isMobile
        ? {
            x: MOBILE_X,
            y: relayPosition.y + MOBILE_RELAY_OFFSET + index * MOBILE_VERTICAL_SPACING,
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
        draggable: true,
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

    // Create edges from source accounts to relay server
    settings.forEach((setting) => {
      const sourceAccount = sourceAccounts.find(
        (acc) => acc.id === setting.master_account
      );

      if (!sourceAccount) return;

      const isActive = setting.enabled && !sourceAccount.hasError;

      // Edge from source to relay server
      edgeList.push({
        id: `source-${setting.master_account}-relay`,
        source: `source-${setting.master_account}`,
        target: 'relay-server',
        animated: isActive,
        style: {
          stroke: isActive ? '#3b82f6' : '#d1d5db',
          strokeWidth: 2,
          strokeDasharray: isActive ? undefined : '5,5',
        },
        data: { setting },
      });
    });

    // Create edges from relay server to receiver accounts
    settings.forEach((setting) => {
      const receiverAccount = receiverAccounts.find(
        (acc) => acc.id === setting.slave_account
      );

      if (!receiverAccount) return;

      const sourceAccount = sourceAccounts.find(
        (acc) => acc.id === setting.master_account
      );

      const isActive =
        setting.enabled &&
        !receiverAccount.hasError &&
        !receiverAccount.hasWarning &&
        sourceAccount &&
        !sourceAccount.hasError;

      // Edge from relay server to receiver
      edgeList.push({
        id: `relay-receiver-${setting.slave_account}-${setting.master_account}`,
        source: 'relay-server',
        target: `receiver-${setting.slave_account}`,
        animated: isActive,
        style: {
          stroke: isActive ? '#22c55e' : '#d1d5db',
          strokeWidth: 2,
          strokeDasharray: isActive ? undefined : '5,5',
        },
        data: { setting },
      });
    });

    return edgeList;
  }, [settings, sourceAccounts, receiverAccounts]);

  return { nodes, edges };
}
