import React, { memo } from 'react';
import { Handle, Position, NodeProps, Node } from '@xyflow/react';
import type { AccountInfo, EaConnection, CopySettings } from '@/types';
import { AccountNodeContent } from '@/components/connections/AccountNodeContent';

export interface AccountNodeData {
  account: AccountInfo;
  connection?: EaConnection;
  accountSettings: CopySettings[];
  onToggle: () => void;
  onToggleEnabled?: (enabled: boolean) => void;
  onEditSetting?: (setting: CopySettings) => void;
  onDeleteSetting?: (setting: CopySettings) => void;
  onEditMasterSettings?: () => void;
  onOpenSettingsDrawer?: () => void;
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
    // Copy Settings Carousel content
    copySettings: string;
    lotMultiplier: string;
    marginRatio: string;
    reverseTrade: string;
    symbolRules: string;
    prefix: string;
    suffix: string;
    mappings: string;
    lotFilter: string;
    min: string;
    max: string;
    noSettings: string;
  };
}

// Type for React Flow node with AccountNodeData
// Use intersection with Record to satisfy Node's constraint
export type AccountNodeType = Node<AccountNodeData & Record<string, unknown>, 'accountNode'>;

/**
 * Custom React Flow node for accounts
 * Wraps the AccountNodeContent component with React Flow handles
 *
 * Drag behavior: The node is draggable by clicking on the header area.
 * Interactive elements (switches, buttons) have the 'noDrag' class to prevent dragging.
 */
export const AccountNode = memo(({ data, selected }: NodeProps<AccountNodeType>) => {
  const { type, isMobile, account } = data;

  // Determine handle color based on account state (same logic as StatusIndicatorBar)
  const handleColorClass = account.hasWarning
    ? '!bg-yellow-500'  // Auto-trading OFF
    : account.isActive
    ? '!bg-green-500'   // Active (ready for trading)
    : '!bg-gray-300';   // Inactive or disabled

  return (
    <div
      className="account-node relative"
      data-account-id={account.id}
      data-testid="account-node"
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
            className={`!w-3 !h-3 ${handleColorClass} !border-2 !border-white`}
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
            className={`!w-3 !h-3 ${handleColorClass} !border-2 !border-white`}
            style={isMobile ? { top: -6 } : { left: -6 }}
          />
        </>
      )}

      {/* Render the AccountNodeContent component */}
      <AccountNodeContent
        account={data.account}
        connection={data.connection}
        accountSettings={data.accountSettings}
        onToggle={data.onToggle}
        onToggleEnabled={data.onToggleEnabled}
        onEditSetting={data.onEditSetting}
        onDeleteSetting={data.onDeleteSetting}
        onEditMasterSettings={data.onEditMasterSettings}
        onOpenSettingsDrawer={data.onOpenSettingsDrawer}
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
