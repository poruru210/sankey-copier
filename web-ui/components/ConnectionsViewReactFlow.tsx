'use client';

import { useState, useCallback, useMemo, useEffect } from 'react';
import { useIntlayer } from 'next-intlayer';
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  NodeTypes,
  Edge,
  Node,
  ReactFlowProvider,
  useNodesState,
  useEdgesState,
  useReactFlow,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';

import type { CopySettings, EaConnection, CreateSettingsRequest } from '@/types';
import {
  useAccountData,
  useConnectionHighlight,
  useAccountToggle,
} from '@/hooks/connections';
import { useMasterFilter } from '@/hooks/useMasterFilter';
import { useFlowData } from '@/hooks/useFlowData';
import { AccountNode } from '@/components/flow-nodes/AccountNode';
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

// Define nodeTypes at module level to prevent recreation warnings
const nodeTypes: NodeTypes = Object.freeze({
  accountNode: AccountNode,
});

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

  // Track selected master to detect filter changes
  const [prevSelectedMaster, setPrevSelectedMaster] = useState(selectedMaster);

  // Update nodes when source data changes (while preserving dragged positions)
  useEffect(() => {
    // Check if filter changed
    const filterChanged = prevSelectedMaster !== selectedMaster;
    if (filterChanged) {
      setPrevSelectedMaster(selectedMaster);
    }

    setNodes((currentNodes) => {
      // When switching to 'all' accounts OR filter changed, reset all node positions
      if (selectedMaster === 'all' && filterChanged) {
        return initialNodes;
      }

      // Preserve positions of ALL existing nodes (even after data updates)
      const updatedNodes = initialNodes.map((newNode) => {
        const existingNode = currentNodes.find((n) => n.id === newNode.id);
        if (existingNode) {
          // Always keep the existing position - this preserves dragged positions
          return { ...newNode, position: existingNode.position };
        }
        return newNode;
      });

      return updatedNodes;
    });
    // Only re-run when actual data changes, not hover states
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [visibleSourceAccounts, visibleReceiverAccounts, settings, selectedMaster]);

  // Update node data when hover state changes (without changing positions)
  useEffect(() => {
    setNodes((currentNodes) =>
      currentNodes.map((node) => {
        const newNode = initialNodes.find((n) => n.id === node.id);
        if (newNode) {
          // Update only the data, preserve position and other properties
          return { ...node, data: newNode.data };
        }
        return node;
      })
    );
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [hoveredSourceId, hoveredReceiverId]);

  // Update edges when data changes
  useEffect(() => {
    setEdges(initialEdges);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [visibleSourceAccounts, visibleReceiverAccounts, settings]);

  // Handle edge click to edit connection
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

  // Get React Flow instance for fitView
  const reactFlowInstance = useReactFlow();

  // Center and fit view when nodes are loaded
  useEffect(() => {
    if (nodes.length > 0 && reactFlowInstance) {
      // Wait for layout to settle, then fit view
      const timer = setTimeout(() => {
        reactFlowInstance.fitView({
          padding: 0.2, // 20% padding around nodes
          duration: 800, // Smooth animation
          maxZoom: 1, // Don't zoom in too much
        });
      }, 100);

      return () => clearTimeout(timer);
    }
  }, [nodes.length, reactFlowInstance]);

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
        {selectedMaster !== 'all' && selectedMasterName && (() => {
          // Split account name into broker and account number
          const lastUnderscoreIndex = selectedMasterName.lastIndexOf('_');
          const brokerName = lastUnderscoreIndex === -1
            ? selectedMasterName
            : selectedMasterName.substring(0, lastUnderscoreIndex).replace(/_/g, ' ');
          const accountNumber = lastUnderscoreIndex === -1
            ? ''
            : selectedMasterName.substring(lastUnderscoreIndex + 1);

          return (
            <div className="mb-4 flex items-center justify-between px-4 py-2 bg-accent rounded-lg border border-border animate-in fade-in slide-in-from-top-2 duration-300">
              <div className="flex items-center gap-2">
                <span className="text-sm font-medium">{sidebarContent.viewingAccount}:</span>
                <div className="flex flex-col">
                  <span className="text-sm text-muted-foreground font-medium">{brokerName}</span>
                  {accountNumber && (
                    <span className="text-xs text-muted-foreground">{accountNumber}</span>
                  )}
                </div>
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
          );
        })()}

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
            edgesFocusable={true}
            selectNodesOnDrag={true}
            noDragClassName="noDrag"
            minZoom={0.1}
            maxZoom={2}
            proOptions={{ hideAttribution: true }}
          >
            <Background />
            <Controls className="!bg-white dark:!bg-gray-800 !border-gray-200 dark:!border-gray-700 [&>button]:!bg-white dark:[&>button]:!bg-gray-700 [&>button]:!border-gray-300 dark:[&>button]:!border-gray-600 [&>button]:hover:!bg-gray-50 dark:[&>button]:hover:!bg-gray-600 [&>button>svg]:!fill-gray-700 dark:[&>button>svg]:!fill-gray-200" />
            <MiniMap
              nodeColor={(node) => {
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
          onDelete={handleDeleteSetting}
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
