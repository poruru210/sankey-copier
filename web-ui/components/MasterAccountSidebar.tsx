'use client';

import { useMemo } from 'react';
import { useIntlayer } from 'next-intlayer';
import { cn } from '@/lib/utils';
import type { CopySettings, EaConnection } from '@/types';

export interface MasterAccountInfo {
  id: string;
  name: string;
  status: 'online' | 'offline';
  connectionCount: number;
  isOnline: boolean;
}

interface MasterAccountSidebarProps {
  connections: EaConnection[];
  settings: CopySettings[];
  selectedMaster: string | 'all';
  onSelectMaster: (masterId: string | 'all') => void;
  className?: string;
}

export function MasterAccountSidebar({
  connections,
  settings,
  selectedMaster,
  onSelectMaster,
  className,
}: MasterAccountSidebarProps) {
  const content = useIntlayer('master-account-sidebar');

  // Aggregate master accounts from connections
  const masterAccounts = useMemo(() => {
    const masters = connections.filter((conn) => conn.ea_type === 'Master');

    return masters.map((master): MasterAccountInfo => {
      const connectionCount = settings.filter(
        (s) => s.master_account === master.account_id && s.enabled
      ).length;

      const isOnline = master.status === 'Online';

      return {
        id: master.account_id,
        name: master.account_name || master.account_id,
        status: isOnline ? 'online' : 'offline',
        connectionCount,
        isOnline,
      };
    });
  }, [connections, settings]);

  // Count total connections for "All Accounts"
  const totalConnections = useMemo(() => {
    return settings.filter((s) => s.enabled).length;
  }, [settings]);

  return (
    <nav
      className={cn(
        'flex flex-col bg-card border-r border-border h-full',
        className
      )}
      aria-label="Master account filter"
    >
      {/* Header */}
      <div className="px-4 py-3 border-b border-border">
        <h3 className="text-sm font-semibold text-foreground">
          {content.filterAccounts}
        </h3>
      </div>

      {/* Account List */}
      <div className="flex-1 overflow-y-auto">
        {/* All Accounts Option */}
        <button
          onClick={() => onSelectMaster('all')}
          className={cn(
            'w-full px-4 py-3 text-left transition-colors',
            'hover:bg-accent hover:text-accent-foreground',
            'flex items-center justify-between gap-2',
            selectedMaster === 'all' && 'bg-accent text-accent-foreground font-medium'
          )}
          role="radio"
          aria-checked={selectedMaster === 'all'}
          aria-label={`All accounts, ${totalConnections} connections`}
        >
          <div className="flex items-center gap-2 flex-1 min-w-0">
            <div className="w-2 h-2 rounded-full bg-muted-foreground flex-shrink-0" />
            <span className="text-sm truncate">{content.allAccounts}</span>
          </div>
          <span className="text-xs text-muted-foreground bg-muted px-2 py-0.5 rounded-full flex-shrink-0">
            {totalConnections}
          </span>
        </button>

        {/* Divider */}
        <div className="border-b border-border my-2" />

        {/* Master Accounts */}
        {masterAccounts.length === 0 ? (
          <div className="px-4 py-6 text-center text-sm text-muted-foreground">
            {content.noMasterAccounts}
          </div>
        ) : (
          <div className="space-y-1">
            {masterAccounts.map((master) => (
              <button
                key={master.id}
                onClick={() => onSelectMaster(master.id)}
                className={cn(
                  'w-full px-4 py-3 text-left transition-colors',
                  'hover:bg-accent hover:text-accent-foreground',
                  'flex items-center gap-2',
                  selectedMaster === master.id &&
                    'bg-accent text-accent-foreground font-medium border-l-2 border-primary'
                )}
                role="radio"
                aria-checked={selectedMaster === master.id}
                aria-label={`${master.name}, ${master.status}, ${master.connectionCount} connections`}
              >
                {/* Status Indicator */}
                <div
                  className={cn(
                    'w-2 h-2 rounded-full flex-shrink-0',
                    master.isOnline ? 'bg-green-500' : 'bg-gray-400'
                  )}
                  aria-hidden="true"
                />

                {/* Account Info */}
                <div className="flex-1 min-w-0">
                  <div className="text-sm truncate">{master.name}</div>
                  <div className="text-xs text-muted-foreground">
                    {master.isOnline ? content.online : content.offline} • {master.connectionCount}{' '}
                    {master.connectionCount !== 1 ? content.links : content.link}
                  </div>
                </div>
              </button>
            ))}
          </div>
        )}
      </div>
    </nav>
  );
}
