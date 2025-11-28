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
}

// Full config response from GET /api/victoria-logs-config
export interface VLogsConfigResponse {
  configured: boolean;
  config: VLogsConfigInfo | null;
  enabled: boolean;
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
  const [toggling, setToggling] = useState(false);
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

  // Toggle enabled state (only operation allowed from web-ui)
  const toggleEnabled = useCallback(async (enabled: boolean) => {
    if (!apiClient) return false;

    try {
      setToggling(true);
      setError(null);
      await apiClient.put<void>('/victoria-logs-settings', { enabled });
      // Update local state
      setConfigData((prev) => ({ ...prev, enabled }));
      return true;
    } catch (err) {
      console.error('Failed to toggle VictoriaLogs enabled state:', err);
      setError(err instanceof Error ? err.message : 'Failed to toggle enabled state');
      return false;
    } finally {
      setToggling(false);
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
    toggling,
    error,
    // Actions
    toggleEnabled,
    refetch: fetchConfig,
  };
}
