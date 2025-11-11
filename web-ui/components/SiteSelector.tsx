'use client';

import { useState } from 'react';
import { Server, Settings } from 'lucide-react';
import { useSiteContext } from '@/lib/contexts/site-context';
import { Button } from '@/components/ui/button';
import { SiteManagementDialog } from '@/components/SiteManagementDialog';

export function SiteSelector() {
  const { sites, selectedSiteId, selectSite, isLoaded } = useSiteContext();
  const [dialogOpen, setDialogOpen] = useState(false);

  if (!isLoaded) {
    return null;
  }

  return (
    <>
      <div className="flex items-center gap-2">
        <Server className="h-4 w-4 text-muted-foreground" />
        <select
          value={selectedSiteId}
          onChange={(e) => selectSite(e.target.value)}
          className="h-9 rounded-md border border-input bg-background px-3 py-1 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
        >
          {sites.map((site) => (
            <option key={site.id} value={site.id}>
              {site.name} ({site.siteUrl})
            </option>
          ))}
        </select>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => setDialogOpen(true)}
          className="h-9 px-2"
          title="サイトを管理"
        >
          <Settings className="h-4 w-4" />
        </Button>
      </div>
      <SiteManagementDialog open={dialogOpen} onOpenChange={setDialogOpen} />
    </>
  );
}
