'use client';

import { useState, useCallback, useEffect, useRef } from 'react';
import { useIntlayer } from 'next-intlayer';
import { Button } from '@/components/ui/button';
import { Switch } from '@/components/ui/switch';
import { Label } from '@/components/ui/label';
import { RefreshCw, ChevronUp, ChevronDown, Maximize2, Minimize2 } from 'lucide-react';
import { useApiClient } from '@/lib/contexts/site-context';

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
  const apiClient = useApiClient();
  const { title, noLogs, refreshButton, loading, error: errorText, toggleLabel, closeLabel } = useIntlayer('server-log');
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [autoRefresh, setAutoRefresh] = useState(false);
  const [isExpanded, setIsExpanded] = useState(false);
  const [height, setHeight] = useState(350);
  const [isResizing, setIsResizing] = useState(false);
  const [isMaximized, setIsMaximized] = useState(false);
  const [previousHeight, setPreviousHeight] = useState(350);
  const resizeStartRef = useRef<{ y: number; height: number } | null>(null);

  const fetchLogs = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      const data = await apiClient.get<ApiResponse<LogEntry[]>>('/logs');

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
  }, [apiClient]);

  // Fetch logs on component mount
  useEffect(() => {
    fetchLogs();
  }, [fetchLogs]);

  // Auto-refresh logs every 3 seconds when enabled
  useEffect(() => {
    if (!autoRefresh) {
      return;
    }

    const intervalId = setInterval(() => {
      fetchLogs();
    }, 3000); // 3 seconds

    return () => clearInterval(intervalId);
  }, [autoRefresh, fetchLogs]);

  // Add padding to main content when expanded to prevent content from being hidden
  useEffect(() => {
    const mainContent = document.querySelector('.relative.z-10') as HTMLElement;
    const pageContainer = document.querySelector('.min-h-screen') as HTMLElement;
    if (!mainContent || !pageContainer) return;

    // Collapsed bar height is approximately 40px
    const collapsedBarHeight = 40;

    // Set fixed height on page container to prevent double scrollbars
    pageContainer.style.height = '100vh';
    pageContainer.style.overflow = 'hidden';

    // Make main content scrollable
    mainContent.style.height = '100vh';
    mainContent.style.overflowY = 'auto';
    mainContent.style.overflowX = 'hidden';

    if (isExpanded) {
      const paddingValue = isMaximized ? window.innerHeight : height;
      mainContent.style.paddingBottom = `${paddingValue}px`;
    } else {
      // Even when collapsed, add padding for the bottom bar
      mainContent.style.paddingBottom = `${collapsedBarHeight}px`;
    }

    // Force browser to recalculate scrollbar by toggling overflow
    requestAnimationFrame(() => {
      const currentScrollTop = mainContent.scrollTop;
      mainContent.style.overflowY = 'hidden';

      // Force reflow
      void mainContent.offsetHeight;

      requestAnimationFrame(() => {
        mainContent.style.overflowY = 'auto';
        mainContent.scrollTop = currentScrollTop;
      });
    });

    return () => {
      mainContent.style.paddingBottom = '0px';
      mainContent.style.height = '';
      mainContent.style.overflowY = '';
      mainContent.style.overflowX = '';
      pageContainer.style.height = '';
      pageContainer.style.overflow = '';
    };
  }, [isExpanded, height, isMaximized]);

  // Handle mouse resize
  useEffect(() => {
    if (!isResizing) return;

    const handleMouseMove = (e: MouseEvent) => {
      if (!resizeStartRef.current) return;

      const deltaY = resizeStartRef.current.y - e.clientY;
      const maxHeight = Math.floor(window.innerHeight * 0.9); // 90% of viewport height
      const newHeight = Math.max(200, Math.min(maxHeight, resizeStartRef.current.height + deltaY));
      setHeight(newHeight);
    };

    const handleMouseUp = () => {
      setIsResizing(false);
      resizeStartRef.current = null;
    };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);

    return () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };
  }, [isResizing]);

  const handleResizeStart = (e: React.MouseEvent) => {
    e.preventDefault();
    setIsResizing(true);
    resizeStartRef.current = { y: e.clientY, height };
  };

  const toggleMaximize = () => {
    if (isMaximized) {
      // Restore to previous height
      setHeight(previousHeight);
      setIsMaximized(false);
    } else {
      // Save current height and maximize
      setPreviousHeight(height);
      setIsMaximized(true);
    }
  };

  const getLevelColor = (level: string) => {
    switch (level.toUpperCase()) {
      case 'ERROR':
        return 'text-red-400';
      case 'WARN':
        return 'text-yellow-400';
      case 'INFO':
        return 'text-blue-400';
      case 'DEBUG':
        return 'text-gray-400';
      default:
        return 'text-gray-400';
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

  // Collapsed bar at bottom
  if (!isExpanded) {
    return (
      <div className="fixed bottom-0 left-0 right-0 z-50 bg-slate-900 border-t border-slate-700 shadow-2xl">
        <div className="flex items-center justify-between px-4 py-2">
          <div className="flex items-center gap-3">
            <span className="text-sm font-semibold text-slate-200">{title}</span>
            {logs.length > 0 && (
              <span className="text-xs text-slate-400">
                {logs.length} {logs.length === 1 ? 'log' : 'logs'}
              </span>
            )}
          </div>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setIsExpanded(true)}
            className="h-7 text-slate-300 hover:text-white hover:bg-slate-800"
          >
            <ChevronUp className="h-4 w-4 mr-1" />
            {toggleLabel || '展開'}
          </Button>
        </div>
      </div>
    );
  }

  // Expanded terminal view
  return (
    <div
      className="fixed left-0 right-0 z-50 bg-slate-950 border-t border-slate-700 shadow-2xl flex flex-col"
      style={{
        bottom: 0,
        height: isMaximized ? 'calc(100vh - 0px)' : `${height}px`,
        top: isMaximized ? 0 : 'auto'
      }}
    >
      {/* Resize handle */}
      {!isMaximized && (
        <div
          className="h-1 bg-slate-700 hover:bg-blue-500 cursor-ns-resize transition-colors"
          onMouseDown={handleResizeStart}
        />
      )}

      {/* Header */}
      <div className="flex items-center justify-between px-4 py-2 bg-slate-900 border-b border-slate-700">
        <div className="flex items-center gap-3">
          <span className="text-sm font-semibold text-slate-200">{title}</span>
          {logs.length > 0 && (
            <span className="text-xs text-slate-400">
              {logs.length} {logs.length === 1 ? 'log' : 'logs'}
            </span>
          )}
        </div>

        <div className="flex items-center gap-3">
          <div className="flex items-center gap-1.5">
            <Switch
              id="auto-refresh-expanded"
              checked={autoRefresh}
              onCheckedChange={setAutoRefresh}
              className="data-[state=checked]:bg-blue-600 scale-75"
            />
            <Label htmlFor="auto-refresh-expanded" className="text-xs cursor-pointer text-slate-300">
              自動更新 (3秒)
            </Label>
          </div>

          <Button
            variant="ghost"
            size="sm"
            onClick={fetchLogs}
            disabled={isLoading}
            className="h-7 text-slate-300 hover:text-white hover:bg-slate-800"
          >
            <RefreshCw className={`h-3 w-3 mr-1 ${isLoading ? 'animate-spin' : ''}`} />
            <span className="text-xs">{refreshButton}</span>
          </Button>

          <div className="h-4 w-px bg-slate-700" />

          <Button
            variant="ghost"
            size="sm"
            onClick={toggleMaximize}
            className="h-7 text-slate-300 hover:text-white hover:bg-slate-800"
            title={isMaximized ? '復元' : '最大化'}
          >
            {isMaximized ? <Minimize2 className="h-3.5 w-3.5" /> : <Maximize2 className="h-3.5 w-3.5" />}
          </Button>

          <Button
            variant="ghost"
            size="sm"
            onClick={() => setIsExpanded(false)}
            className="h-7 text-slate-300 hover:text-white hover:bg-slate-800"
          >
            <ChevronDown className="h-4 w-4" />
          </Button>
        </div>
      </div>

      {/* Log content */}
      <div className="flex-1 overflow-y-auto bg-slate-950 p-3 font-mono text-xs">
        {isLoading && logs.length === 0 ? (
          <p className="text-slate-500">{loading}</p>
        ) : error ? (
          <p className="text-red-400">
            {errorText}: {error}
          </p>
        ) : logs.length === 0 ? (
          <p className="text-slate-500">{noLogs}</p>
        ) : (
          <div className="space-y-0.5">
            {logs.map((log, idx) => (
              <div key={idx} className="flex gap-3 hover:bg-slate-900/50 px-2 py-1 rounded">
                <span className="text-slate-500 whitespace-nowrap select-none">
                  {formatTimestamp(log.timestamp)}
                </span>
                <span className={`font-semibold whitespace-nowrap select-none ${getLevelColor(log.level)}`}>
                  [{log.level.toUpperCase()}]
                </span>
                <span className="flex-1 break-all text-slate-200">{log.message}</span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
