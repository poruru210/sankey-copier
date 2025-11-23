/**
 * Custom hook for managing Master EA configuration
 *
 * Provides methods to fetch, update, and delete Master EA configuration
 * for symbol prefix/suffix settings.
 */

import { useState, useCallback } from 'react';
import { useAtomValue } from 'jotai';
import type { MasterConfig, UpdateMasterConfigRequest } from '@/types';
import { apiClientAtom } from '@/lib/atoms/site';

export function useMasterConfig() {
  const apiClient = useAtomValue(apiClientAtom);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  /**
   * Fetch Master configuration for a specific account
   */
  const getMasterConfig = useCallback(
    async (accountId: string): Promise<MasterConfig | null> => {
      if (!apiClient) {
        throw new Error('API client not initialized');
      }

      setLoading(true);
      setError(null);

      try {
        const config = await apiClient.get<MasterConfig>(
          `/masters/${accountId}/config`
        );
        return config;
      } catch (err) {
        // 404 is expected when config doesn't exist yet
        if (err instanceof Error && err.message.includes('404')) {
          return null;
        }
        const errorMsg =
          err instanceof Error ? err.message : 'Failed to fetch Master config';
        setError(errorMsg);
        throw err;
      } finally {
        setLoading(false);
      }
    },
    [apiClient]
  );

  /**
   * Update Master configuration (creates if doesn't exist)
   */
  const updateMasterConfig = useCallback(
    async (
      accountId: string,
      configData: UpdateMasterConfigRequest
    ): Promise<void> => {
      if (!apiClient) {
        throw new Error('API client not initialized');
      }

      setLoading(true);
      setError(null);

      try {
        await apiClient.put<void>(`/masters/${accountId}/config`, configData);
      } catch (err) {
        const errorMsg =
          err instanceof Error ? err.message : 'Failed to update Master config';
        setError(errorMsg);
        throw err;
      } finally {
        setLoading(false);
      }
    },
    [apiClient]
  );

  /**
   * Delete Master configuration
   */
  const deleteMasterConfig = useCallback(
    async (accountId: string): Promise<void> => {
      if (!apiClient) {
        throw new Error('API client not initialized');
      }

      setLoading(true);
      setError(null);

      try {
        await apiClient.delete<void>(`/masters/${accountId}/config`);
      } catch (err) {
        const errorMsg =
          err instanceof Error ? err.message : 'Failed to delete Master config';
        setError(errorMsg);
        throw err;
      } finally {
        setLoading(false);
      }
    },
    [apiClient]
  );

  return {
    getMasterConfig,
    updateMasterConfig,
    deleteMasterConfig,
    loading,
    error,
  };
}
