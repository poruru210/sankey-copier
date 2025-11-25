'use client';

// ServerLog component - displays server logs in a collapsible panel at the bottom
// Uses shadcn useSidebar for sidebar state and useServerLogContext for ServerLog-specific state

import { useEffect } from 'react';
import { useIntlayer } from 'next-intlayer';
import { useAtomValue } from 'jotai';
import { Button } from '@/components/ui/button';
import { Switch } from '@/components/ui/switch';
import { Label } from '@/components/ui/label';
import { RefreshCw, ChevronUp, ChevronDown, Maximize2, Minimize2 } from 'lucide-react';
import { apiClientAtom } from '@/lib/atoms/site';
import { useSidebar } from '@/components/ui/sidebar';
import { useServerLogContext } from '@/lib/contexts/sidebar-context';
import { useServerLogs, useLogViewerResize, useLogViewerLayout } from './ServerLog.hooks';
import { LOG_LEVEL_COLORS } from './ServerLog.constants';
import { cn } from '@/lib/utils';

export function ServerLog() {
  const apiClient = useAtomValue(apiClientAtom);

  // Sidebar state from shadcn SidebarProvider
  const { open: isSidebarOpen, isMobile } = useSidebar();

  // ServerLog-specific state from ServerLogProvider
  const {
    serverLogExpanded: isExpanded,
    setServerLogExpanded: setIsExpanded,
    setServerLogHeight,
  } = useServerLogContext();
  const { title, noLogs, refreshButton, loading, error: errorText, toggleLabel } = useIntlayer('server-log');

  // Custom hooks
  const { logs, isLoading, error, autoRefresh, setAutoRefresh, fetchLogs } = useServerLogs(apiClient, isExpanded);
  const { height, isMaximized, handleResizeStart, toggleMaximize } = useLogViewerResize();

  // Layout adjustments
  useLogViewerLayout(isExpanded, height, isMaximized);

  // Update ServerLog height in context for page padding adjustment
  useEffect(() => {
    if (isExpanded) {
      setServerLogHeight(isMaximized ? window.innerHeight : height);
    } else {
      setServerLogHeight(40); // Collapsed bar height
    }
  }, [isExpanded, height, isMaximized, setServerLogHeight]);

  // Utility functions
  const getLevelColor = (level: string) => {
    const upperLevel = level.toUpperCase() as keyof typeof LOG_LEVEL_COLORS;
    return LOG_LEVEL_COLORS[upperLevel] || LOG_LEVEL_COLORS.DEFAULT;
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
      <div
        className={cn(
          'fixed bottom-0 right-0 z-[100] bg-background border-t border-border shadow-2xl transition-all duration-300',
          !isMobile && (isSidebarOpen ? 'left-64' : 'left-16'),
          isMobile && 'left-0'
        )}
      >
        <div className="flex items-center justify-between px-4 py-2">
          <div className="flex items-center gap-3">
            <span className="text-sm font-semibold text-foreground">{title}</span>
            {logs.length > 0 && (
              <span className="text-xs text-muted-foreground">
                {logs.length} {logs.length === 1 ? 'log' : 'logs'}
              </span>
            )}
          </div>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setIsExpanded(true)}
            className="h-7"
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
      className={cn(
        'fixed right-0 z-[100] bg-card border-t border-border shadow-2xl flex flex-col transition-all duration-300',
        !isMobile && (isSidebarOpen ? 'left-64' : 'left-16'),
        isMobile && 'left-0'
      )}
      style={{
        bottom: 0,
        height: isMaximized ? 'calc(100vh - 0px)' : `${height}px`,
        top: isMaximized ? 0 : 'auto'
      }}
    >
      {/* Resize handle */}
      {!isMaximized && (
        <div
          className="h-1 bg-border hover:bg-primary cursor-ns-resize transition-colors"
          onMouseDown={handleResizeStart}
        />
      )}

      {/* Header */}
      <div className="flex items-center justify-between px-4 py-2 bg-background border-b border-border">
        <div className="flex items-center gap-3">
          <span className="text-sm font-semibold text-foreground">{title}</span>
          {logs.length > 0 && (
            <span className="text-xs text-muted-foreground">
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
              className="scale-75"
            />
            <Label htmlFor="auto-refresh-expanded" className="text-xs cursor-pointer text-foreground">
              自動更新 (3秒)
            </Label>
          </div>

          <Button
            variant="ghost"
            size="sm"
            onClick={fetchLogs}
            disabled={isLoading}
            className="h-7"
          >
            <RefreshCw className={`h-3 w-3 mr-1 ${isLoading ? 'animate-spin' : ''}`} />
            <span className="text-xs">{refreshButton}</span>
          </Button>

          <div className="h-4 w-px bg-border" />

          <Button
            variant="ghost"
            size="sm"
            onClick={toggleMaximize}
            className="h-7"
            title={isMaximized ? '復元' : '最大化'}
          >
            {isMaximized ? <Minimize2 className="h-3.5 w-3.5" /> : <Maximize2 className="h-3.5 w-3.5" />}
          </Button>

          <Button
            variant="ghost"
            size="sm"
            onClick={() => setIsExpanded(false)}
            className="h-7"
          >
            <ChevronDown className="h-4 w-4" />
          </Button>
        </div>
      </div>

      {/* Log content */}
      <div className="flex-1 overflow-y-auto bg-muted/30 p-3 font-mono text-xs">
        {isLoading && logs.length === 0 ? (
          <p className="text-muted-foreground">{loading}</p>
        ) : error ? (
          <p className="text-destructive">
            {errorText}: {error}
          </p>
        ) : logs.length === 0 ? (
          <p className="text-muted-foreground">{noLogs}</p>
        ) : (
          <div className="space-y-0.5">
            {logs.map((log, idx) => (
              <div key={idx} className="flex gap-3 hover:bg-accent/50 px-2 py-1 rounded">
                <span className="text-muted-foreground whitespace-nowrap select-none">
                  {formatTimestamp(log.timestamp)}
                </span>
                <span className={`font-semibold whitespace-nowrap select-none ${getLevelColor(log.level)}`}>
                  [{log.level.toUpperCase()}]
                </span>
                <span className="flex-1 break-all text-foreground">{log.message}</span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
