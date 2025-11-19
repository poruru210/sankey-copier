import { atom } from 'jotai';
import { atomWithStorage } from 'jotai/utils';
import { Site, DEFAULT_SITE, STORAGE_KEYS } from '@/lib/types/site';
import { ApiClient } from '@/lib/api-client';

// Persistent atoms
export const sitesAtom = atomWithStorage<Site[]>(
    STORAGE_KEYS.SITES,
    [DEFAULT_SITE]
);

export const selectedSiteIdAtom = atomWithStorage<string>(
    STORAGE_KEYS.SELECTED_SITE_ID,
    DEFAULT_SITE.id
);

// Derived atoms
export const selectedSiteAtom = atom(
    (get) => {
        const sites = get(sitesAtom);
        const selectedId = get(selectedSiteIdAtom);
        return sites.find((site) => site.id === selectedId) || sites[0] || DEFAULT_SITE;
    }
);

export const apiClientAtom = atom((get) => {
    const selectedSite = get(selectedSiteAtom);
    return new ApiClient(selectedSite);
});
