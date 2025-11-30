// web-ui/hooks/useTradeGroups.ts
//
// Custom hook for managing TradeGroups (Master account settings) state and API calls.
// Provides loading states, error handling, and data fetching for TradeGroups list.

import { useState, useCallback } from 'react';
import type { ApiClient } from '@/lib/api-client';
import type { TradeGroup } from '@/types';

export function useTradeGroups(apiClient: ApiClient) {
  const [tradeGroups, setTradeGroups] = useState<TradeGroup[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  /**
   * Fetch all TradeGroups from the API
   */
  const fetchTradeGroups = useCallback(async () => {
    setLoading(true);
    setError(null);

    try {
      const data = await apiClient.listTradeGroups();
      setTradeGroups(data);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to fetch trade groups';
      setError(errorMessage);
      console.error('Error fetching trade groups:', err);
    } finally {
      setLoading(false);
    }
  }, [apiClient]);

  /**
   * Toggle Master enabled state
   */
  const toggleMaster = useCallback(async (masterAccount: string, enabled: boolean) => {
    try {
      const updatedTradeGroup = await apiClient.toggleMaster(masterAccount, enabled);
      setTradeGroups((prev) =>
        prev.map((tg) =>
          tg.id === masterAccount ? updatedTradeGroup : tg
        )
      );
      return updatedTradeGroup;
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to toggle master';
      console.error('Error toggling master:', err);
      throw new Error(errorMessage);
    }
  }, [apiClient]);

  /**
   * Get enabled state for a specific Master account
   */
  const isMasterEnabled = useCallback((masterAccount: string): boolean => {
    const tradeGroup = tradeGroups.find((tg) => tg.id === masterAccount);
    return tradeGroup?.master_settings.enabled ?? true;
  }, [tradeGroups]);

  return {
    tradeGroups,
    loading,
    error,
    fetchTradeGroups,
    toggleMaster,
    isMasterEnabled,
  };
}
