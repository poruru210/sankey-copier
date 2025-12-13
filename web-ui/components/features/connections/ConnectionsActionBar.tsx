'use client';

import { useIntlayer } from 'next-intlayer';
import { Button } from '@/components/ui/button';
import { MasterAccountFilter } from '@/components/features/connections/MasterAccountFilter';
import { Plus, RefreshCw } from 'lucide-react';
import type { EaConnection, CopySettings } from '@/types';

interface ConnectionsActionBarProps {
  connections: EaConnection[];
  settings: CopySettings[];
  selectedMaster: string;
  onSelectMaster: (master: string) => void;
  onCreateClick: () => void;
}

export function ConnectionsActionBar({
  connections,
  settings,
  selectedMaster,
  onSelectMaster,
  onCreateClick,
}: ConnectionsActionBarProps) {
  const content = useIntlayer('connections-view');

  return (
    <div className="mb-4 flex flex-col gap-4 sm:flex-row sm:justify-between sm:items-center">
      <div className="flex items-center gap-4">
        <MasterAccountFilter
          connections={connections}
          settings={settings}
          selectedMaster={selectedMaster}
          onSelectMaster={onSelectMaster}
        />
      </div>
      <div className="flex gap-2">
        <Button variant="outline" size="sm" onClick={() => window.location.reload()}>
          <RefreshCw className="h-4 w-4 mr-2" />
          {content.refresh}
        </Button>
        <Button
          size="sm"
          onClick={onCreateClick}
          data-testid="create-connection-button"
        >
          <Plus className="h-4 w-4 mr-2" />
          {content.createNewLink}
        </Button>
      </div>
    </div>
  );
}
