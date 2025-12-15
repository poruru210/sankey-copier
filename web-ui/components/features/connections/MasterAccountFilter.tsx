'use client';

import { useMemo } from 'react';
import { useIntlayer } from 'next-intlayer';
import { cn } from '@/lib/utils';
import { ChevronDown } from 'lucide-react';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Button } from '@/components/ui/button';
import type { CopySettings, EaConnection } from '@/types';

// Master account info interface
export interface MasterAccountInfo {
  id: string;
  name: string;
  brokerName: string;
  accountNumber: string;
  status: 'online' | 'offline';
  connectionCount: number;
  isOnline: boolean;
}

interface MasterAccountFilterProps {
  connections: EaConnection[];
  settings: CopySettings[];
  selectedMaster: string | 'all';
  onSelectMaster: (masterId: string | 'all') => void;
  className?: string;
}

// Master account filter component as dropdown
// Displays selected account and allows filtering by master account
export function MasterAccountFilter({
  connections,
  settings,
  selectedMaster,
  onSelectMaster,
  className,
}: MasterAccountFilterProps) {
  const content = useIntlayer('master-account-sidebar');

  // Aggregate master accounts from connections
  const masterAccounts = useMemo(() => {
    const masters = connections.filter((conn) => conn.ea_type === 'Master');

    return masters.map((master): MasterAccountInfo => {
      const connectionCount = settings.filter(
        (s) =>
          s.master_account === master.account_id &&
          (s.enabled_flag ?? (s.status !== 0))
      ).length;

      const isOnline = master.status === 'Online';

      // Format broker name: replace underscores with spaces
      const brokerName = master.broker.replace(/_/g, ' ');

      return {
        id: master.account_id,
        name: `${master.broker} #${master.account_number}`,
        brokerName,
        accountNumber: master.account_number.toString(),
        status: isOnline ? 'online' : 'offline',
        connectionCount,
        isOnline,
      };
    });
  }, [connections, settings]);

  // Count total connections for "All Accounts"
  const totalConnections = useMemo(() => {
    return settings.filter((s) => s.enabled_flag ?? (s.status !== 0)).length;
  }, [settings]);

  // Get current selection display text
  const currentSelection = useMemo(() => {
    if (selectedMaster === 'all') {
      return content.allAccounts;
    }
    const selected = masterAccounts.find((m) => m.id === selectedMaster);
    return selected ? selected.brokerName : content.allAccounts;
  }, [selectedMaster, masterAccounts, content]);

  // Get current selection count
  const currentCount = useMemo(() => {
    if (selectedMaster === 'all') {
      return totalConnections;
    }
    const selected = masterAccounts.find((m) => m.id === selectedMaster);
    return selected ? selected.connectionCount : 0;
  }, [selectedMaster, masterAccounts, totalConnections]);

  return (
    <div className={cn('flex items-center gap-3', className)}>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button
            variant="outline"
            className="flex items-center gap-2 min-w-[200px] justify-between"
            data-testid="master-filter-trigger"
          >
            <div className="flex items-center gap-2 flex-1 min-w-0">
              <span className="text-sm truncate">{currentSelection}</span>
            </div>
            <div className="flex items-center gap-2 flex-shrink-0">
              <span
                className="text-xs text-muted-foreground bg-muted px-2 py-0.5 rounded-full"
                data-testid="master-filter-count"
              >
                {currentCount}
              </span>
              <ChevronDown className="h-4 w-4 opacity-50" />
            </div>
          </Button>
        </DropdownMenuTrigger>

        <DropdownMenuContent
          align="start"
          className="w-[280px]"
          data-testid="master-filter-menu"
        >
          <DropdownMenuLabel>{content.filterAccounts}</DropdownMenuLabel>
          <DropdownMenuSeparator />

          {/* All Accounts Option */}
          <DropdownMenuItem
            onClick={() => onSelectMaster('all')}
            className={cn(
              'flex items-center justify-between gap-2',
              selectedMaster === 'all' && 'bg-accent font-medium'
            )}
            data-testid="master-filter-option-all"
          >
            <div className="flex items-center gap-2 flex-1 min-w-0">
              <div className="w-2 h-2 rounded-full bg-muted-foreground flex-shrink-0" />
              <span className="text-sm truncate">{content.allAccounts}</span>
            </div>
            <span className="text-xs text-muted-foreground bg-muted px-2 py-0.5 rounded-full flex-shrink-0">
              {totalConnections}
            </span>
          </DropdownMenuItem>

          <DropdownMenuSeparator />

          {/* Master Accounts */}
          {masterAccounts.length === 0 ? (
            <div className="px-2 py-6 text-center text-sm text-muted-foreground">
              {content.noMasterAccounts}
            </div>
          ) : (
            masterAccounts.map((master) => (
              <DropdownMenuItem
                key={master.id}
                onClick={() => onSelectMaster(master.id)}
                className={cn(
                  'flex items-center gap-2',
                  selectedMaster === master.id && 'bg-accent font-medium'
                )}
                data-testid={`master-filter-option-${master.id}`}
              >
                {/* Status Indicator */}
                <div
                  className={cn(
                    'w-2 h-2 rounded-full flex-shrink-0',
                    master.isOnline ? 'bg-green-500' : 'bg-gray-400'
                  )}
                />

                {/* Account Info */}
                <div className="flex-1 min-w-0">
                  <div className="text-sm font-medium truncate">{master.brokerName}</div>
                  <div className="text-xs text-muted-foreground truncate">
                    #{master.accountNumber} • {master.isOnline ? content.online : content.offline} •{' '}
                    {master.connectionCount} {master.connectionCount !== 1 ? content.links : content.link}
                  </div>
                </div>
              </DropdownMenuItem>
            ))
          )}
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
}
