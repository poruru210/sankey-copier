import { useState, useEffect, useCallback, useRef } from 'react';
import { useAtom, useAtomValue } from 'jotai';
import { debounce } from 'lodash-es';
import type {
  CopySettings,
  EaConnection,
  CreateSettingsRequest,
  TradeGroup,
  TradeGroupMember,
} from '@/types';
import { selectedSiteAtom, apiClientAtom } from '@/lib/atoms/site';
import { settingsAtom } from '@/lib/atoms/settings';
import { connectionsAtom } from '@/lib/atoms/connections';
import { convertMembersToCopySettings } from '@/utils/tradeGroupAdapter';

export function useSankeyCopier() {
  const apiClient = useAtomValue(apiClientAtom);
  const selectedSite = useAtomValue(selectedSiteAtom);

  const [settings, setSettings] = useAtom(settingsAtom);
  const [connections, setConnections] = useAtom(connectionsAtom);
  const [tradeGroups, setTradeGroups] = useState<TradeGroup[]>([]);

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

  // Fetch settings (using new TradeGroups API)
  const fetchSettings = useCallback(async () => {
    if (!apiClient) return;
    try {
      setLoading(true);

      // Fetch all TradeGroups (Masters)
      const fetchedTradeGroups = await apiClient.get<TradeGroup[]>('/trade-groups');
      if (!fetchedTradeGroups) {
        setSettings([]);
        setTradeGroups([]);
        setError(null);
        return;
      }
      setTradeGroups(fetchedTradeGroups);

      // Fetch members for each TradeGroup
      const membersMap = new Map<string, TradeGroupMember[]>();
      await Promise.all(
        fetchedTradeGroups.map(async (tradeGroup) => {
          try {
            const members = await apiClient.get<TradeGroupMember[]>(
              `/trade-groups/${encodeURIComponent(tradeGroup.id)}/members`
            );
            if (members) {
              membersMap.set(tradeGroup.id, members);
            }
          } catch (err) {
            console.error(`Failed to fetch members for ${tradeGroup.id}:`, err);
            membersMap.set(tradeGroup.id, []);
          }
        })
      );

      // Convert to legacy CopySettings format
      const copySettings = convertMembersToCopySettings(fetchedTradeGroups, membersMap);
      setSettings(copySettings);
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

      // Refresh data when changes are detected
      // Note: With new API, we refresh all settings instead of parsing individual messages
      if (message.startsWith('trade_') || message.startsWith('member_') || message.startsWith('settings_')) {
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

        // Close connection if it's open or connecting
        if (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING) {
          ws.close();
        }
      }
    };

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
  const debouncedCallsRef = useRef<Map<number, ReturnType<typeof debounce>>>(new Map());

  // Cleanup debounced functions on unmount or when apiClient changes
  useEffect(() => {
    const debouncedCalls = debouncedCallsRef.current;
    return () => {
      debouncedCalls.forEach((debouncedFn) => {
        debouncedFn.cancel();
      });
      debouncedCalls.clear();
    };
  }, []);

  // Toggle status (DISABLED ⇄ ENABLED)
  const toggleEnabled = useCallback(async (id: number, nextEnabled: boolean) => {
    if (!apiClient) return;

    // Find the setting to get master_account and slave_account for API call
    const setting = settings.find(s => s.id === id);
    if (!setting) {
      console.error(`Setting ${id} not found`);
      return;
    }

    // Optimistically update intent flag only; runtime status comes from server heartbeat
    setSettings((prev) =>
      prev.map(s =>
        s.id === id
          ? {
              ...s,
              enabled_flag: nextEnabled,
            }
          : s
      )
    );

    // Get or create debounced function for this specific ID
    let debouncedFn = debouncedCallsRef.current.get(id);

    // Create a new debounced function if it doesn't exist
    // Note: We don't need to worry about stale apiClient here because we clear the map
    // when apiClient changes (in the useEffect above), forcing recreation.
    if (!debouncedFn) {
      debouncedFn = debounce(async (masterAccount: string, slaveAccount: string, enabled: boolean) => {
        try {
          await apiClient.post<void>(
            `/trade-groups/${encodeURIComponent(masterAccount)}/members/${encodeURIComponent(slaveAccount)}/toggle`,
            { enabled }
          );
        } catch (err) {
          console.error(`Failed to toggle setting for ${slaveAccount}`, err);
          fetchSettings(); // Refresh on error
        }
      }, 300);
      debouncedCallsRef.current.set(id, debouncedFn);
    }

    // Call the debounced function for this ID (using slave_account for API)
    debouncedFn(setting.master_account, setting.slave_account, nextEnabled);
  }, [apiClient, fetchSettings, setSettings, settings]);

  // Create new setting
  const createSetting = async (formData: CreateSettingsRequest) => {
    if (!apiClient) return;

    try {
      // Import converter function
      const { convertCreateRequestToMemberData } = await import('@/utils/tradeGroupAdapter');
      const memberData = convertCreateRequestToMemberData(formData);

      // Send to new API endpoint
      await apiClient.post<number>(
        `/trade-groups/${encodeURIComponent(formData.master_account)}/members`,
        memberData
      );

      // Refresh settings to get updated data
      await fetchSettings();
    } catch (err) {
      throw err; // Re-throw for caller to handle
    }
  };

  // Update setting
  const updateSetting = async (id: number, updatedData: CopySettings) => {
    if (!apiClient) return;

    // Find the setting to get master_account and slave_account
    const originalSetting = settings.find(s => s.id === id);
    if (!originalSetting) {
      throw new Error(`Setting ${id} not found`);
    }

    // Optimistically update UI
    const previousSettings = settings;
    setSettings((prev) =>
      prev.map(s => s.id === id ? updatedData : s)
    );

    try {
      // Import converter function
      const { convertCopySettingsToSlaveSettings } = await import('@/utils/tradeGroupAdapter');
      const slaveSettings = convertCopySettingsToSlaveSettings(updatedData);

      // Send to new API endpoint (using slave_account, not numeric id)
      // Server expects SlaveSettings directly (not wrapped in { slave_settings: ... })
      await apiClient.put<void>(
        `/trade-groups/${encodeURIComponent(originalSetting.master_account)}/members/${encodeURIComponent(originalSetting.slave_account)}`,
        slaveSettings
      );

      // Refresh to ensure consistency
      await fetchSettings();
    } catch (err) {
      setSettings(previousSettings); // Revert on error
      throw err; // Re-throw for caller to handle
    }
  };

  // Delete setting
  const deleteSetting = async (id: number) => {
    if (!apiClient) return;

    // Find the setting to get master_account and slave_account
    const setting = settings.find(s => s.id === id);
    if (!setting) {
      throw new Error(`Setting ${id} not found`);
    }

    // Optimistically remove from UI
    const previousSettings = settings;
    setSettings((prev) => prev.filter(s => s.id !== id));

    try {
      // Send to new API endpoint (using slave_account, not numeric id)
      await apiClient.delete<void>(
        `/trade-groups/${encodeURIComponent(setting.master_account)}/members/${encodeURIComponent(setting.slave_account)}`
      );

      // Refresh to ensure consistency
      await fetchSettings();
    } catch (err) {
      setSettings(previousSettings); // Revert on error
      throw err; // Re-throw for caller to handle
    }
  };

  // Toggle Master enabled state
  const toggleMaster = async (masterAccount: string, enabled: boolean) => {
    if (!apiClient) throw new Error('API client not available');

    // Optimistically update TradeGroups
    const previousTradeGroups = tradeGroups;
    setTradeGroups((prev) =>
      prev.map((tg) =>
        tg.id === masterAccount
          ? { ...tg, master_settings: { ...tg.master_settings, enabled } }
          : tg
      )
    );

    try {
      await apiClient.post<TradeGroup>(
        `/trade-groups/${encodeURIComponent(masterAccount)}/toggle`,
        { enabled }
      );
    } catch (err) {
      // Revert on error
      setTradeGroups(previousTradeGroups);
      throw err;
    }
  };

  return {
    settings,
    connections,
    tradeGroups,
    loading,
    error,
    wsMessages,
    toggleEnabled,
    toggleMaster,
    createSetting,
    updateSetting,
    deleteSetting,
  };
}
