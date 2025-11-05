'use client';

import { useMemo, useEffect, useState } from 'react';
import { useIntlayer } from 'next-intlayer';
import type { CopySettings, EaConnection, CreateSettingsRequest } from '@/types';
import {
  useAccountData,
  useConnectionHighlight,
  useAccountToggle,
  useAccountRefs,
  useSVGConnections,
} from '@/hooks/connections';
import { AccountCard } from '@/components/connections';
import { SettingsDialog } from '@/components/SettingsDialog';
import { Button } from '@/components/ui/button';
import { useToast } from '@/hooks/use-toast';
import { Plus, RefreshCw } from 'lucide-react';

// Layout constants - Responsive
// Mobile: Single column (vertical stack)
// Tablet: Narrower middle column
// Desktop: Full width middle column
const GRID_LAYOUT = 'grid grid-cols-1 gap-4 md:grid-cols-[1fr_120px_1fr] md:gap-6 lg:grid-cols-[1fr_200px_1fr]';
const ACCOUNT_LIST_WRAPPER = 'flex items-center justify-center';
const ACCOUNT_LIST = 'space-y-4 w-full max-w-md md:max-w-none';

interface ConnectionsViewProps {
  connections: EaConnection[];
  settings: CopySettings[];
  onToggle: (id: number, currentStatus: boolean) => void;
  onCreate: (data: CreateSettingsRequest) => void;
  onUpdate: (id: number, data: CopySettings) => void;
  onDelete: (id: number) => void;
}

export function ConnectionsView({
  connections,
  settings,
  onToggle,
  onCreate,
  onUpdate,
  onDelete,
}: ConnectionsViewProps) {
  const content = useIntlayer('connections-view');
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
    handleSourceTap,
    clearSelection,
    isAccountHighlighted,
    isMobile,
    getConnectedReceivers,
    getConnectedSources,
  } = useConnectionHighlight(settings);

  // Use custom hook for refs management
  const { sourceRefs, receiverRefs, registerSourceRef, registerReceiverRef } = useAccountRefs();

  // Use custom hook for toggle operations
  const { toggleSourceEnabled, toggleReceiverEnabled } = useAccountToggle({
    settings,
    sourceAccounts,
    receiverAccounts,
    setSourceAccounts,
    setReceiverAccounts,
    onToggle,
  });

  // Handle settings dialog
  const handleOpenDialog = () => {
    setEditingSettings(null);
    setDialogOpen(true);
  };

  const handleSaveSettings = async (data: CreateSettingsRequest | CopySettings) => {
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
  };

  // Auto-select first source on mobile
  useEffect(() => {
    if (isMobile && sourceAccounts.length > 0 && !selectedSourceId) {
      handleSourceTap(sourceAccounts[0].id);
    }
  }, [isMobile, sourceAccounts, selectedSourceId, handleSourceTap]);

  // Use custom hook for SVG connection drawing
  useSVGConnections({
    sourceAccounts,
    receiverAccounts,
    sourceRefs,
    receiverRefs,
    hoveredSourceId: isMobile ? selectedSourceId : hoveredSourceId,
    hoveredReceiverId: isMobile ? null : hoveredReceiverId,
    getConnectedReceivers,
    getConnectedSources,
  });

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

  return (
    <div className="relative">
      {/* Action Bar */}
      <div className="mb-6 flex justify-between items-center">
        <h2 className="text-2xl font-bold">{content.tradingConnections}</h2>
        <div className="flex gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => window.location.reload()}
          >
            <RefreshCw className="h-4 w-4 mr-2" />
            {content.refresh}
          </Button>
          <Button
            size="sm"
            onClick={handleOpenDialog}
          >
            <Plus className="h-4 w-4 mr-2" />
            {content.createNewLink}
          </Button>
        </div>
      </div>

      <div className="max-w-7xl mx-auto relative">
        <svg
          id="connection-svg"
          className="absolute inset-0 w-full h-full pointer-events-none"
          style={{ zIndex: 0 }}
        />

        {/* Main Content */}
        <div className="relative z-10 px-4 md:px-0">
          {/* Mobile: Source selection dropdown */}
          {isMobile && (
            <div className="mb-4 flex flex-col gap-2">
              <label htmlFor="source-select" className="text-sm font-medium text-gray-700 dark:text-gray-300">
                {content.selectSource}
              </label>
              <select
                id="source-select"
                value={selectedSourceId || sourceAccounts[0]?.id || ''}
                onChange={(e) => handleSourceTap(e.target.value)}
                className="w-full px-4 py-2 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
              >
                {sourceAccounts.map((account) => (
                  <option key={account.id} value={account.id}>
                    {account.name}
                  </option>
                ))}
              </select>
            </div>
          )}

          {/* Main Layout - Source and Receivers */}
          <div className={GRID_LAYOUT}>
            {/* Left Column: Source Accounts */}
            <div className={ACCOUNT_LIST_WRAPPER}>
              <div className={ACCOUNT_LIST}>
                {sourceAccounts.map((account) => {
                  const isHighlighted = isAccountHighlighted(account.id, 'source');
                  const connection = getAccountConnection(account.id);
                  const accountSettings = getAccountSettings(account.id, 'source');

                  return (
                    <div
                      key={account.id}
                      ref={registerSourceRef(account.id)}
                    >
                      <AccountCard
                        account={account}
                        connection={connection}
                        accountSettings={accountSettings}
                        onToggle={() => toggleSourceExpand(account.id)}
                        onToggleEnabled={(enabled) => toggleSourceEnabled(account.id, enabled)}
                        type="source"
                        onMouseEnter={() => !isMobile && setHoveredSource(account.id)}
                        onMouseLeave={() => !isMobile && setHoveredSource(null)}
                        isHighlighted={isHighlighted}
                        hoveredSourceId={hoveredSourceId}
                        hoveredReceiverId={hoveredReceiverId}
                        selectedSourceId={selectedSourceId}
                        isMobile={isMobile}
                        content={accountCardContent}
                      />
                    </div>
                  );
                })}
              </div>
            </div>

            {/* Middle Column: Server indicator */}
            <div className={`${ACCOUNT_LIST_WRAPPER} my-2 md:my-0`}>
              {/* Server icon will be drawn here by SVG */}
            </div>

            {/* Right Column: Receiver Accounts */}
            <div className={ACCOUNT_LIST_WRAPPER}>
              <div className={ACCOUNT_LIST}>
                {receiverAccounts.map((account) => {
                  const isHighlighted = isAccountHighlighted(account.id, 'receiver');
                  const connection = getAccountConnection(account.id);
                  const accountSettings = getAccountSettings(account.id, 'receiver');

                  return (
                    <div key={account.id} ref={registerReceiverRef(account.id)}>
                      <AccountCard
                        account={account}
                        connection={connection}
                        accountSettings={accountSettings}
                        onToggle={() => toggleReceiverExpand(account.id)}
                        onToggleEnabled={(enabled) => toggleReceiverEnabled(account.id, enabled)}
                        type="receiver"
                        onMouseEnter={() => setHoveredReceiver(account.id)}
                        onMouseLeave={() => setHoveredReceiver(null)}
                        isHighlighted={isHighlighted}
                        hoveredSourceId={hoveredSourceId}
                        hoveredReceiverId={hoveredReceiverId}
                        selectedSourceId={selectedSourceId}
                        isMobile={isMobile}
                        content={accountCardContent}
                      />
                    </div>
                  );
                })}
              </div>
            </div>
          </div>
        </div>
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
  );
}
