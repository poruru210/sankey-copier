'use client';

import {
  ReactFlow,
  Background,
  Controls,
  NodeTypes,
  EdgeTypes,
  Node,
  Edge,
  NodeChange,
  EdgeChange,
  useReactFlow,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import { useEffect } from 'react';
import { AccountNode } from '@/components/features/connections/flow-nodes/AccountNode';
import { SettingsEdge } from '@/components/features/connections/flow-edges';

// Define nodeTypes at module level to prevent recreation warnings
const nodeTypes = Object.freeze({
  accountNode: AccountNode,
}) as NodeTypes;

// Define edgeTypes at module level to prevent recreation warnings
const edgeTypes = Object.freeze({
  settingsEdge: SettingsEdge,
}) as EdgeTypes;

interface FlowGraphProps {
  nodes: Node[];
  edges: Edge[];
  onNodesChange: (changes: NodeChange[]) => void;
  onEdgesChange: (changes: EdgeChange[]) => void;
  onNodeMouseEnter: (event: React.MouseEvent, node: Node) => void;
  onNodeMouseLeave: (event: React.MouseEvent, node: Node) => void;
}

export function FlowGraph({
  nodes,
  edges,
  onNodesChange,
  onEdgesChange,
  onNodeMouseEnter,
  onNodeMouseLeave,
}: FlowGraphProps) {
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
        onNodesChange={onNodesChange}
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
        <Controls className="!bg-white dark:!bg-gray-800 !border-gray-200 dark:!border-gray-700 [&>button]:!bg-white dark:[&>button]:!bg-gray-700 [&>button]:!border-gray-300 dark:[&>button]:!border-gray-600 [&>button]:hover:!bg-gray-50 dark:[&>button]:hover:!bg-gray-600 [&>button>svg]:!fill-gray-700 dark:[&>button>svg]:!fill-gray-200" />
      </ReactFlow>
    </div>
  );
}
