'use client';

import { useCallback, useEffect, useRef, useMemo } from 'react';
import {
  ReactFlow,
  Background,
  Controls,
  NodeTypes,
  EdgeTypes,
  Node,
  NodeChange,
  useNodesState,
  useEdgesState,
  useReactFlow,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import { useAtomValue } from 'jotai';

import {
  expandedSourceIdsAtom,
  expandedReceiverIdsAtom,
} from '@/lib/atoms/ui';
import { useDagreLayout } from '@/hooks/useDagreLayout';
import { AccountNode } from '@/components/features/connections/flow-nodes/AccountNode';
import type { AccountNodeData } from '@/components/features/connections/flow-nodes/AccountNode';
import { SettingsEdge } from '@/components/features/connections/flow-edges';
import { useFlowData } from '@/hooks/useFlowData';
import { useConnectionHighlight } from '@/hooks/connections';
import { CopySettings, EaConnection } from '@/types';

// Define nodeTypes at module level to prevent recreation warnings
const nodeTypes = Object.freeze({
  accountNode: AccountNode,
}) as NodeTypes;

// Define edgeTypes at module level to prevent recreation warnings
const edgeTypes = Object.freeze({
  settingsEdge: SettingsEdge,
}) as EdgeTypes;

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
    setHoveredSource,
    setHoveredReceiver,
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

  // Track user-dragged nodes to preserve their positions
  const userDraggedNodesRef = useRef<Set<string>>(new Set());

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
  }, [visibleSourceAccounts.length, visibleReceiverAccounts.length, selectedMaster, applyLayout, initialNodes, initialEdges, setNodes]);

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

  // Track user drags
  const onNodesChangeWithTracking = useCallback(
    (changes: NodeChange[]) => {
      changes.forEach((change) => {
        if (change.type === 'position' && change.dragging && 'id' in change) {
          userDraggedNodesRef.current.add(change.id);
        }
      });
      onNodesChange(changes);
    },
    [onNodesChange]
  );

  // Handle node hover
  const onNodeMouseEnter = useCallback(
    (event: React.MouseEvent, node: Node) => {
      if (!isMobile) {
        if (node.id.startsWith('source-')) {
          const accountId = node.id.replace('source-', '');
          setHoveredSource(accountId);
        } else if (node.id.startsWith('receiver-')) {
          const accountId = node.id.replace('receiver-', '');
          setHoveredReceiver(accountId);
        }
      }
    },
    [isMobile, setHoveredSource, setHoveredReceiver]
  );

  const onNodeMouseLeave = useCallback(() => {
    if (!isMobile) {
      setHoveredSource(null);
      setHoveredReceiver(null);
    }
  }, [isMobile, setHoveredSource, setHoveredReceiver]);

  // Auto-fit view
  const reactFlowInstance = useReactFlow();

  useEffect(() => {
    if (nodes.length > 0 && reactFlowInstance) {
      const timer = setTimeout(() => {
        reactFlowInstance.fitView({
          padding: 0.2,
          duration: 800,
          maxZoom: 1,
        });
      }, 100);
      return () => clearTimeout(timer);
    }
  }, [nodes.length, reactFlowInstance]);

  // Resize handler
  useEffect(() => {
    if (!reactFlowInstance) return;
    let resizeTimer: NodeJS.Timeout;
    const handleResize = () => {
      clearTimeout(resizeTimer);
      resizeTimer = setTimeout(() => {
        reactFlowInstance.fitView({
          padding: 0.2,
          duration: 800,
          maxZoom: 1,
        });
      }, 300);
    };
    window.addEventListener('resize', handleResize);
    return () => {
      window.removeEventListener('resize', handleResize);
      clearTimeout(resizeTimer);
    };
  }, [reactFlowInstance]);

  return (
    <div className="flex-1 bg-gray-50 dark:bg-gray-900 rounded-lg border border-border overflow-hidden">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChangeWithTracking}
        onEdgesChange={onEdgesChange}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        onNodeMouseEnter={onNodeMouseEnter}
        onNodeMouseLeave={onNodeMouseLeave}
        nodesDraggable={true}
        nodeDragThreshold={1}
        nodesConnectable={false}
        nodesFocusable={true}
        edgesFocusable={false}
        selectNodesOnDrag={true}
        noDragClassName="noDrag"
        minZoom={0.1}
        maxZoom={2}
        proOptions={{ hideAttribution: true }}
      >
        <Background />
        <Controls
          className="!bg-white dark:!bg-gray-800 !border-gray-200 dark:!border-gray-700 [&>button]:!bg-white dark:[&>button]:!bg-gray-700 [&>button]:!border-gray-300 dark:[&>button]:!border-gray-600 [&>button]:hover:!bg-gray-50 dark:[&>button]:hover:!bg-gray-600 [&>button>svg]:!fill-gray-700 dark:[&>button>svg]:!fill-gray-200"
        />
      </ReactFlow>
    </div>
  );
}
