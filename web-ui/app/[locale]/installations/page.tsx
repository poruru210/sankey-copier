'use client';

import { useEffect, useState } from 'react';
import { Header } from '@/components/Header';
import { ParticlesBackground } from '@/components/ParticlesBackground';
import { useMtInstallations } from '@/hooks/useMtInstallations';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { AlertCircle, CheckCircle, Download, Loader2, RefreshCw } from 'lucide-react';
import type { MtInstallation } from '@/types';

export default function InstallationsPage() {
  const { installations, loading, error, installing, fetchInstallations, installToMt } = useMtInstallations();
  const [message, setMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null);

  useEffect(() => {
    fetchInstallations();
  }, [fetchInstallations]);

  const handleInstall = async (installation: MtInstallation) => {
    setMessage(null);
    const result = await installToMt(installation.id);

    if (result.success) {
      setMessage({ type: 'success', text: result.message || 'Installation completed successfully' });
    } else {
      setMessage({ type: 'error', text: result.message || 'Installation failed' });
    }

    // Clear message after 5 seconds
    setTimeout(() => setMessage(null), 5000);
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
          <div className="text-xl">Loading installations...</div>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-background relative overflow-hidden">
      {/* Particles Background */}
      <ParticlesBackground />

      {/* Main Content */}
      <div className="relative z-10">
        <Header />
        <div className="container mx-auto p-6 max-w-[1200px]">
          {/* Page Title */}
          <div className="mb-6">
            <h1 className="text-3xl font-bold mb-2">Installation Manager</h1>
            <p className="text-muted-foreground">
              Detect and install SANKEY Copier components to your MT4/MT5 platforms
            </p>
          </div>

          {/* Refresh Button */}
          <div className="mb-6">
            <Button
              onClick={fetchInstallations}
              disabled={loading}
              variant="outline"
              className="gap-2"
            >
              <RefreshCw className={`h-4 w-4 ${loading ? 'animate-spin' : ''}`} />
              Refresh Detection
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

          {/* Installations Grid */}
          {installations.length === 0 ? (
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
            <>
              {/* MT5 Group */}
              {(() => {
                const mt5Installations = installations.filter((i) => i.type === 'MT5');
                if (mt5Installations.length === 0) return null;

                return (
                  <div className="mb-8">
                    <div className="flex items-center gap-3 mb-4">
                      <h2 className="text-2xl font-bold">MetaTrader 5</h2>
                      <Badge variant="outline">{mt5Installations.length} found</Badge>
                    </div>
                    <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                      {mt5Installations.map((installation) => {
                        const componentStatus = getComponentStatus(installation);
                        const isInstalling = installing === installation.id;
                        const allComponentsInstalled = componentStatus.installed === componentStatus.total;

                        return (
                          <Card key={installation.id}>
                            <CardHeader>
                              <div className="flex items-start justify-between">
                                <div className="flex-1">
                                  <CardTitle className="text-lg">{installation.name}</CardTitle>
                                  <CardDescription className="mt-1">
                                    {installation.platform}
                                  </CardDescription>
                                </div>
                                <Badge variant={installation.is_running ? 'default' : 'secondary'}>
                                  {installation.is_running ? 'Running' : 'Stopped'}
                                </Badge>
                              </div>
                            </CardHeader>
                            <CardContent>
                              {/* Path */}
                              <div className="mb-4">
                                <p className="text-xs text-muted-foreground mb-1">Installation Path</p>
                                <p className="text-sm font-mono truncate" title={installation.path}>
                                  {installation.path}
                                </p>
                              </div>

                              {/* Version */}
                              {installation.version && (
                                <div className="mb-4">
                                  <p className="text-xs text-muted-foreground mb-1">Version</p>
                                  <p className="text-sm">{installation.version}</p>
                                </div>
                              )}

                              {/* Component Status */}
                              <div className="mb-4">
                                <p className="text-xs text-muted-foreground mb-2">Components Status</p>
                                <div className="space-y-1">
                                  <ComponentStatusItem
                                    name="DLL"
                                    installed={installation.components.dll}
                                  />
                                  <ComponentStatusItem
                                    name="Master EA"
                                    installed={installation.components.master_ea}
                                  />
                                  <ComponentStatusItem
                                    name="Slave EA"
                                    installed={installation.components.slave_ea}
                                  />
                                </div>
                                <div className="mt-2">
                                  <Badge variant={allComponentsInstalled ? 'default' : 'secondary'}>
                                    {componentStatus.installed}/{componentStatus.total} installed
                                  </Badge>
                                </div>
                              </div>

                              {/* Install Button */}
                              <Button
                                onClick={() => handleInstall(installation)}
                                disabled={isInstalling || allComponentsInstalled}
                                className="w-full gap-2"
                                variant={allComponentsInstalled ? 'outline' : 'default'}
                              >
                                {isInstalling ? (
                                  <>
                                    <Loader2 className="h-4 w-4 animate-spin" />
                                    Installing...
                                  </>
                                ) : allComponentsInstalled ? (
                                  <>
                                    <CheckCircle className="h-4 w-4" />
                                    All Components Installed
                                  </>
                                ) : (
                                  <>
                                    <Download className="h-4 w-4" />
                                    Install Components
                                  </>
                                )}
                              </Button>

                              {installation.is_running && !allComponentsInstalled && (
                                <p className="text-xs text-muted-foreground mt-2 text-center">
                                  Warning: MT is running. Installation may require restart.
                                </p>
                              )}
                            </CardContent>
                          </Card>
                        );
                      })}
                    </div>
                  </div>
                );
              })()}

              {/* MT4 Group */}
              {(() => {
                const mt4Installations = installations.filter((i) => i.type === 'MT4');
                if (mt4Installations.length === 0) return null;

                return (
                  <div className="mb-8">
                    <div className="flex items-center gap-3 mb-4">
                      <h2 className="text-2xl font-bold">MetaTrader 4</h2>
                      <Badge variant="outline">{mt4Installations.length} found</Badge>
                    </div>
                    <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                      {mt4Installations.map((installation) => {
                        const componentStatus = getComponentStatus(installation);
                        const isInstalling = installing === installation.id;
                        const allComponentsInstalled = componentStatus.installed === componentStatus.total;

                        return (
                          <Card key={installation.id}>
                            <CardHeader>
                              <div className="flex items-start justify-between">
                                <div className="flex-1">
                                  <CardTitle className="text-lg">{installation.name}</CardTitle>
                                  <CardDescription className="mt-1">
                                    {installation.platform}
                                  </CardDescription>
                                </div>
                                <Badge variant={installation.is_running ? 'default' : 'secondary'}>
                                  {installation.is_running ? 'Running' : 'Stopped'}
                                </Badge>
                              </div>
                            </CardHeader>
                            <CardContent>
                              {/* Path */}
                              <div className="mb-4">
                                <p className="text-xs text-muted-foreground mb-1">Installation Path</p>
                                <p className="text-sm font-mono truncate" title={installation.path}>
                                  {installation.path}
                                </p>
                              </div>

                              {/* Version */}
                              {installation.version && (
                                <div className="mb-4">
                                  <p className="text-xs text-muted-foreground mb-1">Version</p>
                                  <p className="text-sm">{installation.version}</p>
                                </div>
                              )}

                              {/* Component Status */}
                              <div className="mb-4">
                                <p className="text-xs text-muted-foreground mb-2">Components Status</p>
                                <div className="space-y-1">
                                  <ComponentStatusItem
                                    name="DLL"
                                    installed={installation.components.dll}
                                  />
                                  <ComponentStatusItem
                                    name="Master EA"
                                    installed={installation.components.master_ea}
                                  />
                                  <ComponentStatusItem
                                    name="Slave EA"
                                    installed={installation.components.slave_ea}
                                  />
                                </div>
                                <div className="mt-2">
                                  <Badge variant={allComponentsInstalled ? 'default' : 'secondary'}>
                                    {componentStatus.installed}/{componentStatus.total} installed
                                  </Badge>
                                </div>
                              </div>

                              {/* Install Button */}
                              <Button
                                onClick={() => handleInstall(installation)}
                                disabled={isInstalling || allComponentsInstalled}
                                className="w-full gap-2"
                                variant={allComponentsInstalled ? 'outline' : 'default'}
                              >
                                {isInstalling ? (
                                  <>
                                    <Loader2 className="h-4 w-4 animate-spin" />
                                    Installing...
                                  </>
                                ) : allComponentsInstalled ? (
                                  <>
                                    <CheckCircle className="h-4 w-4" />
                                    All Components Installed
                                  </>
                                ) : (
                                  <>
                                    <Download className="h-4 w-4" />
                                    Install Components
                                  </>
                                )}
                              </Button>

                              {installation.is_running && !allComponentsInstalled && (
                                <p className="text-xs text-muted-foreground mt-2 text-center">
                                  Warning: MT is running. Installation may require restart.
                                </p>
                              )}
                            </CardContent>
                          </Card>
                        );
                      })}
                    </div>
                  </div>
                );
              })()}
            </>
          )}
        </div>
      </div>
    </div>
  );
}

function ComponentStatusItem({ name, installed }: { name: string; installed: boolean }) {
  return (
    <div className="flex items-center gap-2 text-sm">
      {installed ? (
        <CheckCircle className="h-4 w-4 text-green-500" />
      ) : (
        <div className="h-4 w-4 rounded-full border-2 border-muted" />
      )}
      <span className={installed ? 'text-foreground' : 'text-muted-foreground'}>{name}</span>
    </div>
  );
}
