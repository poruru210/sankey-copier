'use client';

// Installations page - displays MT4/MT5 installations and allows component installation
// Layout is managed by SidebarInset in LayoutWrapper, only ServerLog height adjustment needed

import { useEffect, useState, useOptimistic, useTransition } from 'react';
import { useIntlayer } from 'next-intlayer';
import { ParticlesBackground } from '@/components/ParticlesBackground';
import { useMtInstallations } from '@/hooks/useMtInstallations';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Checkbox } from '@/components/ui/checkbox';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Typography, Muted } from '@/components/ui/typography';
import { AlertCircle, AlertTriangle, CheckCircle, Download, Loader2, RefreshCw } from 'lucide-react';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { HoverCard, HoverCardContent, HoverCardTrigger } from '@/components/ui/hover-card';
import type { MtInstallation, EaPortConfig } from '@/types';

export default function InstallationsPage() {
  const content = useIntlayer('installations-page');
  const { installations, loading, error, installing, fetchInstallations, installToMt } = useMtInstallations();
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
      setMessage({ type: 'success', text: result.message || content.installationCompleted });
    } else {
      setMessage({ type: 'error', text: result.message || content.installationFailed });
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
        text: content.successfullyInstalled.replace('{count}', successCount.toString())
      });
    } else if (successCount === 0) {
      setMessage({
        type: 'error',
        text: content.failedToInstall.replace('{count}', failCount.toString())
      });
    } else {
      setMessage({
        type: 'error',
        text: content.completedWithErrors
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

  const getInstallationStatus = (installation: MtInstallation) => {
    const { components, port_mismatch, port_config } = installation;
    const allComponentsInstalled = components.dll && components.master_ea && components.slave_ea;

    // Status Logic
    if (allComponentsInstalled && !port_mismatch && port_config) {
      return { status: 'healthy', icon: CheckCircle, color: 'text-green-500', label: content.statusHealthy };
    } else if (port_mismatch || (port_config && !allComponentsInstalled)) {
      // Installed but mismatch or partial install
      return { status: 'warning', icon: AlertTriangle, color: 'text-yellow-500', label: content.statusWarning };
    } else {
      // Not installed or error
      return { status: 'error', icon: AlertCircle, color: 'text-muted-foreground', label: content.notInstalled };
    }
  };

  if (loading && installations.length === 0) {
    return (
      <div className="min-h-screen bg-background flex items-center justify-center">
        <div className="flex flex-col items-center gap-4">
          <Loader2 className="h-8 w-8 animate-spin" />
          <Typography variant="large">{content.loadingInstallations}</Typography>
        </div>
      </div>
    );
  }

  return (
    <div className="h-full bg-background relative overflow-hidden flex flex-col">
      {/* Particles Background */}
      <ParticlesBackground />

      {/* Main Content */}
      <div className="relative z-10 flex flex-col overflow-y-auto h-full">
        <div className="w-[95%] mx-auto p-4">
          {/* Page Title */}
          <div className="mb-6">
            <Typography variant="h3" className="mb-2">{content.title}</Typography>
            <Muted>{content.description}</Muted>
          </div>

          {/* Action Buttons */}
          <div className="mb-6 flex gap-3">
            {optimisticInstallations.length > 0 && (
              <Button
                onClick={handleBatchInstall}
                disabled={selectedIds.size === 0 || isPending || installing !== null}
                className="gap-2 min-h-[44px] md:min-h-0"
              >
                {isPending ? (
                  <>
                    <Loader2 className="h-4 w-4 animate-spin" />
                    {content.installing} {selectedIds.size} {content.installationsCount}...
                  </>
                ) : (
                  <>
                    <Download className="h-4 w-4" />
                    {content.install} ({selectedIds.size})
                  </>
                )}
              </Button>
            )}
            <Button
              onClick={fetchInstallations}
              disabled={loading}
              variant="outline"
              className="gap-2 min-h-[44px] md:min-h-0"
            >
              <RefreshCw className={`h-4 w-4 ${loading ? 'animate-spin' : ''}`} />
              {content.refresh}
            </Button>
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
              className={`px-4 py-3 rounded-lg mb-6 flex items-center gap-2 ${message.type === 'success'
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
                <Typography variant="large" className="text-muted-foreground">
                  {content.noInstallationsDetected}
                </Typography>
                <Muted className="mt-2">
                  {content.clickRefreshToScan}
                </Muted>
              </CardContent>
            </Card>
          ) : (
            <div className="rounded-lg border bg-card overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow className="h-12 md:h-10">
                    <TableHead className="w-[40px] py-2">
                      <Checkbox
                        checked={selectedIds.size === optimisticInstallations.length && optimisticInstallations.length > 0}
                        onCheckedChange={toggleSelectAll}
                      />
                    </TableHead>
                    <TableHead className="py-2 text-sm font-medium">{content.name}</TableHead>
                    <TableHead className="py-2 text-sm font-medium w-[100px]">{content.status}</TableHead>
                    <TableHead className="py-2 text-sm font-medium hidden md:table-cell">{content.installationPath}</TableHead>
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
                        onClick={() => toggleSelection(installation.id)}
                        className="cursor-pointer h-14 md:h-12"
                      >
                        <TableCell onClick={(e) => e.stopPropagation()} className="py-2">
                          <Checkbox
                            checked={isSelected}
                            onCheckedChange={() => toggleSelection(installation.id)}
                          />
                        </TableCell>
                        <TableCell className="font-medium py-2">
                          <div className="flex items-center gap-2">
                            <Badge
                              className={`text-xs px-1.5 py-0 ${installation.type === 'MT4'
                                ? 'bg-blue-500 text-white hover:bg-blue-600'
                                : 'bg-purple-500 text-white hover:bg-purple-600'
                                }`}
                            >
                              {installation.type}
                            </Badge>
                            <span className="text-sm">{installation.name}</span>
                          </div>
                        </TableCell>
                        <TableCell className="py-2">
                          {(() => {
                            const { icon: StatusIcon, color, label } = getInstallationStatus(installation);
                            return (
                              <HoverCard>
                                <HoverCardTrigger asChild>
                                  <div className={`flex items-center gap-1 w-fit px-2 py-1 rounded-full bg-muted/50 hover:bg-muted ${color}`}>
                                    <StatusIcon className="h-4 w-4" />
                                    <span className="text-xs font-medium">{label}</span>
                                  </div>
                                </HoverCardTrigger>
                                <HoverCardContent className="w-auto min-w-[200px] p-3" side="right" align="start">
                                  <div className="grid grid-cols-[auto,1fr] gap-x-3 gap-y-1 text-xs items-center">
                                    {/* Master EA */}
                                    <div className="flex items-center gap-1.5">
                                      {installation.components.master_ea ? <CheckCircle className="h-3.5 w-3.5 text-green-500" /> : <div className="h-3.5 w-3.5 rounded-full border-2 border-muted" />}
                                      <span>{content.master}</span>
                                    </div>
                                    <div className="text-right text-muted-foreground">-</div>

                                    {/* Slave EA */}
                                    <div className="flex items-center gap-1.5">
                                      {installation.components.slave_ea ? <CheckCircle className="h-3.5 w-3.5 text-green-500" /> : <div className="h-3.5 w-3.5 rounded-full border-2 border-muted" />}
                                      <span>{content.slave}</span>
                                    </div>
                                    <div className="text-right text-muted-foreground">-</div>

                                    {/* DLL */}
                                    <div className="flex items-center gap-1.5">
                                      {installation.components.dll ? <CheckCircle className="h-3.5 w-3.5 text-green-500" /> : <div className="h-3.5 w-3.5 rounded-full border-2 border-muted" />}
                                      <span>{content.dll}</span>
                                    </div>
                                    <div className="text-right font-mono">
                                      {installation.version ? `v${installation.version}` : '-'}
                                    </div>

                                    {/* Ports */}
                                    {installation.port_config && (
                                      <>
                                        <div className="flex items-center gap-1.5">
                                          {!installation.port_mismatch ? <CheckCircle className="h-3.5 w-3.5 text-green-500" /> : <AlertTriangle className="h-3.5 w-3.5 text-yellow-500" />}
                                          <span>{content.ports}</span>
                                        </div>
                                        <div className="text-right font-mono">
                                          {installation.port_config.receiver_port}, {installation.port_config.publisher_port}
                                        </div>
                                      </>
                                    )}
                                  </div>
                                </HoverCardContent>
                              </HoverCard>
                            );
                          })()}
                        </TableCell>
                        <TableCell className="py-2 hidden md:table-cell">
                          <p className="text-sm text-muted-foreground truncate max-w-xs lg:max-w-2xl" title={installation.path}>
                            {installation.path}
                          </p>
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
  );
}
