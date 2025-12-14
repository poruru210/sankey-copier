/**
 * Master Configuration Section
 *
 * Shows Master EA configuration settings and provides access to configure
 * symbol prefix/suffix for the Master account.
 */

'use client';

import { useState } from 'react';
import type { EaConnection } from '@/types';
import { Button } from '@/components/ui/button';
import { MasterConfigDialog } from '@/components/features/settings/MasterConfigDialog';

interface MasterConfigSectionProps {
  connection?: EaConnection;
}

/**
 * Master configuration section with button to open config dialog
 * Only shown for Master EA accounts
 */
export function MasterConfigSection({ connection }: MasterConfigSectionProps) {
  const [dialogOpen, setDialogOpen] = useState(false);

  // Only show for Master accounts
  if (!connection || connection.ea_type !== 'Master') {
    return null;
  }

  return (
    <div className="space-y-1.5">
      <div className="flex items-start gap-2 mb-2">
        <span className="text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wide">
          Master Settings
        </span>
      </div>
      <div className="h-px bg-gray-300 dark:bg-gray-600 -mt-1 mb-2"></div>

      <div className="text-xs">
        <p className="text-gray-600 dark:text-gray-400 mb-2">
          Configure global symbol transformation for all Slaves in this
          Master&apos;s trade groups.
        </p>
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={() => setDialogOpen(true)}
          className="w-full pointer-events-auto"
        >
          Configure Master Settings
        </Button>
      </div>

      {/* Master Config Dialog */}
      <MasterConfigDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        masterAccount={connection}
      />
    </div>
  );
}
