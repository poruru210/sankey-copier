'use client';

import { useState, useEffect, useCallback } from 'react';
import { Site, DEFAULT_SITE, STORAGE_KEYS } from '@/lib/types/site';

/**
 * Generate a unique ID for a site
 */
function generateSiteId(): string {
  return `site-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`;
}

/**
 * Hook for managing sites
 */
export function useSites() {
  const [sites, setSites] = useState<Site[]>([]);
  const [selectedSiteId, setSelectedSiteId] = useState<string>(DEFAULT_SITE.id);
  const [isLoaded, setIsLoaded] = useState(false);

  // Load sites from localStorage on mount
  useEffect(() => {
    if (typeof window === 'undefined') return;

    const storedSites = localStorage.getItem(STORAGE_KEYS.SITES);
    const storedSelectedId = localStorage.getItem(STORAGE_KEYS.SELECTED_SITE_ID);

    if (storedSites) {
      try {
        const parsed = JSON.parse(storedSites) as Site[];
        setSites(parsed);
      } catch (error) {
        console.error('Failed to parse stored sites:', error);
        setSites([DEFAULT_SITE]);
      }
    } else {
      // Initialize with default site
      setSites([DEFAULT_SITE]);
    }

    if (storedSelectedId) {
      setSelectedSiteId(storedSelectedId);
    }

    setIsLoaded(true);
  }, []);

  // Save sites to localStorage whenever they change
  useEffect(() => {
    if (!isLoaded || typeof window === 'undefined') return;
    localStorage.setItem(STORAGE_KEYS.SITES, JSON.stringify(sites));
  }, [sites, isLoaded]);

  // Save selected site ID to localStorage whenever it changes
  useEffect(() => {
    if (!isLoaded || typeof window === 'undefined') return;
    localStorage.setItem(STORAGE_KEYS.SELECTED_SITE_ID, selectedSiteId);
  }, [selectedSiteId, isLoaded]);

  /**
   * Get the currently selected site
   */
  const selectedSite = sites.find(site => site.id === selectedSiteId) || sites[0] || DEFAULT_SITE;

  /**
   * Add a new site
   */
  const addSite = useCallback((name: string, siteUrl: string) => {
    const newSite: Site = {
      id: generateSiteId(),
      name,
      siteUrl,
    };
    setSites(prev => [...prev, newSite]);
    return newSite;
  }, []);

  /**
   * Update an existing site
   */
  const updateSite = useCallback((id: string, updates: Partial<Omit<Site, 'id'>>) => {
    setSites(prev =>
      prev.map(site =>
        site.id === id ? { ...site, ...updates } : site
      )
    );
  }, []);

  /**
   * Delete a site
   */
  const deleteSite = useCallback((id: string) => {
    setSites(prev => {
      const filtered = prev.filter(site => site.id !== id);
      // If we deleted the selected site, select the first remaining site
      if (id === selectedSiteId && filtered.length > 0) {
        setSelectedSiteId(filtered[0].id);
      }
      return filtered;
    });
  }, [selectedSiteId]);

  /**
   * Select a site
   */
  const selectSite = useCallback((id: string) => {
    setSelectedSiteId(id);
  }, []);

  return {
    sites,
    selectedSite,
    selectedSiteId,
    isLoaded,
    addSite,
    updateSite,
    deleteSite,
    selectSite,
  };
}
