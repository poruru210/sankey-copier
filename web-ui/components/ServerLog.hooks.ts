import { useState, useCallback, useEffect, useRef } from 'react';
import { LOG_VIEWER_CONSTANTS, DOM_SELECTORS } from './ServerLog.constants';

interface LogEntry {
  timestamp: string;
  level: string;
  message: string;
}

interface ApiClient {
  get: <T>(path: string) => Promise<T>;
}

// Hook for fetching and managing server logs
export function useServerLogs(apiClient: ApiClient) {
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

// Hook for managing log viewer resize functionality
export function useLogViewerResize(initialHeight: number = LOG_VIEWER_CONSTANTS.DEFAULT_HEIGHT) {
  const [height, setHeight] = useState<number>(initialHeight);
  const [isResizing, setIsResizing] = useState(false);
  const [isMaximized, setIsMaximized] = useState(false);
  const [previousHeight, setPreviousHeight] = useState(initialHeight);
  const resizeStartRef = useRef<{ y: number; height: number } | null>(null);

  const handleResizeStart = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsResizing(true);
    resizeStartRef.current = { y: e.clientY, height };
  }, [height]);

  const toggleMaximize = useCallback(() => {
    if (isMaximized) {
      setHeight(previousHeight);
      setIsMaximized(false);
    } else {
      setPreviousHeight(height);
      setIsMaximized(true);
    }
  }, [isMaximized, height, previousHeight]);

  // Handle mouse resize
  useEffect(() => {
    if (!isResizing) return;

    const handleMouseMove = (e: MouseEvent) => {
      if (!resizeStartRef.current) return;

      const deltaY = resizeStartRef.current.y - e.clientY;
      const maxHeight = Math.floor(window.innerHeight * LOG_VIEWER_CONSTANTS.MAX_HEIGHT_RATIO);
      const newHeight = Math.max(
        LOG_VIEWER_CONSTANTS.MIN_HEIGHT,
        Math.min(maxHeight, resizeStartRef.current.height + deltaY)
      );
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

  return {
    height,
    isMaximized,
    handleResizeStart,
    toggleMaximize,
  };
}

// Hook for managing layout adjustments when log viewer is expanded
export function useLogViewerLayout(
  isExpanded: boolean,
  height: number,
  isMaximized: boolean
) {
  useEffect(() => {
    const mainContent = document.querySelector(DOM_SELECTORS.MAIN_CONTENT) as HTMLElement;
    const pageContainer = document.querySelector(DOM_SELECTORS.PAGE_CONTAINER) as HTMLElement;

    if (!mainContent || !pageContainer) return;

    // Set fixed height on page container to prevent page scrollbar
    pageContainer.style.height = '100vh';
    pageContainer.style.overflow = 'hidden';

    // Make main content scrollable with height adjusted for log viewer
    mainContent.style.overflowY = 'auto';
    mainContent.style.overflowX = 'hidden';

    if (isExpanded) {
      const logViewerHeight = isMaximized ? window.innerHeight : height;
      // Adjust main content height to exclude log viewer height
      mainContent.style.height = isMaximized ? '0px' : `calc(100vh - ${logViewerHeight}px)`;
      mainContent.style.paddingBottom = '0px';
    } else {
      // When collapsed, main content height excludes collapsed bar
      mainContent.style.height = `calc(100vh - ${LOG_VIEWER_CONSTANTS.COLLAPSED_BAR_HEIGHT}px)`;
      mainContent.style.paddingBottom = '0px';
    }

    return () => {
      mainContent.style.paddingBottom = '';
      mainContent.style.height = '';
      mainContent.style.overflowY = '';
      mainContent.style.overflowX = '';
      pageContainer.style.height = '';
      pageContainer.style.overflow = '';
    };
  }, [isExpanded, height, isMaximized]);
}
