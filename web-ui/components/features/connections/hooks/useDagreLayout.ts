import { useCallback } from 'react';
import { Node, Edge } from '@xyflow/react';
import dagre from '@dagrejs/dagre';

// Height constants
const COLLAPSED_HEIGHT = 96;
const EXPANDED_HEIGHT = 520;  // 展開時の高さ（マージン含む）
const WARNING_EXTRA_HEIGHT = 60; // 警告表示時の追加高さ
const NODE_WIDTH = 380;

interface UseDagreLayoutOptions {
  /** Expanded source account IDs */
  expandedSourceIds: string[];
  /** Expanded receiver account IDs */
  expandedReceiverIds: string[];
  /** IDs of nodes showing warning messages */
  warningNodeIds?: string[];
  /** Direction: 'LR' for left-to-right (horizontal), 'TB' for top-to-bottom */
  direction?: 'LR' | 'TB';
  /** Spacing between nodes */
  nodeSpacing?: number;
  /** Spacing between ranks (columns in LR, rows in TB) */
  rankSpacing?: number;
}

interface UseDagreLayoutReturn {
  applyLayout: (nodes: Node[], edges: Edge[]) => { nodes: Node[]; edges: Edge[] };
}

/**
 * Hook to apply dagre layout to React Flow nodes
 * Uses dagre library for automatic graph layout
 */
export function useDagreLayout(
  options: UseDagreLayoutOptions
): UseDagreLayoutReturn {
  const {
    expandedSourceIds,
    expandedReceiverIds,
    warningNodeIds = [],
    direction = 'LR',
    nodeSpacing = 30,
    rankSpacing = 200,
  } = options;

  // Check if a node is expanded
  const isNodeExpanded = useCallback(
    (nodeId: string): boolean => {
      if (nodeId.startsWith('source-')) {
        const accountId = nodeId.replace('source-', '');
        return expandedSourceIds.includes(accountId);
      } else if (nodeId.startsWith('receiver-')) {
        const accountId = nodeId.replace('receiver-', '');
        return expandedReceiverIds.includes(accountId);
      }
      return false;
    },
    [expandedSourceIds, expandedReceiverIds]
  );

  // Check if a node has warning
  const hasWarning = useCallback(
    (nodeId: string): boolean => {
      // warningNodeIds contains full node IDs (including source-/receiver- prefix) or account IDs?
      // Convention: let's expect Account IDs in warningNodeIds for consistency with expandedIds?
      // Actually ConnectionsView passes node objects usually.
      // Let's assume warningNodeIds contains ACCOUNT IDs for consistency with expandedSourceIds.
      if (nodeId.startsWith('source-')) {
        const accountId = nodeId.replace('source-', '');
        return warningNodeIds.includes(accountId);
      } else if (nodeId.startsWith('receiver-')) {
        const accountId = nodeId.replace('receiver-', '');
        return warningNodeIds.includes(accountId);
      }
      return false;
    },
    [warningNodeIds]
  );

  // Get node height based on expansion state
  const getNodeHeight = useCallback(
    (nodeId: string): number => {
      let height = isNodeExpanded(nodeId) ? EXPANDED_HEIGHT : COLLAPSED_HEIGHT;
      if (hasWarning(nodeId)) {
        height += WARNING_EXTRA_HEIGHT;
      }
      return height;
    },
    [isNodeExpanded, hasWarning]
  );

  // Apply dagre layout
  const applyLayout = useCallback(
    (nodes: Node[], edges: Edge[]): { nodes: Node[]; edges: Edge[] } => {
      if (nodes.length === 0) {
        return { nodes, edges };
      }

      // Create a new dagre graph
      const dagreGraph = new dagre.graphlib.Graph();
      dagreGraph.setDefaultEdgeLabel(() => ({}));

      // Configure the graph
      dagreGraph.setGraph({
        rankdir: direction,
        nodesep: nodeSpacing,  // Spacing between nodes in same rank
        ranksep: rankSpacing,  // Spacing between ranks
        marginx: 50,
        marginy: 50,
      });

      // Add nodes to dagre graph with their dimensions
      nodes.forEach((node) => {
        const height = getNodeHeight(node.id);
        dagreGraph.setNode(node.id, {
          width: NODE_WIDTH,
          height: height,
        });
      });

      // Add edges to dagre graph
      edges.forEach((edge) => {
        dagreGraph.setEdge(edge.source, edge.target);
      });

      // Run dagre layout
      dagre.layout(dagreGraph);

      // Apply calculated positions to nodes
      const layoutedNodes = nodes.map((node) => {
        const nodeWithPosition = dagreGraph.node(node.id);
        const height = getNodeHeight(node.id);

        return {
          ...node,
          position: {
            // Dagre returns center position, we need top-left
            x: nodeWithPosition.x - NODE_WIDTH / 2,
            y: nodeWithPosition.y - height / 2,
          },
        };
      });

      return { nodes: layoutedNodes, edges };
    },
    [direction, nodeSpacing, rankSpacing, getNodeHeight]
  );

  return {
    applyLayout,
  };
}
