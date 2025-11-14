'use client';

import { useEffect, useState, useOptimistic, useTransition } from 'react';
import { useIntlayer } from 'next-intlayer';
import { ParticlesBackground } from '@/components/ParticlesBackground';
import { useMtInstallations } from '@/hooks/useMtInstallations';
import { useSidebar } from '@/lib/contexts/sidebar-context';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Checkbox } from '@/components/ui/checkbox';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { AlertCircle, CheckCircle, Download, Loader2, RefreshCw } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { MtInstallation } from '@/types';

export default function InstallationsPage() {
  const content = useIntlayer('installations-page');
  const { installations, loading, error, installing, fetchInstallations, installToMt } = useMtInstallations();
  const { isOpen: isSidebarOpen, isMobile, serverLogHeight } = useSidebar();
  const [message, setMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [isPending, startTransition] = useTransition();
  const [optimisticInstallations, setOptimisticInstallations] = useOptimistic(
    installations,
    (currentInstallations, updatedId: string) => {
      return currentInstallations.map(inst =>
        inst.id === updatedId
          ? {
              ...inst,
              components: {
                dll: true,
                master_ea: true,
                slave_ea: true,
              }
            }
          : inst
      );
    }
  );

  useEffect(() => {
    fetchInstallations();
  }, [fetchInstallations]);

  const handleInstall = async (installation: MtInstallation) => {
    setMessage(null);

    // Optimistically update the UI
    startTransition(() => {
      setOptimisticInstallations(installation.id);
    });

    const result = await installToMt(installation.id);

    if (result.success) {
      setMessage({ type: 'success', text: result.message || content.installationCompleted.value });
    } else {
      setMessage({ type: 'error', text: result.message || content.installationFailed.value });
    }

    // Clear message after 5 seconds
    setTimeout(() => setMessage(null), 5000);
  };

  const handleBatchInstall = async () => {
    if (selectedIds.size === 0) return;

    setMessage(null);

    const selectedInstallations = optimisticInstallations.filter(inst => selectedIds.has(inst.id));
    let successCount = 0;
    let failCount = 0;

    // Optimistically update all selected installations
    startTransition(() => {
      selectedInstallations.forEach(installation => {
        setOptimisticInstallations(installation.id);
      });
    });

    for (const installation of selectedInstallations) {
      const result = await installToMt(installation.id);
      if (result.success) {
        successCount++;
      } else {
        failCount++;
      }
    }

    setSelectedIds(new Set());

    if (failCount === 0) {
      setMessage({
        type: 'success',
        text: content.successfullyInstalled.value.replace('{count}', successCount.toString())
      });
    } else if (successCount === 0) {
      setMessage({
        type: 'error',
        text: content.failedToInstall.value.replace('{count}', failCount.toString())
      });
    } else {
      setMessage({
        type: 'error',
        text: content.completedWithErrors.value
          .replace('{successCount}', successCount.toString())
          .replace('{failCount}', failCount.toString())
      });
    }

    setTimeout(() => setMessage(null), 5000);
  };

  const toggleSelection = (id: string) => {
    const newSelection = new Set(selectedIds);
    if (newSelection.has(id)) {
      newSelection.delete(id);
    } else {
      newSelection.add(id);
    }
    setSelectedIds(newSelection);
  };

  const toggleSelectAll = () => {
    if (selectedIds.size === optimisticInstallations.length) {
      setSelectedIds(new Set());
    } else {
      setSelectedIds(new Set(optimisticInstallations.map(inst => inst.id)));
    }
  };

  const getComponentStatus = (installation: MtInstallation) => {
    const { components } = installation;
    const installed = [components.dll, components.master_ea, components.slave_ea].filter(Boolean).length;
    const total = 3;
    return { installed, total };
  };

  if (loading && installations.length === 0) {
    return (
      <div className="min-h-screen bg-background flex items-center justify-center">
        <div className="flex flex-col items-center gap-4">
          <Loader2 className="h-8 w-8 animate-spin" />
          <div className="text-xl">{content.loadingInstallations}</div>
        </div>
      </div>
    );
  }

  return (
    <div className="h-screen bg-background relative overflow-hidden flex flex-col">
      {/* Particles Background */}
      <ParticlesBackground />

      {/* Main Content */}
      <div className="relative z-10 flex flex-col h-full">
        <div
          className={cn(
            'overflow-y-auto transition-all duration-300',
            !isMobile && (isSidebarOpen ? 'lg:ml-64' : 'lg:ml-16')
          )}
          style={{
            height: `calc(100vh - 56px - ${serverLogHeight}px)`,
            maxHeight: `calc(100vh - 56px - ${serverLogHeight}px)`
          }}
        >
          <div className="w-[95%] mx-auto p-4">
          {/* Page Title */}
          <div className="mb-4">
            <h1 className="text-xl font-bold mb-1">{content.title}</h1>
            <p className="text-sm text-muted-foreground">
              {content.description}
            </p>
          </div>

          {/* Action Buttons */}
          <div className="mb-6 flex gap-3">
            <Button
              onClick={fetchInstallations}
              disabled={loading}
              variant="outline"
              className="gap-2"
            >
              <RefreshCw className={`h-4 w-4 ${loading ? 'animate-spin' : ''}`} />
              {content.refreshDetection}
            </Button>
            {optimisticInstallations.length > 0 && (
              <Button
                onClick={handleBatchInstall}
                disabled={selectedIds.size === 0 || isPending || installing !== null}
                className="gap-2"
              >
                {isPending ? (
                  <>
                    <Loader2 className="h-4 w-4 animate-spin" />
                    {content.installing} {selectedIds.size} {content.installationsCount}...
                  </>
                ) : (
                  <>
                    <Download className="h-4 w-4" />
                    {content.installToSelected} ({selectedIds.size})
                  </>
                )}
              </Button>
            )}
          </div>

          {/* Error Display */}
          {error && (
            <div className="bg-destructive/10 border border-destructive text-destructive px-4 py-3 rounded-lg mb-6 flex items-center gap-2">
              <AlertCircle className="h-5 w-5" />
              {error}
            </div>
          )}

          {/* Message Display */}
          {message && (
            <div
              className={`px-4 py-3 rounded-lg mb-6 flex items-center gap-2 ${
                message.type === 'success'
                  ? 'bg-green-500/10 border border-green-500 text-green-500'
                  : 'bg-destructive/10 border border-destructive text-destructive'
              }`}
            >
              {message.type === 'success' ? (
                <CheckCircle className="h-5 w-5" />
              ) : (
                <AlertCircle className="h-5 w-5" />
              )}
              {message.text}
            </div>
          )}

          {/* Installations Table */}
          {optimisticInstallations.length === 0 ? (
            <Card>
              <CardContent className="py-12 text-center">
                <p className="text-lg text-muted-foreground">
                  {content.noInstallationsDetected}
                </p>
                <p className="text-sm text-muted-foreground mt-2">
                  {content.clickRefreshToScan}
                </p>
              </CardContent>
            </Card>
          ) : (
            <div className="rounded-lg border bg-card">
              <Table>
                <TableHeader>
                  <TableRow className="h-9">
                    <TableHead className="w-[40px] py-2">
                      <Checkbox
                        checked={selectedIds.size === optimisticInstallations.length && optimisticInstallations.length > 0}
                        onCheckedChange={toggleSelectAll}
                      />
                    </TableHead>
                    <TableHead className="py-2">{content.name}</TableHead>
                    <TableHead className="py-2">{content.installationPath}</TableHead>
                    <TableHead className="py-2">{content.version}</TableHead>
                    <TableHead className="py-2">{content.components}</TableHead>
                    <TableHead className="text-right py-2">{content.actions}</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {optimisticInstallations.map((installation) => {
                    const componentStatus = getComponentStatus(installation);
                    const isInstalling = installing === installation.id;
                    const allComponentsInstalled = componentStatus.installed === componentStatus.total;
                    const isSelected = selectedIds.has(installation.id);

                    return (
                      <TableRow
                        key={installation.id}
                        data-state={isSelected ? 'selected' : undefined}
                        onClick={() => toggleSelection(installation.id)}
                        className="cursor-pointer h-10"
                      >
                        <TableCell onClick={(e) => e.stopPropagation()} className="py-1">
                          <Checkbox
                            checked={isSelected}
                            onCheckedChange={() => toggleSelection(installation.id)}
                          />
                        </TableCell>
                        <TableCell className="font-medium py-1">
                          <div className="flex items-center gap-2">
                            <Badge variant="outline" className="text-xs px-1.5 py-0">{installation.type}</Badge>
                            <span>{installation.name}</span>
                          </div>
                        </TableCell>
                        <TableCell className="py-1">
                          <p className="text-xs font-mono truncate max-w-xs" title={installation.path}>
                            {installation.path}
                          </p>
                        </TableCell>
                        <TableCell className="py-1">
                          {installation.version ? (
                            <span className="text-xs font-mono">v{installation.version}</span>
                          ) : (
                            <span className="text-xs text-muted-foreground">-</span>
                          )}
                        </TableCell>
                        <TableCell className="py-1">
                          <div className="flex gap-2">
                            <div className="flex items-center gap-1" title={content.dll.value}>
                              {installation.components.dll ? (
                                <CheckCircle className="h-3 w-3 text-green-500" />
                              ) : (
                                <div className="h-3 w-3 rounded-full border-2 border-muted" />
                              )}
                              <span className="text-xs">DLL</span>
                            </div>
                            <div className="flex items-center gap-1" title={content.master.value}>
                              {installation.components.master_ea ? (
                                <CheckCircle className="h-3 w-3 text-green-500" />
                              ) : (
                                <div className="h-3 w-3 rounded-full border-2 border-muted" />
                              )}
                              <span className="text-xs">M</span>
                            </div>
                            <div className="flex items-center gap-1" title={content.slave.value}>
                              {installation.components.slave_ea ? (
                                <CheckCircle className="h-3 w-3 text-green-500" />
                              ) : (
                                <div className="h-3 w-3 rounded-full border-2 border-muted" />
                              )}
                              <span className="text-xs">S</span>
                            </div>
                          </div>
                        </TableCell>
                        <TableCell className="text-right py-1" onClick={(e) => e.stopPropagation()}>
                          <Button
                            onClick={() => handleInstall(installation)}
                            disabled={isInstalling || isPending}
                            size="sm"
                            variant={allComponentsInstalled ? 'outline' : 'default'}
                          >
                            {isInstalling ? (
                              <>
                                <Loader2 className="h-3 w-3 animate-spin mr-1" />
                                {content.installing}...
                              </>
                            ) : allComponentsInstalled ? (
                              content.reinstall
                            ) : (
                              content.install
                            )}
                          </Button>
                        </TableCell>
                      </TableRow>
                    );
                  })}
                </TableBody>
              </Table>
            </div>
          )}
          </div>
        </div>
      </div>
    </div>
  );
}
