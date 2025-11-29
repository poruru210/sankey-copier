// Custom hook for ZeroMQ configuration API
// Fetches port configuration from relay-server (read-only)
// Ports are dynamically assigned by the server or read from config.toml

import { useState, useEffect, useCallback } from 'react';
import { useAtomValue } from 'jotai';
import { apiClientAtom } from '@/lib/atoms/site';

// ZeroMQ config response from GET /api/zeromq-config
// 2-port architecture: receiver (PULL) and unified sender (PUB)
export interface ZeromqConfigResponse {
  /** PULL socket port (EA → Server) */
  receiver_port: number;
  /** PUB socket port (unified for trade signals and config messages) */
  sender_port: number;
  /** Whether ports are dynamically assigned (from runtime.toml) or fixed (from config.toml) */
  is_dynamic: boolean;
  /** When dynamic ports were generated (ISO 8601 format) */
  generated_at?: string;
}

// Default response when not loaded
const DEFAULT_CONFIG: ZeromqConfigResponse = {
  receiver_port: 5555,
  sender_port: 5556,
  is_dynamic: false,
};

export function useZeromqConfig() {
  const apiClient = useAtomValue(apiClientAtom);
  const [config, setConfig] = useState<ZeromqConfigResponse>(DEFAULT_CONFIG);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Fetch config from server
  const fetchConfig = useCallback(async () => {
    if (!apiClient) return;

    try {
      setLoading(true);
      const data = await apiClient.get<ZeromqConfigResponse>('/zeromq-config');
      if (data) {
        setConfig(data);
      }
      setError(null);
    } catch (err) {
      console.error('Failed to fetch ZeroMQ config:', err);
      setError(err instanceof Error ? err.message : 'Failed to fetch config');
    } finally {
      setLoading(false);
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
    config,
    // State
    loading,
    error,
    // Actions
    refetch: fetchConfig,
  };
}
