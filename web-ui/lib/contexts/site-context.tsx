'use client';

import React, { createContext, useContext, ReactNode, useMemo } from 'react';
import { Site } from '@/lib/types/site';
import { useSites } from '@/lib/hooks/use-sites';
import { ApiClient } from '@/lib/api-client';

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

const SiteContext = createContext<SiteContextValue | undefined>(undefined);

export function SiteProvider({ children }: { children: ReactNode }) {
  const siteManagement = useSites();
  const { selectedSite } = siteManagement;

  // Memoize apiClient to prevent recreating on every render
  // This ensures stable reference for hooks that depend on it
  const apiClient = useMemo(() => new ApiClient(selectedSite), [selectedSite]);

  return (
    <SiteContext.Provider value={{ ...siteManagement, apiClient }}>
      {children}
    </SiteContext.Provider>
  );
}

/**
 * Hook to access site context
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
 */
export function useApiClient() {
  const { apiClient } = useSiteContext();
  return apiClient;
}
