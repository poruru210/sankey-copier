import { useState, useEffect, useCallback, useOptimistic, startTransition } from 'react';
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
          return state.map(s => s.id === action.id ? { ...s, status: s.status === 0 ? 1 : 0 } : s);
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
        console.error('Cannot connect to server - is relay-server running?');
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
    const wsUrl = `ws://${siteHost}/ws`;
    console.log('WebSocket connecting to:', wsUrl);

    let ws: WebSocket | null = null;
    let isCleanup = false;

    try {
      ws = new WebSocket(wsUrl);
    } catch (err) {
      console.error('Failed to create WebSocket:', err);
      return;
    }

    ws.onopen = () => {
      if (!isCleanup) {
        console.log('WebSocket connected to', wsUrl);
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
      if (!isCleanup) {
        console.error('WebSocket error:', error, 'URL:', wsUrl, 'ReadyState:', ws?.readyState);
      }
    };

    ws.onclose = (event) => {
      if (!isCleanup) {
        console.log('WebSocket disconnected. Code:', event.code, 'Reason:', event.reason);
      }
    };

    return () => {
      isCleanup = true;
      if (ws) {
        // Remove event handlers to prevent error messages during cleanup
        ws.onerror = null;
        ws.onclose = null;
        ws.onopen = null;
        ws.onmessage = null;

        // Only close if connection is established
        if (ws.readyState === WebSocket.OPEN) {
          ws.close();
        }
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

  // Toggle status (DISABLED ⇄ ENABLED)
  const toggleEnabled = async (id: number, currentStatus: number) => {
    // Optimistically update UI
    startTransition(() => {
      addOptimisticSettings({ type: 'toggle', id });
    });

    try {
      // Toggle between DISABLED (0) and ENABLED (1)
      const newStatus = currentStatus === 0 ? 1 : 0;
      // Rust API returns StatusCode::NO_CONTENT (204) on success
      await apiClient.post<void>(`/settings/${id}/toggle`, { status: newStatus });
      fetchSettings();
    } catch (err) {
      fetchSettings(); // Revert on error
      throw err; // Re-throw for caller to handle
    }
  };

  // Create new setting
  const createSetting = async (formData: CreateSettingsRequest) => {
    // Optimistically add to UI with temporary ID
    const tempSetting: CopySettings = {
      ...formData,
      id: Date.now(), // Temporary ID
      symbol_mappings: [],
      filters: {
        allowed_symbols: null,
        blocked_symbols: null,
        allowed_magic_numbers: null,
        blocked_magic_numbers: null,
      },
    };
    startTransition(() => {
      addOptimisticSettings({ type: 'add', data: tempSetting });
    });

    try {
      // Rust API returns the new ID as Json<i32> with StatusCode::CREATED (201)
      await apiClient.post<number>('/settings', formData);
      fetchSettings();
    } catch (err) {
      fetchSettings(); // Revert on error
      throw err; // Re-throw for caller to handle
    }
  };

  // Update setting
  const updateSetting = async (id: number, updatedData: CopySettings) => {
    // Optimistically update UI
    startTransition(() => {
      addOptimisticSettings({ type: 'update', id, data: updatedData });
    });

    try {
      // Rust API returns StatusCode::NO_CONTENT (204) on success
      await apiClient.put<void>(`/settings/${id}`, updatedData);
      fetchSettings();
    } catch (err) {
      fetchSettings(); // Revert on error
      throw err; // Re-throw for caller to handle
    }
  };

  // Delete setting
  const deleteSetting = async (id: number) => {
    // Optimistically remove from UI
    startTransition(() => {
      addOptimisticSettings({ type: 'delete', id });
    });

    try {
      // Rust API returns StatusCode::NO_CONTENT (204) on success
      await apiClient.delete<void>(`/settings/${id}`);
      fetchSettings();
    } catch (err) {
      fetchSettings(); // Revert on error
      throw err; // Re-throw for caller to handle
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
