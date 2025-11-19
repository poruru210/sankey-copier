'use client';

import { Globe } from 'lucide-react';
import { useAtom, useAtomValue } from 'jotai';
import { sitesAtom, selectedSiteIdAtom } from '@/lib/atoms/site';

// Site selector component for switching between server connections
// Site management (add/edit/delete) is now handled in the Sites page
export function SiteSelector() {
  const sites = useAtomValue(sitesAtom);
  const [selectedSiteId, setSelectedSiteId] = useAtom(selectedSiteIdAtom);

  // If sites are not loaded yet (empty array), we might want to show nothing or a loader
  // For now, we assume initial state is handled correctly by atoms
  if (sites.length === 0) {
    return null;
  }

  return (
    <div className="flex items-center gap-2">
      <Globe className="h-4 w-4 text-muted-foreground" />
      <select
        value={selectedSiteId}
        onChange={(e) => setSelectedSiteId(e.target.value)}
        className="h-9 rounded-md border border-input bg-background px-3 py-1 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
      >
        {sites.map((site) => (
          <option key={site.id} value={site.id}>
            {site.name}
          </option>
        ))}
      </select>
    </div>
  );
}
