import React, { memo } from 'react';
import { Handle, Position, NodeProps, NodeToolbar } from 'reactflow';
import type { AccountInfo, EaConnection, CopySettings } from '@/types';
import { AccountCard } from '@/components/connections/AccountCard';
import { GripVertical } from 'lucide-react';

export interface AccountNodeData {
  account: AccountInfo;
  connection?: EaConnection;
  accountSettings: CopySettings[];
  onToggle: () => void;
  onToggleEnabled?: (enabled: boolean) => void;
  onEditSetting?: (setting: CopySettings) => void;
  onDeleteSetting?: (setting: CopySettings) => void;
  type: 'source' | 'receiver';
  isHighlighted?: boolean;
  hoveredSourceId: string | null;
  hoveredReceiverId: string | null;
  selectedSourceId: string | null;
  isMobile: boolean;
  content: {
    settings: string;
    accountInfo: string;
    accountNumber: string;
    platform: string;
    broker: string;
    leverage: string;
    server: string;
    balanceInfo: string;
    balance: string;
    equity: string;
    currency: string;
    connectionInfo: string;
    status: string;
    online: string;
    offline: string;
    receivers: string;
    sources: string;
    lastHeartbeat: string;
    fixError: string;
  };
}

/**
 * Custom React Flow node for account cards
 * Wraps the existing AccountCard component with React Flow handles
 *
 * Drag behavior: The entire node background is draggable EXCEPT for interactive elements
 * which have the 'noDrag' class applied internally
 */
export const AccountNode = memo(({ data, selected }: NodeProps<AccountNodeData>) => {
  const { type, isMobile } = data;

  return (
    <div className="account-node relative group">
      {/* Drag indicator - shows when hovering over draggable area */}
      <div
        className="absolute -left-7 top-1/2 -translate-y-1/2 opacity-0 group-hover:opacity-60 transition-opacity pointer-events-none z-10"
        title="Drag to reposition"
      >
        <div className="bg-blue-500 text-white rounded p-1">
          <GripVertical className="w-4 h-4" />
        </div>
      </div>

      {/* Connection handles - position based on mobile/desktop and source/receiver type */}

      {/* Source account handles */}
      {type === 'source' && (
        <>
          {/* Desktop: right side, Mobile: bottom */}
          <Handle
            type="source"
            position={isMobile ? Position.Bottom : Position.Right}
            className="!w-3 !h-3 !bg-blue-500 !border-2 !border-white"
            style={isMobile ? { bottom: -6 } : { right: -6 }}
          />
        </>
      )}

      {/* Receiver account handles */}
      {type === 'receiver' && (
        <>
          {/* Desktop: left side, Mobile: top */}
          <Handle
            type="target"
            position={isMobile ? Position.Top : Position.Left}
            className="!w-3 !h-3 !bg-green-500 !border-2 !border-white"
            style={isMobile ? { top: -6 } : { left: -6 }}
          />
        </>
      )}

      {/* Render the existing AccountCard component */}
      <AccountCard
        account={data.account}
        connection={data.connection}
        accountSettings={data.accountSettings}
        onToggle={data.onToggle}
        onToggleEnabled={data.onToggleEnabled}
        onEditSetting={data.onEditSetting}
        onDeleteSetting={data.onDeleteSetting}
        type={data.type}
        isHighlighted={data.isHighlighted}
        hoveredSourceId={data.hoveredSourceId}
        hoveredReceiverId={data.hoveredReceiverId}
        selectedSourceId={data.selectedSourceId}
        isMobile={data.isMobile}
        content={data.content}
      />
    </div>
  );
});

AccountNode.displayName = 'AccountNode';
