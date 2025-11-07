'use client';

import { useState, useCallback, useMemo, useEffect } from 'react';
import { useIntlayer } from 'next-intlayer';
import ReactFlow, {
  Background,
  Controls,
  MiniMap,
  NodeTypes,
  Edge,
  Node,
  ReactFlowProvider,
  useReactFlow,
  useNodesState,
  useEdgesState,
} from 'reactflow';
import 'reactflow/dist/style.css';

import type { CopySettings, EaConnection, CreateSettingsRequest } from '@/types';
import {
  useAccountData,
  useConnectionHighlight,
  useAccountToggle,
} from '@/hooks/connections';
import { useMasterFilter } from '@/hooks/useMasterFilter';
import { useFlowData } from '@/hooks/useFlowData';
import { AccountNode, RelayServerNode } from '@/components/flow-nodes';
import { SettingsDialog } from '@/components/SettingsDialog';
import { MasterAccountSidebarContainer } from '@/components/MasterAccountSidebarContainer';
import { Button } from '@/components/ui/button';
import { useToast } from '@/hooks/use-toast';
import { Plus, RefreshCw } from 'lucide-react';

interface ConnectionsViewReactFlowProps {
  connections: EaConnection[];
  settings: CopySettings[];
  onToggle: (id: number, currentStatus: boolean) => void;
  onCreate: (data: CreateSettingsRequest) => void;
  onUpdate: (id: number, data: CopySettings) => void;
  onDelete: (id: number) => void;
  isMobileDrawerOpen?: boolean;
  onCloseMobileDrawer?: () => void;
}

// Define custom node types for React Flow
const nodeTypes: NodeTypes = {
  accountNode: AccountNode,
  relayServerNode: RelayServerNode,
};

function ConnectionsViewReactFlowInner({
  connections,
  settings,
  onToggle,
  onCreate,
  onUpdate,
  onDelete,
  isMobileDrawerOpen,
  onCloseMobileDrawer,
}: ConnectionsViewReactFlowProps) {
  const content = useIntlayer('connections-view');
  const sidebarContent = useIntlayer('master-account-sidebar');
  const { toast } = useToast();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingSettings, setEditingSettings] = useState<CopySettings | null>(null);

  // Use custom hooks for account data management
  const {
    sourceAccounts,
    receiverAccounts,
    setSourceAccounts,
    setReceiverAccounts,
    getAccountConnection,
    getAccountSettings,
    toggleSourceExpand,
    toggleReceiverExpand,
  } = useAccountData({
    connections,
    settings,
    content: {
      allSourcesInactive: content.allSourcesInactive,
      someSourcesInactive: content.someSourcesInactive,
    },
  });

  // Use custom hook for hover/highlight management
  const {
    hoveredSourceId,
    hoveredReceiverId,
    selectedSourceId,
    setHoveredSource,
    setHoveredReceiver,
    isAccountHighlighted,
    isMobile,
  } = useConnectionHighlight(settings);

  // Use custom hook for toggle operations
  const { toggleSourceEnabled, toggleReceiverEnabled } = useAccountToggle({
    settings,
    sourceAccounts,
    receiverAccounts,
    setSourceAccounts,
    setReceiverAccounts,
    onToggle,
  });

  // Use custom hook for master account filtering
  const {
    selectedMaster,
    setSelectedMaster,
    visibleSourceAccounts,
    visibleReceiverAccounts,
    selectedMasterName,
  } = useMasterFilter({
    connections,
    settings,
    sourceAccounts,
    receiverAccounts,
  });

  // Handle settings dialog
  const handleOpenDialog = useCallback(() => {
    setEditingSettings(null);
    setDialogOpen(true);
  }, []);

  const handleEditSetting = useCallback((setting: CopySettings) => {
    setEditingSettings(setting);
    setDialogOpen(true);
  }, []);

  const handleDeleteSetting = useCallback(
    async (setting: CopySettings) => {
      if (window.confirm(`Delete setting: ${setting.master_account} → ${setting.slave_account}?`)) {
        try {
          await onDelete(setting.id);
          toast({
            title: content.settingsDeleted,
            description: `${setting.master_account} → ${setting.slave_account}`,
          });
        } catch (error) {
          toast({
            title: content.deleteFailed,
            description: error instanceof Error ? error.message : content.unknownError,
            variant: 'destructive',
          });
        }
      }
    },
    [onDelete, toast, content.settingsDeleted, content.deleteFailed, content.unknownError]
  );

  const handleSaveSettings = useCallback(
    async (data: CreateSettingsRequest | CopySettings) => {
      try {
        if ('id' in data) {
          // Update existing settings
          await onUpdate(data.id, data);
          toast({
            title: content.settingsUpdated,
            description: `${data.master_account} → ${data.slave_account}`,
          });
        } else {
          // Create new settings
          await onCreate(data);
          toast({
            title: content.settingsCreated,
            description: `${data.master_account} → ${data.slave_account}`,
          });
        }
        setDialogOpen(false);
      } catch (error) {
        toast({
          title: content.saveFailed,
          description: error instanceof Error ? error.message : content.unknownError,
          variant: 'destructive',
        });
      }
    },
    [onCreate, onUpdate, toast, content.settingsCreated, content.settingsUpdated, content.saveFailed, content.unknownError]
  );

  // Memoize content object to prevent unnecessary re-renders
  const accountCardContent = useMemo(
    () => ({
      settings: content.settings,
      accountInfo: content.accountInfo,
      accountNumber: content.accountNumber,
      platform: content.platform,
      broker: content.broker,
      leverage: content.leverage,
      server: content.server,
      balanceInfo: content.balanceInfo,
      balance: content.balance,
      equity: content.equity,
      currency: content.currency,
      connectionInfo: content.connectionInfo,
      status: content.status,
      online: content.online,
      offline: content.offline,
      receivers: content.receivers,
      sources: content.sources,
      lastHeartbeat: content.lastHeartbeat,
      fixError: content.fixError,
    }),
    [content]
  );

  // Convert account data to React Flow nodes and edges
  const { nodes: initialNodes, edges: initialEdges } = useFlowData({
    sourceAccounts: visibleSourceAccounts,
    receiverAccounts: visibleReceiverAccounts,
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
    content: accountCardContent,
  });

  // Use React Flow's state management for nodes and edges
  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);

  // Update nodes when source data changes (while preserving dragged positions)
  useEffect(() => {
    setNodes((currentNodes) => {
      // Preserve positions of existing nodes
      return initialNodes.map((newNode) => {
        const existingNode = currentNodes.find((n) => n.id === newNode.id);
        if (existingNode) {
          // Keep the existing position if node was already there
          return { ...newNode, position: existingNode.position };
        }
        return newNode;
      });
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [visibleSourceAccounts, visibleReceiverAccounts, settings]);

  // Update edges when settings change
  useEffect(() => {
    setEdges(initialEdges);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [settings]);

  // Handle edge click to show connection details
  const onEdgeClick = useCallback(
    (event: React.MouseEvent, edge: Edge) => {
      if (edge.data?.setting) {
        handleEditSetting(edge.data.setting);
      }
    },
    [handleEditSetting]
  );

  // Handle node hover for highlighting
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

  // Get React Flow instance for programmatic control
  const reactFlowInstance = useReactFlow();

  // Center view on RelayServer node when nodes change
  useEffect(() => {
    if (nodes.length > 0 && reactFlowInstance) {
      // Wait for layout to settle, then center on relay server
      const timer = setTimeout(() => {
        const relayNode = nodes.find(node => node.id === 'relay-server');
        if (relayNode) {
          // Center view on relay server node
          reactFlowInstance.setCenter(
            relayNode.position.x + 40, // +40 to account for node width (80px / 2)
            relayNode.position.y + 40, // +40 to account for node height (80px / 2)
            {
              zoom: 0.7,
              duration: 500,
            }
          );
        }
      }, 200);

      return () => clearTimeout(timer);
    }
  }, [nodes.length, reactFlowInstance]); // Only trigger on nodes.length change, not full nodes array

  return (
    <div className="relative flex gap-6 h-full">
      {/* Sidebar */}
      <MasterAccountSidebarContainer
        connections={connections}
        settings={settings}
        selectedMaster={selectedMaster}
        onSelectMaster={setSelectedMaster}
        isMobileDrawerOpen={isMobileDrawerOpen}
        onCloseMobileDrawer={onCloseMobileDrawer}
      />

      {/* Main Content */}
      <div className="flex-1 min-w-0 flex flex-col">
        {/* Action Bar */}
        <div className="mb-6 flex justify-between items-center">
          <h2 className="text-2xl font-bold">{content.tradingConnections}</h2>
          <div className="flex gap-2">
            <Button variant="outline" size="sm" onClick={() => window.location.reload()}>
              <RefreshCw className="h-4 w-4 mr-2" />
              {content.refresh}
            </Button>
            <Button size="sm" onClick={handleOpenDialog}>
              <Plus className="h-4 w-4 mr-2" />
              {content.createNewLink}
            </Button>
          </div>
        </div>

        {/* Filter Indicator */}
        {selectedMaster !== 'all' && selectedMasterName && (
          <div className="mb-4 flex items-center justify-between px-4 py-2 bg-accent rounded-lg border border-border animate-in fade-in slide-in-from-top-2 duration-300">
            <div className="flex items-center gap-2">
              <span className="text-sm font-medium">{sidebarContent.viewingAccount}:</span>
              <span className="text-sm text-muted-foreground">{selectedMasterName}</span>
            </div>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setSelectedMaster('all')}
              className="h-auto px-2 py-1"
            >
              {sidebarContent.clearFilter}
            </Button>
          </div>
        )}

        {/* React Flow Canvas */}
        <div className="flex-1 min-h-[800px] bg-gray-50 dark:bg-gray-900 rounded-lg border border-border overflow-hidden">
          <ReactFlow
            nodes={nodes}
            edges={edges}
            onNodesChange={onNodesChange}
            onEdgesChange={onEdgesChange}
            nodeTypes={nodeTypes}
            onEdgeClick={onEdgeClick}
            onNodeMouseEnter={onNodeMouseEnter}
            onNodeMouseLeave={onNodeMouseLeave}
            nodesDraggable={true}
            nodeDragThreshold={1}
            nodesConnectable={false}
            nodesFocusable={true}
            selectNodesOnDrag={true}
            noDragClassName="noDrag"
            minZoom={0.1}
            maxZoom={2}
            proOptions={{ hideAttribution: true }}
          >
            <Background />
            <Controls />
            <MiniMap
              nodeColor={(node) => {
                if (node.id === 'relay-server') return '#3b82f6';
                if (node.id.startsWith('source-')) return '#8b5cf6';
                return '#22c55e';
              }}
              className="!bg-white dark:!bg-gray-800"
            />
          </ReactFlow>
        </div>

        {/* Settings Dialog */}
        <SettingsDialog
          open={dialogOpen}
          onOpenChange={setDialogOpen}
          onSave={handleSaveSettings}
          initialData={editingSettings}
          connections={connections}
          existingSettings={settings}
        />
      </div>
    </div>
  );
}

/**
 * Wrapper component with ReactFlowProvider
 */
export function ConnectionsViewReactFlow(props: ConnectionsViewReactFlowProps) {
  return (
    <ReactFlowProvider>
      <ConnectionsViewReactFlowInner {...props} />
    </ReactFlowProvider>
  );
}
