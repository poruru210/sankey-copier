'use client';

import React, { createContext, useContext, ReactNode } from 'react';
import { useAtom, useAtomValue, useSetAtom } from 'jotai';
import { Site } from '@/lib/types/site';
import { ApiClient } from '@/lib/api-client';
import {
  sitesAtom,
  selectedSiteIdAtom,
  selectedSiteAtom,
  apiClientAtom,
} from '@/lib/atoms/site';

interface SiteContextValue {
  sites: Site[];
  selectedSite: Site;
  selectedSiteId: string;
  isLoaded: boolean;
  apiClient: ApiClient;
  addSite: (name: string, siteUrl: string) => Site;
  updateSite: (id: string, updates: Partial<Omit<Site, 'id'>>) => void;
  deleteSite: (id: string) => void;
  selectSite: (id: string) => void;
}

// Context is kept for backward compatibility if needed, but we'll try to use atoms directly
const SiteContext = createContext<SiteContextValue | undefined>(undefined);

export function SiteProvider({ children }: { children: ReactNode }) {
  const [sites, setSites] = useAtom(sitesAtom);
  const [selectedSiteId, setSelectedSiteId] = useAtom(selectedSiteIdAtom);
  const selectedSite = useAtomValue(selectedSiteAtom);
  const apiClient = useAtomValue(apiClientAtom);

  // Helper functions to match previous interface
  const addSite = (name: string, siteUrl: string) => {
    const newSite: Site = {
      id: `site-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`,
      name,
      siteUrl,
    };
    setSites((prev) => [...prev, newSite]);
    return newSite;
  };

  const updateSite = (id: string, updates: Partial<Omit<Site, 'id'>>) => {
    setSites((prev) =>
      prev.map((site) => (site.id === id ? { ...site, ...updates } : site))
    );
  };

  const deleteSite = (id: string) => {
    setSites((prev) => {
      const filtered = prev.filter((site) => site.id !== id);
      if (id === selectedSiteId && filtered.length > 0) {
        setSelectedSiteId(filtered[0].id);
      }
      return filtered;
    });
  };

  const selectSite = (id: string) => {
    setSelectedSiteId(id);
  };

  const value = {
    sites,
    selectedSite,
    selectedSiteId,
    isLoaded: true, // atomWithStorage handles loading internally (mostly)
    apiClient,
    addSite,
    updateSite,
    deleteSite,
    selectSite,
  };

  return <SiteContext.Provider value={value}>{children}</SiteContext.Provider>;
}

/**
 * Hook to access site context
 * @deprecated Use atoms directly instead
 */
export function useSiteContext() {
  const context = useContext(SiteContext);
  if (!context) {
    throw new Error('useSiteContext must be used within a SiteProvider');
  }
  return context;
}

/**
 * Hook to get the API client for the selected site
 * @deprecated Use apiClientAtom directly instead
 */
export function useApiClient() {
  return useAtomValue(apiClientAtom);
}
