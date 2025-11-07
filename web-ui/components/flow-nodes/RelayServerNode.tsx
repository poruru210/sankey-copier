import React, { memo } from 'react';
import { Handle, Position, NodeProps } from 'reactflow';
import { Server } from 'lucide-react';

export interface RelayServerNodeData {
  label?: string;
  isMobile?: boolean;
}

/**
 * Custom React Flow node for the relay server
 * Displays a server icon in the center with connection handles on both sides (desktop) or top/bottom (mobile)
 */
export const RelayServerNode = memo(({ data }: NodeProps<RelayServerNodeData>) => {
  const { isMobile = false } = data;

  return (
    <div className="relay-server-node">
      {/* Input handle from source accounts */}
      {/* Desktop: left side, Mobile: top */}
      <Handle
        type="target"
        position={isMobile ? Position.Top : Position.Left}
        className="!w-4 !h-4 !bg-blue-500 !border-2 !border-white"
        style={isMobile ? { top: -8 } : { left: -8 }}
      />

      {/* Server icon */}
      <div className="flex items-center justify-center w-20 h-20 bg-blue-500 rounded-full shadow-lg border-4 border-white dark:border-gray-800">
        <Server className="w-10 h-10 text-white" strokeWidth={2.5} />
      </div>

      {/* Output handle to receiver accounts */}
      {/* Desktop: right side, Mobile: bottom */}
      <Handle
        type="source"
        position={isMobile ? Position.Bottom : Position.Right}
        className="!w-4 !h-4 !bg-green-500 !border-2 !border-white"
        style={isMobile ? { bottom: -8 } : { right: -8 }}
      />
    </div>
  );
});

RelayServerNode.displayName = 'RelayServerNode';
