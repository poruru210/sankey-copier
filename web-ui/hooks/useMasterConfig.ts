/**
 * Custom hook for managing Master EA configuration
 *
 * Provides methods to fetch, update, and delete Master EA configuration
 * for symbol prefix/suffix settings.
 *
 * Note: This hook uses the TradeGroups API (/api/trade-groups/:id)
 */

import { useState, useCallback } from 'react';
import { useAtomValue } from 'jotai';
import type { MasterConfig, UpdateMasterConfigRequest, TradeGroup } from '@/types';
import { apiClientAtom } from '@/lib/atoms/site';

export function useMasterConfig() {
  const apiClient = useAtomValue(apiClientAtom);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  /**
   * Fetch Master configuration for a specific account
   * Uses the TradeGroups API to get master_settings
   */
  const getMasterConfig = useCallback(
    async (accountId: string): Promise<MasterConfig | null> => {
      if (!apiClient) {
        throw new Error('API client not initialized');
      }

      setLoading(true);
      setError(null);

      try {
        const tradeGroup = await apiClient.get<TradeGroup>(
          `/trade-groups/${encodeURIComponent(accountId)}`
        );
        // Convert TradeGroup.master_settings to MasterConfig format
        return {
          account_id: accountId,
          symbol_prefix: tradeGroup.master_settings.symbol_prefix,
          symbol_suffix: tradeGroup.master_settings.symbol_suffix,
          config_version: tradeGroup.master_settings.config_version,
          timestamp: tradeGroup.updated_at,
        };
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
        // Use TradeGroups API to update master_settings
        // First fetch current config to get the config_version
        const currentConfig = await getMasterConfig(accountId);
        const currentVersion = currentConfig?.config_version || 0;

        await apiClient.updateTradeGroupSettings(accountId, {
          symbol_prefix: configData.symbol_prefix,
          symbol_suffix: configData.symbol_suffix,
          config_version: currentVersion,
        });
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
        // TradeGroups API doesn't have a DELETE endpoint for settings
        // Instead, we update with null values to reset the configuration
        // First fetch current config to get the version
        const currentConfig = await getMasterConfig(accountId);
        if (!currentConfig) {
          throw new Error('Configuration not found');
        }

        await apiClient.updateTradeGroupSettings(accountId, {
          symbol_prefix: null,
          symbol_suffix: null,
          config_version: currentConfig.config_version,
        });
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
