'use client';

// ServerLog component - displays server logs in a Drawer (bottom sheet)
// Uses useServerLogContext for open/close state

import { useIntlayer } from 'next-intlayer';
import { useAtomValue } from 'jotai';
import { Button } from '@/components/ui/button';
import { Switch } from '@/components/ui/switch';
import { Label } from '@/components/ui/label';
import { RefreshCw } from 'lucide-react';
import { apiClientAtom } from '@/lib/atoms/site';
import { useServerLogContext } from '@/lib/contexts/sidebar-context';
import { useServerLogs } from './ServerLog.hooks';
import { LOG_LEVEL_COLORS } from './ServerLog.constants';
import {
  Drawer,
  DrawerContent,
  DrawerHeader,
  DrawerTitle,
  DrawerDescription,
  DrawerFooter,
  DrawerClose,
} from '@/components/ui/drawer';

export function ServerLog() {
  const apiClient = useAtomValue(apiClientAtom);

  // ServerLog-specific state from ServerLogProvider
  // serverLogExpanded is now used as "isDrawerOpen"
  const {
    serverLogExpanded: isOpen,
    setServerLogExpanded: setIsOpen,
  } = useServerLogContext();

  const { title, noLogs, refreshButton, loading, error: errorText } = useIntlayer('server-log');

  // Custom hooks
  // We pass isOpen to fetch logs when opened
  const { logs, isLoading, error, autoRefresh, setAutoRefresh, fetchLogs } = useServerLogs(apiClient, isOpen);

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

  return (
    <Drawer open={isOpen} onOpenChange={setIsOpen}>
      <DrawerContent className="h-[85vh] max-h-[85vh] flex flex-col">
        <DrawerHeader className="border-b px-4 py-2">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <DrawerTitle className="text-sm font-semibold text-foreground text-left">{title}</DrawerTitle>
              {logs.length > 0 && (
                <DrawerDescription className="text-xs text-muted-foreground m-0">
                  {logs.length} {logs.length === 1 ? 'log' : 'logs'}
                </DrawerDescription>
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
            </div>
          </div>
        </DrawerHeader>

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

        <DrawerFooter className="pt-2 pb-4">
          <DrawerClose asChild>
            <Button variant="outline">Close</Button>
          </DrawerClose>
        </DrawerFooter>
      </DrawerContent>
    </Drawer>
  );
}

