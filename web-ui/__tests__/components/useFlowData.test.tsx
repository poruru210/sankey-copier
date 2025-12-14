import { renderHook } from '@testing-library/react';
import type { ReactNode } from 'react';
import { Provider } from 'jotai';
import { describe, it, expect } from 'vitest';
import type { AccountInfo, CopySettings } from '@/types';
import { useFlowData } from '@/hooks/useFlowData';
import type { AccountNodeData } from '@/components/features/connections/flow-nodes/AccountNode';
import type { Node } from '@xyflow/react';

const contentStub = {
  settings: 'settings',
  accountInfo: 'account info',
  accountNumber: 'account number',
  platform: 'platform',
  broker: 'broker',
  leverage: 'leverage',
  server: 'server',
  balanceInfo: 'balance info',
  balance: 'balance',
  equity: 'equity',
  currency: 'currency',
  connectionInfo: 'connection info',
  status: 'status',
  online: 'online',
  offline: 'offline',
  receivers: 'receivers',
  sources: 'sources',
  lastHeartbeat: 'last heartbeat',
  fixError: 'fix error',
  copySettings: 'copy settings',
  lotMultiplier: 'lot multiplier',
  marginRatio: 'margin ratio',
  reverseTrade: 'reverse trade',
  symbolRules: 'symbol rules',
  prefix: 'prefix',
  suffix: 'suffix',
  mappings: 'mappings',
  lotFilter: 'lot filter',
  min: 'min',
  max: 'max',
  noSettings: 'no settings',
};

function createAccount(id: string, overrides: Partial<AccountInfo> = {}): AccountInfo {
  return {
    id,
    name: id,
    accountType: 'master',
    isOnline: true,
    isEnabled: true,
    isActive: true,
    hasError: false,
    hasWarning: false,
    errorMsg: '',
    isExpanded: false,
    ...overrides,
  };
}

function createSetting(id: number, master: string, slave: string): CopySettings {
  return {
    id,
    status: 2,
    enabled_flag: true,
    master_account: master,
    slave_account: slave,
    lot_multiplier: 1,
    reverse_trade: false,
    symbol_mappings: [],
    filters: {
      allowed_symbols: null,
      blocked_symbols: null,
      allowed_magic_numbers: null,
      blocked_magic_numbers: null,
    },
  };
}

const withJotaiProvider = ({ children }: { children: ReactNode }) => (
  <Provider>{children}</Provider>
);

function renderFlowData({
  sourceAccounts,
  receiverAccounts,
  settings,
}: {
  sourceAccounts: AccountInfo[];
  receiverAccounts: AccountInfo[];
  settings: CopySettings[];
}) {
  const getAccountSettings = (accountId: string, type: 'source' | 'receiver') => {
    if (type === 'source') {
      return settings.filter((setting) => setting.master_account === accountId);
    }
    return settings.filter((setting) => setting.slave_account === accountId);
  };

  const hookProps = {
    sourceAccounts,
    receiverAccounts,
    settings,
    getAccountConnection: () => undefined,
    getAccountSettings,
    handleEditSetting: () => {},
    handleDeleteSetting: () => {},
    handleEditMasterSettings: undefined,
    isAccountHighlighted: () => false,
    isMobile: false,
    content: contentStub,
    onToggle: async () => {},
    onToggleMaster: async () => {},
  };

  const { result } = renderHook(() => useFlowData(hookProps), { wrapper: withJotaiProvider });
  return result.current;
}

function getNodeData(nodes: Node[], id: string): AccountNodeData {
  const target = nodes.find((node) => node.id === id);
  if (!target) {
    throw new Error(`Node ${id} was not created`);
  }
  return target.data as unknown as AccountNodeData;
}

describe('useFlowData master/slave combinations', () => {
  it('creates nodes and edges for 1:N master to slave relationships', () => {
    const sourceAccounts = [createAccount('MASTER-1')];
    const receiverAccounts = [
      createAccount('SLAVE-A', { accountType: 'slave' }),
      createAccount('SLAVE-B', { accountType: 'slave' }),
    ];
    const settings = [
      createSetting(1, 'MASTER-1', 'SLAVE-A'),
      createSetting(2, 'MASTER-1', 'SLAVE-B'),
    ];

    const { nodes, edges } = renderFlowData({ sourceAccounts, receiverAccounts, settings });

    expect(nodes.filter((node) => node.id.startsWith('source-'))).toHaveLength(1);
    expect(nodes.filter((node) => node.id.startsWith('receiver-'))).toHaveLength(2);
    expect(edges).toHaveLength(2);

    expect(getNodeData(nodes, 'source-MASTER-1').accountSettings).toHaveLength(2);
    expect(getNodeData(nodes, 'receiver-SLAVE-A').accountSettings).toHaveLength(1);
    expect(getNodeData(nodes, 'receiver-SLAVE-B').accountSettings).toHaveLength(1);
  });

  it('supports N:1 relationships where multiple masters feed the same slave', () => {
    const sourceAccounts = [createAccount('MASTER-1'), createAccount('MASTER-2')];
    const receiverAccounts = [createAccount('SLAVE-A', { accountType: 'slave' })];
    const settings = [
      createSetting(1, 'MASTER-1', 'SLAVE-A'),
      createSetting(2, 'MASTER-2', 'SLAVE-A'),
    ];

    const { nodes, edges } = renderFlowData({ sourceAccounts, receiverAccounts, settings });

    expect(nodes.filter((node) => node.id.startsWith('source-'))).toHaveLength(2);
    expect(nodes.filter((node) => node.id.startsWith('receiver-'))).toHaveLength(1);
    expect(edges).toHaveLength(2);

    expect(getNodeData(nodes, 'receiver-SLAVE-A').accountSettings).toHaveLength(2);
    expect(getNodeData(nodes, 'source-MASTER-1').accountSettings).toHaveLength(1);
    expect(getNodeData(nodes, 'source-MASTER-2').accountSettings).toHaveLength(1);
  });

  it('handles N:N mesh graphs without duplicating nodes', () => {
    const sourceAccounts = [
      createAccount('MASTER-1'),
      createAccount('MASTER-2'),
      createAccount('MASTER-3'),
    ];
    const receiverAccounts = [
      createAccount('SLAVE-A', { accountType: 'slave' }),
      createAccount('SLAVE-B', { accountType: 'slave' }),
      createAccount('SLAVE-C', { accountType: 'slave' }),
    ];
    const settings = [
      createSetting(1, 'MASTER-1', 'SLAVE-A'),
      createSetting(2, 'MASTER-1', 'SLAVE-B'),
      createSetting(3, 'MASTER-2', 'SLAVE-A'),
      createSetting(4, 'MASTER-2', 'SLAVE-C'),
      createSetting(5, 'MASTER-3', 'SLAVE-B'),
    ];

    const { nodes, edges } = renderFlowData({ sourceAccounts, receiverAccounts, settings });

    expect(nodes.filter((node) => node.id.startsWith('source-'))).toHaveLength(3);
    expect(nodes.filter((node) => node.id.startsWith('receiver-'))).toHaveLength(3);
    expect(new Set(nodes.map((node) => node.id)).size).toBe(nodes.length);
    expect(edges).toHaveLength(settings.length);

    expect(getNodeData(nodes, 'source-MASTER-1').accountSettings).toHaveLength(2);
    expect(getNodeData(nodes, 'source-MASTER-2').accountSettings).toHaveLength(2);
    expect(getNodeData(nodes, 'source-MASTER-3').accountSettings).toHaveLength(1);

    expect(getNodeData(nodes, 'receiver-SLAVE-A').accountSettings).toHaveLength(2);
    expect(getNodeData(nodes, 'receiver-SLAVE-B').accountSettings).toHaveLength(2);
    expect(getNodeData(nodes, 'receiver-SLAVE-C').accountSettings).toHaveLength(1);
  });
});
