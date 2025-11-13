import { useState, useEffect, useCallback, useOptimistic } from 'react';
import type { CopySettings, EaConnection, CreateSettingsRequest } from '@/types';
import { useApiClient, useSiteContext } from '@/lib/contexts/site-context';

type SettingsAction =
  | { type: 'add'; data: CopySettings }
  | { type: 'update'; id: number; data: CopySettings }
  | { type: 'delete'; id: number }
  | { type: 'toggle'; id: number };

export function useSankeyCopier() {
  const apiClient = useApiClient();
  const { selectedSite } = useSiteContext();
  const [settings, setSettings] = useState<CopySettings[]>([]);
  const [optimisticSettings, addOptimisticSettings] = useOptimistic(
    settings,
    (state, action: SettingsAction) => {
      switch (action.type) {
        case 'add':
          return [...state, action.data];
        case 'update':
          return state.map(s => s.id === action.id ? action.data : s);
        case 'delete':
          return state.filter(s => s.id !== action.id);
        case 'toggle':
          return state.map(s => s.id === action.id ? { ...s, enabled: !s.enabled } : s);
        default:
          return state;
      }
    }
  );
  const [connections, setConnections] = useState<EaConnection[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [wsMessages, setWsMessages] = useState<string[]>([]);

  // Fetch connections
  const fetchConnections = useCallback(async () => {
    try {
      // Rust API returns Vec<EaConnection> directly (not wrapped)
      const connections = await apiClient.get<EaConnection[]>('/connections');
      setConnections(connections);
    } catch (err) {
      if (err instanceof TypeError && err.message.includes('fetch')) {
        console.error('Cannot connect to server - is rust-server running?');
      } else {
        console.error('Failed to fetch connections:', err);
      }
    }
  }, [apiClient]);

  // Fetch settings
  const fetchSettings = useCallback(async () => {
    try {
      setLoading(true);
      // Rust API returns Vec<CopySettings> directly (not wrapped)
      const settings = await apiClient.get<CopySettings[]>('/settings');
      setSettings(settings);
      setError(null);
    } catch (err) {
      if (err instanceof TypeError && (err.message.includes('fetch') || err.message.includes('Failed to fetch'))) {
        setError('Cannot connect to server. Please check if Rust Server is running.');
      } else if (err instanceof Error && err.message.includes('JSON')) {
        setError('Invalid server response. Rust Server may not be running correctly.');
      } else if (err instanceof Error && (err.message.includes('500') || err.message.includes('502') || err.message.includes('503'))) {
        setError('Cannot connect to server. Please check if Rust Server is running.');
      } else {
        setError(err instanceof Error ? `Communication error: ${err.message}` : 'Unknown error');
      }
      console.error('Failed to fetch settings:', err);
    } finally {
      setLoading(false);
    }
  }, [apiClient]);

  // WebSocket connection
  useEffect(() => {
    // Extract host from siteUrl (e.g., "http://localhost:3000" -> "localhost:3000")
    const siteHost = selectedSite.siteUrl.replace(/^https?:\/\//, '');
    const ws = new WebSocket(`ws://${siteHost}/ws`);
    let isCleanup = false;

    ws.onopen = () => {
      if (!isCleanup) {
        console.log('WebSocket connected');
      }
    };

    ws.onmessage = (event) => {
      if (isCleanup) return;
      const message = event.data;
      console.log('WS message:', message);
      setWsMessages((prev) => [message, ...prev].slice(0, 20));

      if (message.startsWith('settings_')) {
        fetchSettings();
      }
    };

    ws.onerror = (error) => {
      if (!isCleanup && ws.readyState !== WebSocket.CLOSING && ws.readyState !== WebSocket.CLOSED) {
        console.error('WebSocket error:', error);
      }
    };

    ws.onclose = () => {
      if (!isCleanup) {
        console.log('WebSocket disconnected');
      }
    };

    return () => {
      isCleanup = true;
      if (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING) {
        ws.close();
      }
    };
  }, [selectedSite.siteUrl, fetchSettings]);

  // Initial load and periodic connection refresh
  useEffect(() => {
    fetchSettings();
    fetchConnections();
    const interval = setInterval(fetchConnections, 5000);
    return () => clearInterval(interval);
  }, [fetchSettings, fetchConnections]);

  // Toggle enabled status
  const toggleEnabled = async (id: number, currentStatus: boolean) => {
    // Optimistically update UI
    addOptimisticSettings({ type: 'toggle', id });

    try {
      // Rust API returns StatusCode::NO_CONTENT (204) on success
      await apiClient.post<void>(`/settings/${id}/toggle`, { enabled: !currentStatus });
      fetchSettings();
    } catch (err) {
      alert('Error: ' + (err instanceof Error ? err.message : 'Unknown error'));
      fetchSettings(); // Revert on error
    }
  };

  // Create new setting
  const createSetting = async (formData: CreateSettingsRequest) => {
    // Optimistically add to UI with temporary ID
    const tempSetting: CopySettings = {
      ...formData,
      id: Date.now(), // Temporary ID
      enabled: true,
      symbol_mappings: [],
      filters: {
        allowed_symbols: null,
        blocked_symbols: null,
        allowed_magic_numbers: null,
        blocked_magic_numbers: null,
      },
    };
    addOptimisticSettings({ type: 'add', data: tempSetting });

    try {
      // Rust API returns the new ID as Json<i32> with StatusCode::CREATED (201)
      await apiClient.post<number>('/settings', formData);
      fetchSettings();
    } catch (err) {
      alert('Error: ' + (err instanceof Error ? err.message : 'Unknown error'));
      fetchSettings(); // Revert on error
    }
  };

  // Update setting
  const updateSetting = async (id: number, updatedData: CopySettings) => {
    // Optimistically update UI
    addOptimisticSettings({ type: 'update', id, data: updatedData });

    try {
      // Rust API returns StatusCode::NO_CONTENT (204) on success
      await apiClient.put<void>(`/settings/${id}`, updatedData);
      fetchSettings();
    } catch (err) {
      alert('Error: ' + (err instanceof Error ? err.message : 'Unknown error'));
      fetchSettings(); // Revert on error
    }
  };

  // Delete setting
  const deleteSetting = async (id: number) => {
    if (!confirm('Are you sure you want to delete this connection?')) {
      return;
    }

    // Optimistically remove from UI
    addOptimisticSettings({ type: 'delete', id });

    try {
      // Rust API returns StatusCode::NO_CONTENT (204) on success
      await apiClient.delete<void>(`/settings/${id}`);
      fetchSettings();
    } catch (err) {
      alert('Error: ' + (err instanceof Error ? err.message : 'Unknown error'));
      fetchSettings(); // Revert on error
    }
  };

  return {
    settings: optimisticSettings, // Use optimistic settings for instant UI updates
    connections,
    loading,
    error,
    wsMessages,
    toggleEnabled,
    createSetting,
    updateSetting,
    deleteSetting,
  };
}
