import React, { memo } from 'react';
import { Handle, Position, NodeProps, Node } from '@xyflow/react';
import type { AccountInfo, EaConnection, CopySettings } from '@/types';
import { AccountCard } from '@/components/connections/AccountCard';

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

// Type for React Flow node with AccountNodeData
// Use intersection with Record to satisfy Node's constraint
export type AccountNodeType = Node<AccountNodeData & Record<string, unknown>, 'accountNode'>;

/**
 * Custom React Flow node for account cards
 * Wraps the existing AccountCard component with React Flow handles
 *
 * Drag behavior: The node is draggable by clicking on the header area.
 * Interactive elements (switches, buttons) have the 'noDrag' class to prevent dragging.
 */
export const AccountNode = memo(({ data, selected }: NodeProps<AccountNodeType>) => {
  const { type, isMobile } = data;

  return (
    <div
      className="account-node relative"
      style={{ width: isMobile ? '100%' : '380px', maxWidth: isMobile ? '100%' : '380px' }}
    >
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
