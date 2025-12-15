'use client';

import { useCallback, useEffect, useMemo, useRef } from 'react';
import {
  useNodesState,
  useEdgesState,
} from '@xyflow/react';
import { useAtomValue } from 'jotai';

import {
  expandedSourceIdsAtom,
  expandedReceiverIdsAtom,
} from '@/lib/atoms/ui';
import { useDagreLayout } from '@/hooks/useDagreLayout';
import { useFlowData, useConnectionHighlight, useFlowInteractions } from '@/hooks/connections';
import { CopySettings, EaConnection } from '@/types';
import type { AccountNodeData } from '@/components/features/connections/flow-nodes/AccountNode';
import { FlowGraph } from './FlowGraph';

interface FlowCanvasProps {
  connections: EaConnection[];
  settings: CopySettings[];
  visibleSourceAccounts: any[];
  visibleReceiverAccounts: any[];
  selectedMaster: string;
  onToggle: (id: number, enabled: boolean) => Promise<void>;
  onToggleMaster: (masterAccount: string, enabled: boolean) => Promise<void>;
  handleEditSetting: (setting: CopySettings) => void;
  handleDeleteSetting: (setting: CopySettings) => Promise<void>;
  handleEditMasterSettings: (masterAccount: string) => void;
  accountCardContent: any;
}

export function FlowCanvas({
  connections,
  settings,
  visibleSourceAccounts,
  visibleReceiverAccounts,
  selectedMaster,
  onToggle,
  onToggleMaster,
  handleEditSetting,
  handleDeleteSetting,
  handleEditMasterSettings,
  accountCardContent,
}: FlowCanvasProps) {
  // Use custom hook for hover/highlight management
  const {
    hoveredSourceId,
    hoveredReceiverId,
    isAccountHighlighted,
    isMobile,
  } = useConnectionHighlight(settings);

  // Helper functions required by useFlowData
  const getAccountConnection = useCallback((accountId: string) => {
    return connections.find((c) => c.account_id === accountId);
  }, [connections]);

  const getAccountSettings = useCallback((accountId: string, type: 'source' | 'receiver') => {
    if (type === 'source') {
      return settings.filter((s) => s.master_account === accountId);
    } else {
      return settings.filter((s) => s.slave_account === accountId);
    }
  }, [settings]);

  // Convert account data to React Flow nodes and edges
  const { nodes: initialNodes, edges: initialEdges, pendingAccountIds } = useFlowData({
    sourceAccounts: visibleSourceAccounts,
    receiverAccounts: visibleReceiverAccounts,
    settings,
    getAccountConnection,
    getAccountSettings,
    handleEditSetting,
    handleDeleteSetting,
    handleEditMasterSettings,
    isAccountHighlighted,
    isMobile,
    content: accountCardContent,
    onToggle,
    onToggleMaster,
  });

  // Use React Flow's state management for nodes and edges
  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes as any);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);

  // Interaction logic
  const { onNodesChangeWithTracking, onNodeMouseEnter, onNodeMouseLeave, userDraggedNodesRef } = useFlowInteractions({
    isMobile,
    onNodesChange,
  });

  // atoms: expanded ids
  const expandedSourceIds = useAtomValue(expandedSourceIdsAtom);
  const expandedReceiverIds = useAtomValue(expandedReceiverIdsAtom);

  // Derive IDs of accounts with warnings/errors for layout adjustment
  const warningAccountIds = useMemo(() => {
    const ids: string[] = [];
    visibleSourceAccounts.forEach((acc) => {
      if (acc.hasWarning || acc.hasError) ids.push(acc.id);
    });
    visibleReceiverAccounts.forEach((acc) => {
      if (acc.hasWarning || acc.hasError) ids.push(acc.id);
    });
    return ids;
  }, [visibleSourceAccounts, visibleReceiverAccounts]);

  // --- Dagre layout ---
  const { applyLayout } = useDagreLayout({
    expandedSourceIds,
    expandedReceiverIds,
    warningNodeIds: warningAccountIds,
    direction: 'LR',
    nodeSpacing: 30,
    rankSpacing: 200,
  });

  // Track node count and filter for layout recalculation
  const layoutTriggerRef = useRef({
    nodeCount: 0,
    selectedMaster: null as string | null,
  });

  // Effect 1: Apply layout when node count or filter changes (full reset)
  useEffect(() => {
    const currentNodeCount = visibleSourceAccounts.length + visibleReceiverAccounts.length;
    const prev = layoutTriggerRef.current;

    if (currentNodeCount !== prev.nodeCount || selectedMaster !== prev.selectedMaster) {
      layoutTriggerRef.current = { nodeCount: currentNodeCount, selectedMaster };
      userDraggedNodesRef.current.clear();

      const { nodes: layoutedNodes } = applyLayout(initialNodes, initialEdges);
      setNodes(layoutedNodes);
    }
  }, [visibleSourceAccounts.length, visibleReceiverAccounts.length, selectedMaster, applyLayout, initialNodes, initialEdges, setNodes, userDraggedNodesRef]);

  // Compute hash of warning codes to trigger layout updates
  const warningStateHash = useMemo(() => {
    return initialNodes
      .map((node) => {
        const data = node.data as unknown as AccountNodeData;
        const account = data.account;
        return `${node.id}:${account?.hasWarning}:${account?.errorMsg || ''}`;
      })
      .join('|');
  }, [initialNodes]);

  // Effect 2: Apply layout when expansion or warnings change (preserve dragged positions)
  useEffect(() => {
    const { nodes: layoutedNodes } = applyLayout(initialNodes, initialEdges);

    setNodes((currentNodes) => {
      if (currentNodes.length === 0) return layoutedNodes;

      return layoutedNodes.map((layoutedNode) => {
        const existingNode = currentNodes.find((n) => n.id === layoutedNode.id);
        if (existingNode && userDraggedNodesRef.current.has(layoutedNode.id)) {
          return { ...layoutedNode, position: existingNode.position };
        }
        return layoutedNode;
      });
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [expandedSourceIds, expandedReceiverIds, warningStateHash]);

  // Effect 3: Update node data without changing positions
  useEffect(() => {
    setNodes((currentNodes) =>
      currentNodes.map((node) => {
        const newNode = initialNodes.find((n) => n.id === node.id);
        if (newNode) {
          return { ...node, data: newNode.data };
        }
        return node;
      })
    );
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [hoveredSourceId, hoveredReceiverId, settings, pendingAccountIds.size]);

  // Effect 4: Update edges when settings change
  useEffect(() => {
    setEdges(initialEdges);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [settings]);

  return (
    <FlowGraph
      nodes={nodes}
      edges={edges}
      onNodesChange={onNodesChangeWithTracking}
      onEdgesChange={onEdgesChange}
      onNodeMouseEnter={onNodeMouseEnter}
      onNodeMouseLeave={onNodeMouseLeave}
    />
  );
}
