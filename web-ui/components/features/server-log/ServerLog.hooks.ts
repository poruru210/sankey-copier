import { useState, useCallback, useEffect } from 'react';
import { LOG_VIEWER_CONSTANTS } from './ServerLog.constants';

interface LogEntry {
  timestamp: string;
  level: string;
  message: string;
}

interface ApiClient {
  get: <T>(path: string) => Promise<T>;
}

// Hook for fetching and managing server logs
export function useServerLogs(apiClient: ApiClient, isExpanded?: boolean) {
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [autoRefresh, setAutoRefresh] = useState(false);

  const fetchLogs = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      // Rust API returns Vec<LogEntry> directly (not wrapped)
      const logs = await apiClient.get<LogEntry[]>('/logs');
      setLogs(logs);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch logs');
    } finally {
      setIsLoading(false);
    }
  }, [apiClient]);

  // Fetch logs on mount
  useEffect(() => {
    fetchLogs();
  }, [fetchLogs]);

  // Fetch logs when expanded
  useEffect(() => {
    if (isExpanded) {
      fetchLogs();
    }
  }, [isExpanded, fetchLogs]);

  // Auto-refresh logs
  useEffect(() => {
    if (!autoRefresh) return;

    const intervalId = setInterval(() => {
      fetchLogs();
    }, LOG_VIEWER_CONSTANTS.AUTO_REFRESH_INTERVAL_MS);

    return () => clearInterval(intervalId);
  }, [autoRefresh, fetchLogs]);

  return {
    logs,
    isLoading,
    error,
    autoRefresh,
    setAutoRefresh,
    fetchLogs,
  };
}

