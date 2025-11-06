import { useMemo, useState, useCallback } from 'react';
import type { CopySettings, EaConnection, AccountInfo } from '@/types';

interface UseMasterFilterOptions {
  connections: EaConnection[];
  settings: CopySettings[];
  sourceAccounts: AccountInfo[];
  receiverAccounts: AccountInfo[];
}

interface UseMasterFilterReturn {
  selectedMaster: string | 'all';
  setSelectedMaster: (master: string | 'all') => void;
  visibleSourceAccounts: AccountInfo[];
  visibleReceiverAccounts: AccountInfo[];
  selectedMasterName: string | null;
}

/**
 * Custom hook for managing master account filtering
 */
export function useMasterFilter({
  connections,
  settings,
  sourceAccounts,
  receiverAccounts,
}: UseMasterFilterOptions): UseMasterFilterReturn {
  const [selectedMaster, setSelectedMaster] = useState<string | 'all'>('all');

  // Filter source accounts based on selected master
  const visibleSourceAccounts = useMemo(() => {
    if (selectedMaster === 'all') return sourceAccounts;
    return sourceAccounts.filter((acc) => acc.id === selectedMaster);
  }, [selectedMaster, sourceAccounts]);

  // Filter receiver accounts based on selected master
  const visibleReceiverAccounts = useMemo(() => {
    if (selectedMaster === 'all') return receiverAccounts;
    return receiverAccounts.filter((acc) =>
      settings.some(
        (s) =>
          s.master_account === selectedMaster &&
          s.slave_account === acc.id &&
          s.enabled
      )
    );
  }, [selectedMaster, receiverAccounts, settings]);

  // Get selected master account name
  const selectedMasterName = useMemo(() => {
    if (selectedMaster === 'all') return null;
    const masterConnection = connections.find(
      (conn) => conn.account_id === selectedMaster && conn.ea_type === 'Master'
    );
    return masterConnection?.account_name || selectedMaster;
  }, [selectedMaster, connections]);

  return {
    selectedMaster,
    setSelectedMaster,
    visibleSourceAccounts,
    visibleReceiverAccounts,
    selectedMasterName,
  };
}
