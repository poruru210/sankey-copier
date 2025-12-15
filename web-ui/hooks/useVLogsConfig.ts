// Custom hook for VictoriaLogs configuration API
// Fetches config.toml settings (read-only) and provides toggle for enabled state
// Replaces useVLogsSettings for the new architecture where config comes from config.toml

import { useState, useEffect, useCallback } from 'react';
import { useAtomValue } from 'jotai';
import { apiClientAtom } from '@/lib/atoms/site';

// Config info from config.toml (read-only)
export interface VLogsConfigInfo {
  host: string;
  batch_size: number;
  flush_interval_secs: number;
  source: string;
  log_level: string;
}

// Full config response from GET /api/victoria-logs-config
export interface VLogsConfigResponse {
  configured: boolean;
  config: VLogsConfigInfo | null;
  enabled: boolean;
}

// Update request payload
export interface VLogsUpdateRequest {
  enabled?: boolean;
  log_level?: string;
}

// Default response when not loaded or error
const DEFAULT_CONFIG: VLogsConfigResponse = {
  configured: false,
  config: null,
  enabled: false,
};

export function useVLogsConfig() {
  const apiClient = useAtomValue(apiClientAtom);
  const [configData, setConfigData] = useState<VLogsConfigResponse>(DEFAULT_CONFIG);
  const [loading, setLoading] = useState(true);
  const [updating, setUpdating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Fetch config from server
  const fetchConfig = useCallback(async () => {
    if (!apiClient) return;

    try {
      setLoading(true);
      const data = await apiClient.get<VLogsConfigResponse>('/victoria-logs-config');
      if (data) {
        setConfigData(data);
      }
      setError(null);
    } catch (err) {
      console.error('Failed to fetch VictoriaLogs config:', err);
      setError(err instanceof Error ? err.message : 'Failed to fetch config');
    } finally {
      setLoading(false);
    }
  }, [apiClient]);

  // Update settings (enabled state, log level)
  const updateSettings = useCallback(async (updates: VLogsUpdateRequest) => {
    if (!apiClient) return false;

    try {
      setUpdating(true);
      setError(null);
      await apiClient.put<void>('/victoria-logs-settings', updates);
      
      // Update local state optimistically or re-fetch?
      // Since log_level is inside 'config' which mimics config.toml, and 'enabled' is top level..
      // We can update local state partly. Best is to refetch or manually patch.
      setConfigData((prev) => {
        const next = { ...prev };
        if (updates.enabled !== undefined) next.enabled = updates.enabled;
        if (updates.log_level !== undefined && next.config) {
            next.config = { ...next.config, log_level: updates.log_level };
        }
        return next;
      });
      return true;
    } catch (err) {
      console.error('Failed to update VictoriaLogs settings:', err);
      setError(err instanceof Error ? err.message : 'Failed to update settings');
      return false;
    } finally {
      setUpdating(false);
    }
  }, [apiClient]);

  // Initial load
  useEffect(() => {
    if (apiClient) {
      fetchConfig();
    }
  }, [apiClient, fetchConfig]);

  return {
    // Config data
    configured: configData.configured,
    config: configData.config,
    enabled: configData.enabled,
    // State
    loading,
    updating, // renamed from toggling
    error,
    // Actions
    updateSettings, // renamed from toggleEnabled
    refetch: fetchConfig,
  };
}
