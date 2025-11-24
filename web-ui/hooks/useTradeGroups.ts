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

  return {
    tradeGroups,
    loading,
    error,
    fetchTradeGroups,
  };
}
