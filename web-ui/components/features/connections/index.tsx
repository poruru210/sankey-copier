'use client';

import { useEffect, useMemo } from 'react';
import { useIntlayer } from 'next-intlayer';
import { ReactFlowProvider } from '@xyflow/react';
import '@xyflow/react/dist/style.css';

import type { CopySettings, EaConnection, CreateSettingsRequest, TradeGroup } from '@/types';
import { useAtomValue, useSetAtom } from 'jotai';
import {
  connectionsAtom,
  settingsAtom,
  tradeGroupsAtom,
  localizationAtom,
} from '@/lib/atoms/data';
import {
  sourceAccountsAtom,
  receiverAccountsAtom,
} from '@/lib/atoms/computed';
import { useMasterFilter } from '@/hooks/useMasterFilter';
import { ConnectionsActionBar } from '@/components/features/connections/ConnectionsActionBar';
import { FilterIndicator } from '@/components/features/connections/FilterIndicator';
import { FlowCanvas } from '@/components/features/connections/FlowCanvas';
import { ConnectionsDialogManager } from '@/components/features/connections/ConnectionsDialogManager';

interface ConnectionsViewReactFlowProps {
  connections: EaConnection[];
  settings: CopySettings[];
  tradeGroups: TradeGroup[];
  onToggle: (id: number, enabled: boolean) => Promise<void>;
  onToggleMaster: (masterAccount: string, enabled: boolean) => Promise<void>;
  onCreate: (data: CreateSettingsRequest) => Promise<void>;
  onUpdate: (id: number, data: CopySettings) => Promise<void>;
  onDelete: (id: number) => Promise<void>;
}

function ConnectionsViewReactFlowInner({
  connections,
  settings,
  tradeGroups,
  onToggle,
  onToggleMaster,
  onCreate,
  onUpdate,
  onDelete,
}: ConnectionsViewReactFlowProps) {
  const content = useIntlayer('connections-view');

  // --- Initialize Atoms with props ---
  const setConnections = useSetAtom(connectionsAtom);
  const setSettings = useSetAtom(settingsAtom);
  const setTradeGroups = useSetAtom(tradeGroupsAtom);
  const setLocalization = useSetAtom(localizationAtom);

  useEffect(() => {
    setConnections(connections);
    setSettings(settings);
    setTradeGroups(tradeGroups);
    setLocalization({
      allSourcesInactive: content.allSourcesInactive,
      someSourcesInactive: content.someSourcesInactive,
      autoTradingDisabled: content.autoTradingDisabled,
    });
  }, [connections, settings, tradeGroups, content, setConnections, setSettings, setTradeGroups, setLocalization]);

  // Use derived atoms
  const sourceAccounts = useAtomValue(sourceAccountsAtom);
  const receiverAccounts = useAtomValue(receiverAccountsAtom);

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

  // Dialog Manager
  const {
    openCreateDialog,
    openEditDialog,
    openMasterSettings,
    handleDeleteSetting,
    renderDialogs,
  } = ConnectionsDialogManager({
    connections,
    settings,
    onCreate,
    onUpdate,
    onDelete,
  });

  // Memoize content object for FlowCanvas
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
      copySettings: content.copySettings,
      lotMultiplier: content.lotMultiplier,
      marginRatio: content.marginRatio,
      reverseTrade: content.reverseTrade,
      symbolRules: content.symbolRules,
      prefix: content.prefix,
      suffix: content.suffix,
      mappings: content.mappings,
      lotFilter: content.lotFilter,
      min: content.min,
      max: content.max,
      noSettings: content.noSettings,
    }),
    [content]
  );

  return (
    <div className="relative flex flex-col h-full">
      <ConnectionsActionBar
        connections={connections}
        settings={settings}
        selectedMaster={selectedMaster}
        onSelectMaster={setSelectedMaster}
        onCreateClick={openCreateDialog}
      />

      <div className="flex-1 min-w-0 flex flex-col min-h-0">
        <FilterIndicator
          selectedMaster={selectedMaster}
          selectedMasterName={selectedMasterName}
          onClearFilter={() => setSelectedMaster('all')}
        />

        <FlowCanvas
          connections={connections}
          settings={settings}
          visibleSourceAccounts={visibleSourceAccounts}
          visibleReceiverAccounts={visibleReceiverAccounts}
          selectedMaster={selectedMaster}
          onToggle={onToggle}
          onToggleMaster={onToggleMaster}
          handleEditSetting={openEditDialog}
          handleDeleteSetting={handleDeleteSetting}
          handleEditMasterSettings={openMasterSettings}
          accountCardContent={accountCardContent}
        />

        {renderDialogs}
      </div>
    </div>
  );
}

export function ConnectionsViewReactFlow(props: ConnectionsViewReactFlowProps) {
  return (
    <ReactFlowProvider>
      <ConnectionsViewReactFlowInner {...props} />
    </ReactFlowProvider>
  );
}
