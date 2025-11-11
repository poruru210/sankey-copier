'use client';

import { useState, useCallback, useEffect } from 'react';
import { useIntlayer } from 'next-intlayer';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { RefreshCw } from 'lucide-react';

interface LogEntry {
  timestamp: string;
  level: string;
  message: string;
}

interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
}

export function ServerLog() {
  const { title, noLogs, refreshButton, loading, error: errorText } = useIntlayer('server-log');
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchLogs = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      const response = await fetch('/api/logs');
      const data: ApiResponse<LogEntry[]> = await response.json();

      if (data.success && data.data) {
        setLogs(data.data);
      } else {
        setError(data.error || 'Failed to fetch logs');
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch logs');
    } finally {
      setIsLoading(false);
    }
  }, []);

  // Fetch logs on component mount
  useEffect(() => {
    fetchLogs();
  }, [fetchLogs]);

  const getLevelColor = (level: string) => {
    switch (level.toUpperCase()) {
      case 'ERROR':
        return 'text-red-600 dark:text-red-400';
      case 'WARN':
        return 'text-yellow-600 dark:text-yellow-400';
      case 'INFO':
        return 'text-blue-600 dark:text-blue-400';
      case 'DEBUG':
        return 'text-gray-600 dark:text-gray-400';
      default:
        return 'text-gray-600 dark:text-gray-400';
    }
  };

  const formatTimestamp = (timestamp: string) => {
    try {
      const date = new Date(timestamp);
      return date.toLocaleTimeString('ja-JP', {
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit',
        hour12: false
      });
    } catch {
      return timestamp;
    }
  };

  return (
    <Card className="mb-6 mt-6">
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-4">
        <CardTitle className="text-xl">{title}</CardTitle>
        <Button
          variant="outline"
          size="sm"
          onClick={fetchLogs}
          disabled={isLoading}
          className="h-8"
        >
          <RefreshCw className={`h-4 w-4 mr-2 ${isLoading ? 'animate-spin' : ''}`} />
          {refreshButton}
        </Button>
      </CardHeader>
      <CardContent>
        <div className="space-y-1 max-h-60 overflow-y-auto">
          {isLoading && logs.length === 0 ? (
            <p className="text-muted-foreground text-sm">{loading}</p>
          ) : error ? (
            <p className="text-red-600 dark:text-red-400 text-sm">
              {errorText}: {error}
            </p>
          ) : logs.length === 0 ? (
            <p className="text-muted-foreground text-sm">{noLogs}</p>
          ) : (
            logs.map((log, idx) => (
              <div key={idx} className="text-xs font-mono bg-muted p-2 rounded flex gap-2">
                <span className="text-muted-foreground whitespace-nowrap">
                  {formatTimestamp(log.timestamp)}
                </span>
                <span className={`font-semibold whitespace-nowrap ${getLevelColor(log.level)}`}>
                  {log.level.toUpperCase()}
                </span>
                <span className="flex-1 break-all">{log.message}</span>
              </div>
            ))
          )}
        </div>
      </CardContent>
    </Card>
  );
}
