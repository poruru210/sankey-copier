// Custom hook for VictoriaLogs settings API
// Provides CRUD operations for VictoriaLogs global settings

import { useState, useEffect, useCallback } from 'react';
import { useAtomValue } from 'jotai';
import { apiClientAtom } from '@/lib/atoms/site';

// VictoriaLogs settings type (matches relay-server model)
export interface VLogsGlobalSettings {
  enabled: boolean;
  endpoint: string;
  batch_size: number;
  flush_interval_secs: number;
}

// Default settings (matches relay-server defaults)
const DEFAULT_SETTINGS: VLogsGlobalSettings = {
  enabled: false,
  endpoint: '',
  batch_size: 100,
  flush_interval_secs: 5,
};

export function useVLogsSettings() {
  const apiClient = useAtomValue(apiClientAtom);
  const [settings, setSettings] = useState<VLogsGlobalSettings>(DEFAULT_SETTINGS);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Fetch settings
  const fetchSettings = useCallback(async () => {
    if (!apiClient) return;

    try {
      setLoading(true);
      const data = await apiClient.get<VLogsGlobalSettings>('/victoria-logs-settings');
      if (data) {
        setSettings(data);
      }
      setError(null);
    } catch (err) {
      console.error('Failed to fetch VictoriaLogs settings:', err);
      setError(err instanceof Error ? err.message : 'Failed to fetch settings');
    } finally {
      setLoading(false);
    }
  }, [apiClient]);

  // Save settings
  const saveSettings = useCallback(async (newSettings: VLogsGlobalSettings) => {
    if (!apiClient) return;

    try {
      setSaving(true);
      setError(null);
      await apiClient.put<void>('/victoria-logs-settings', newSettings);
      setSettings(newSettings);
      return true;
    } catch (err) {
      console.error('Failed to save VictoriaLogs settings:', err);
      setError(err instanceof Error ? err.message : 'Failed to save settings');
      return false;
    } finally {
      setSaving(false);
    }
  }, [apiClient]);

  // Initial load
  useEffect(() => {
    if (apiClient) {
      fetchSettings();
    }
  }, [apiClient, fetchSettings]);

  return {
    settings,
    loading,
    saving,
    error,
    saveSettings,
    refetch: fetchSettings,
  };
}
