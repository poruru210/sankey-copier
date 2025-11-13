'use client';

import { useEffect, useState, useOptimistic, useTransition } from 'react';
import { Header } from '@/components/Header';
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
                dll: { installed: true, version: inst.components.dll.version },
                master_ea: { installed: true, version: inst.components.master_ea.version },
                slave_ea: { installed: true, version: inst.components.slave_ea.version },
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
      setMessage({ type: 'success', text: result.message || 'Installation completed successfully' });
    } else {
      setMessage({ type: 'error', text: result.message || 'Installation failed' });
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
        text: `Successfully installed components to ${successCount} installation(s)`
      });
    } else if (successCount === 0) {
      setMessage({
        type: 'error',
        text: `Failed to install components to all ${failCount} installation(s)`
      });
    } else {
      setMessage({
        type: 'error',
        text: `Completed with ${successCount} success and ${failCount} failure(s)`
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
    const installed = [components.dll.installed, components.master_ea.installed, components.slave_ea.installed].filter(Boolean).length;
    const total = 3;
    return { installed, total };
  };

  if (loading && installations.length === 0) {
    return (
      <div className="min-h-screen bg-background flex items-center justify-center">
        <div className="flex flex-col items-center gap-4">
          <Loader2 className="h-8 w-8 animate-spin" />
          <div className="text-xl">Loading installations...</div>
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
        <Header />
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
          <div className="w-[80%] mx-auto p-6">
          {/* Page Title */}
          <div className="mb-6">
            <h1 className="text-3xl font-bold mb-2">Installation Manager</h1>
            <p className="text-muted-foreground">
              Detect and install SANKEY Copier components to your MT4/MT5 platforms
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
              Refresh Detection
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
                    Installing to {selectedIds.size} installation(s)...
                  </>
                ) : (
                  <>
                    <Download className="h-4 w-4" />
                    Install to Selected ({selectedIds.size})
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
                  No MT4/MT5 installations detected.
                </p>
                <p className="text-sm text-muted-foreground mt-2">
                  Click &quot;Refresh Detection&quot; to scan for installations
                </p>
              </CardContent>
            </Card>
          ) : (
            <div className="rounded-lg border bg-card">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead className="w-[50px]">
                      <Checkbox
                        checked={selectedIds.size === optimisticInstallations.length && optimisticInstallations.length > 0}
                        onCheckedChange={toggleSelectAll}
                      />
                    </TableHead>
                    <TableHead>Name</TableHead>
                    <TableHead>Type</TableHead>
                    <TableHead>Platform</TableHead>
                    <TableHead>Installation Path</TableHead>
                    <TableHead>Version</TableHead>
                    <TableHead>Components</TableHead>
                    <TableHead className="text-right">Actions</TableHead>
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
                        className="cursor-pointer"
                      >
                        <TableCell onClick={(e) => e.stopPropagation()}>
                          <Checkbox
                            checked={isSelected}
                            onCheckedChange={() => toggleSelection(installation.id)}
                          />
                        </TableCell>
                        <TableCell className="font-medium">{installation.name}</TableCell>
                        <TableCell>
                          <Badge variant="outline">{installation.type}</Badge>
                        </TableCell>
                        <TableCell>{installation.platform}</TableCell>
                        <TableCell>
                          <div className="max-w-[300px]">
                            <p className="text-sm font-mono truncate" title={installation.path}>
                              {installation.path}
                            </p>
                          </div>
                        </TableCell>
                        <TableCell>{installation.version || '-'}</TableCell>
                        <TableCell>
                          <div className="space-y-1">
                            <ComponentStatusItem
                              name="DLL"
                              installed={installation.components.dll.installed}
                              version={installation.components.dll.version}
                            />
                            <ComponentStatusItem
                              name="Master"
                              installed={installation.components.master_ea.installed}
                              version={installation.components.master_ea.version}
                            />
                            <ComponentStatusItem
                              name="Slave"
                              installed={installation.components.slave_ea.installed}
                              version={installation.components.slave_ea.version}
                            />
                          </div>
                        </TableCell>
                        <TableCell className="text-right" onClick={(e) => e.stopPropagation()}>
                          <Button
                            onClick={() => handleInstall(installation)}
                            disabled={isInstalling || isPending}
                            size="sm"
                            variant={allComponentsInstalled ? 'outline' : 'default'}
                          >
                            {isInstalling ? (
                              <>
                                <Loader2 className="h-3 w-3 animate-spin mr-1" />
                                Installing...
                              </>
                            ) : allComponentsInstalled ? (
                              'Reinstall'
                            ) : (
                              'Install'
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

function ComponentStatusItem({ name, installed, version }: { name: string; installed: boolean; version: string | null }) {
  return (
    <div className="flex items-center justify-between gap-2 text-sm">
      <div className="flex items-center gap-2">
        {installed ? (
          <CheckCircle className="h-4 w-4 text-green-500" />
        ) : (
          <div className="h-4 w-4 rounded-full border-2 border-muted" />
        )}
        <span className={`text-xs ${installed ? 'text-foreground' : 'text-muted-foreground'}`}>{name}</span>
      </div>
      {installed && version && (
        <span className="text-xs text-muted-foreground font-mono">{version}</span>
      )}
    </div>
  );
}
