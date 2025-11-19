import { useState, useEffect, useCallback, useRef } from 'react';
import { useAtom, useAtomValue } from 'jotai';
import { debounce } from 'lodash-es';
import type { CopySettings, EaConnection, CreateSettingsRequest } from '@/types';
import { selectedSiteAtom, apiClientAtom } from '@/lib/atoms/site';
import { settingsAtom } from '@/lib/atoms/settings';
import { connectionsAtom } from '@/lib/atoms/connections';

export function useSankeyCopier() {
  const apiClient = useAtomValue(apiClientAtom);
  const selectedSite = useAtomValue(selectedSiteAtom);

  const [settings, setSettings] = useAtom(settingsAtom);
  const [connections, setConnections] = useAtom(connectionsAtom);

  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [wsMessages, setWsMessages] = useState<string[]>([]);

  // Fetch connections
  const fetchConnections = useCallback(async () => {
    if (!apiClient) return;
    try {
      // Rust API returns Vec<EaConnection> directly (not wrapped)
      const data = await apiClient.get<EaConnection[]>('/connections');
      if (data) {
        setConnections(data);
      }
    } catch (err) {
      if (err instanceof TypeError && err.message.includes('fetch')) {
        console.error('Cannot connect to server - is relay-server running?');
      } else {
        console.error('Failed to fetch connections:', err);
      }
    }
  }, [apiClient, setConnections]);

  // Fetch settings
  const fetchSettings = useCallback(async () => {
    if (!apiClient) return;
    try {
      setLoading(true);
      // Rust API returns Vec<CopySettings> directly (not wrapped)
      const data = await apiClient.get<CopySettings[]>('/settings');
      if (data) {
        setSettings(data);
      }
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
  }, [apiClient, setSettings]);

  // WebSocket connection
  useEffect(() => {
    if (!selectedSite?.siteUrl) return;

    // Extract host from siteUrl (e.g., "https://localhost:3000" -> "localhost:3000")
    // Use wss:// for https:// sites, ws:// for http:// sites
    const siteHost = selectedSite.siteUrl.replace(/^https?:\/\//, '');
    const wsProtocol = selectedSite.siteUrl.startsWith('https') ? 'wss' : 'ws';
    const wsUrl = `${wsProtocol}://${siteHost}/ws`;
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

      if (message.startsWith('settings_created:')) {
        try {
          const jsonStr = message.substring('settings_created:'.length);
          const newSetting = JSON.parse(jsonStr) as CopySettings;
          setSettings((prev) => [...prev, newSetting]);
        } catch (e) {
          console.error('Failed to parse settings_created message:', e);
          fetchSettings(); // Fallback
        }
      } else if (message.startsWith('settings_updated:')) {
        try {
          const jsonStr = message.substring('settings_updated:'.length);
          const updatedSetting = JSON.parse(jsonStr) as CopySettings;
          setSettings((prev) =>
            prev.map((s) => (s.id === updatedSetting.id ? updatedSetting : s))
          );
        } catch (e) {
          console.error('Failed to parse settings_updated message:', e);
          fetchSettings(); // Fallback
        }
      } else if (message.startsWith('settings_deleted:')) {
        const idStr = message.substring('settings_deleted:'.length);
        const id = parseInt(idStr, 10);
        if (!isNaN(id)) {
          setSettings((prev) => prev.filter((s) => s.id !== id));
        } else {
          fetchSettings(); // Fallback
        }
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

        // Close connection if it's open or connecting
        if (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING) {
          ws.close();
        }
      }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedSite?.siteUrl, fetchSettings]);

  // Initial load and periodic connection refresh
  useEffect(() => {
    if (apiClient) {
      fetchSettings();
      fetchConnections();
      const interval = setInterval(fetchConnections, 5000);
      return () => clearInterval(interval);
    }
  }, [apiClient, fetchSettings, fetchConnections]);

  // Map to store debounced functions for each setting ID
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const debouncedCallsRef = useRef<Map<number, any>>(new Map());

  // Toggle status (DISABLED ⇄ ENABLED)
  const toggleEnabled = useCallback(async (id: number, currentStatus: number) => {
    if (!apiClient) return;

    // Optimistically update UI
    setSettings((prev) =>
      prev.map(s => s.id === id ? { ...s, status: s.status === 0 ? 1 : 0 } : s)
    );

    const newStatus = currentStatus === 0 ? 1 : 0;

    // Get or create debounced function for this specific ID
    let debouncedFn = debouncedCallsRef.current.get(id);
    if (!debouncedFn) {
      debouncedFn = debounce(async (status: number) => {
        try {
          await apiClient.post<void>(`/settings/${id}/toggle`, { status });
        } catch (err) {
          console.error(`Failed to toggle setting ${id}`, err);
          fetchSettings(); // Refresh on error
        }
      }, 300);
      debouncedCallsRef.current.set(id, debouncedFn);
    }

    // Call the debounced function for this ID
    debouncedFn(newStatus);
  }, [apiClient, fetchSettings, setSettings]);

  // Create new setting
  const createSetting = async (formData: CreateSettingsRequest) => {
    if (!apiClient) return;
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

    const previousSettings = settings;
    setSettings((prev) => [...prev, tempSetting]);

    try {
      // Rust API returns the new ID as Json<i32> with StatusCode::CREATED (201)
      await apiClient.post<number>('/settings', formData);
      // fetchSettings(); // Removed to avoid duplicate fetch (handled by WS)
    } catch (err) {
      setSettings(previousSettings); // Revert on error
      throw err; // Re-throw for caller to handle
    }
  };

  // Update setting
  const updateSetting = async (id: number, updatedData: CopySettings) => {
    if (!apiClient) return;
    // Optimistically update UI
    const previousSettings = settings;
    setSettings((prev) =>
      prev.map(s => s.id === id ? updatedData : s)
    );

    try {
      // Rust API returns StatusCode::NO_CONTENT (204) on success
      await apiClient.put<void>(`/settings/${id}`, updatedData);
      // fetchSettings(); // Removed to avoid duplicate fetch (handled by WS)
    } catch (err) {
      setSettings(previousSettings); // Revert on error
      throw err; // Re-throw for caller to handle
    }
  };

  // Delete setting
  const deleteSetting = async (id: number) => {
    if (!apiClient) return;
    // Optimistically remove from UI
    const previousSettings = settings;
    setSettings((prev) => prev.filter(s => s.id !== id));

    try {
      // Rust API returns StatusCode::NO_CONTENT (204) on success
      await apiClient.delete<void>(`/settings/${id}`);
      // fetchSettings(); // Removed to avoid duplicate fetch (handled by WS)
    } catch (err) {
      setSettings(previousSettings); // Revert on error
      throw err; // Re-throw for caller to handle
    }
  };

  return {
    settings,
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
