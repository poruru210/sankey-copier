'use client';

import { useState, useCallback } from 'react';
import {
  ReactFlow,
  Node,
  Edge,
  addEdge,
  Connection,
  useNodesState,
  useEdgesState,
  Background,
  Controls,
  MiniMap,
  NodeTypes,
  Handle,
  Position,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';

// Simple custom node for testing
function SimpleAccountNode({ data }: any) {
  return (
    <div className="bg-white border-2 border-gray-300 rounded-lg p-4 shadow-lg min-w-[300px]">
      {/* Drag handle area */}
      <div className="bg-blue-100 p-2 rounded cursor-move mb-2">
        <h3 className="font-bold text-sm">{data.label}</h3>
        <p className="text-xs text-gray-600">Drag me by this header</p>
      </div>

      {/* Interactive content */}
      <div className="noDrag space-y-2">
        <button
          className="px-3 py-1 bg-blue-500 text-white rounded text-xs hover:bg-blue-600"
          onClick={() => alert('Button clicked!')}
        >
          Click me (noDrag)
        </button>
        <input
          type="text"
          placeholder="Type here (noDrag)"
          className="w-full px-2 py-1 border rounded text-xs"
        />
      </div>

      {/* Handles */}
      <Handle
        type="source"
        position={Position.Right}
        className="!w-3 !h-3 !bg-blue-500"
      />
      <Handle
        type="target"
        position={Position.Left}
        className="!w-3 !h-3 !bg-green-500"
      />
    </div>
  );
}

// Simple relay node
function SimpleRelayNode() {
  return (
    <div className="bg-blue-500 text-white rounded-full w-20 h-20 flex items-center justify-center font-bold shadow-lg">
      <div>Relay</div>
      <Handle
        type="source"
        position={Position.Right}
        className="!w-3 !h-3 !bg-blue-500"
      />
      <Handle
        type="target"
        position={Position.Left}
        className="!w-3 !h-3 !bg-green-500"
      />
    </div>
  );
}

const nodeTypes: NodeTypes = {
  simpleAccount: SimpleAccountNode,
  simpleRelay: SimpleRelayNode,
};

const initialNodes: Node[] = [
  {
    id: '1',
    type: 'simpleAccount',
    position: { x: 0, y: 0 },
    data: { label: 'Source Account 1' },
  },
  {
    id: '2',
    type: 'simpleAccount',
    position: { x: 0, y: 200 },
    data: { label: 'Source Account 2' },
  },
  {
    id: 'relay',
    type: 'simpleRelay',
    position: { x: 400, y: 100 },
    data: {},
    draggable: false, // Relay server is not draggable
  },
  {
    id: '3',
    type: 'simpleAccount',
    position: { x: 800, y: 0 },
    data: { label: 'Receiver Account 1' },
  },
  {
    id: '4',
    type: 'simpleAccount',
    position: { x: 800, y: 200 },
    data: { label: 'Receiver Account 2' },
  },
];

const initialEdges: Edge[] = [
  { id: 'e1-relay', source: '1', target: 'relay', animated: true },
  { id: 'e2-relay', source: '2', target: 'relay', animated: true },
  { id: 'relay-e3', source: 'relay', target: '3', animated: true },
  { id: 'relay-e4', source: 'relay', target: '4', animated: true },
];

export function TestSimpleFlow() {
  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);

  const onConnect = useCallback(
    (params: Connection) => setEdges((eds) => addEdge(params, eds)),
    [setEdges]
  );

  return (
    <div className="w-full h-screen">
      <div className="p-4 bg-gray-100 border-b">
        <h1 className="text-2xl font-bold">React Flow Drag Test</h1>
        <p className="text-sm text-gray-600">
          Try dragging the account nodes by their blue headers. Buttons and inputs should not trigger dragging.
        </p>
      </div>

      <div className="w-full" style={{ height: 'calc(100vh - 100px)' }}>
        <ReactFlow
          nodes={nodes}
          edges={edges}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          onConnect={onConnect}
          nodeTypes={nodeTypes}
          nodesDraggable={true}
          nodesConnectable={true}
          nodesFocusable={true}
          noDragClassName="noDrag"
          fitView
        >
          <Background />
          <Controls />
          <MiniMap />
        </ReactFlow>
      </div>
    </div>
  );
}
