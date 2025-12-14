'use client';

// Settings page - Centralized configuration and management
// Integrates:
// 1. Unified Site Management (Select/Connect/Add/Edit/Delete)
// 2. VictoriaLogs configuration (Read-only status, Enabled toggle, Log Level)
// 3. ZeroMQ configuration (Read-only status)

import { useState } from 'react';
import { useIntlayer } from 'next-intlayer';
import { RefreshCw, Activity, AlertCircle, CheckCircle2, Info, Settings2, Radio, Globe, Plus, Trash2, Edit2 } from 'lucide-react';
import { useAtom } from 'jotai';
import { useVLogsConfig } from '@/hooks/useVLogsConfig';
import { useZeromqConfig } from '@/hooks/useZeromqConfig';
import { Typography, Muted } from '@/components/ui/typography';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { useToast } from '@/hooks/use-toast';
import { sitesAtom, selectedSiteIdAtom } from '@/lib/atoms/site';
import { Site } from '@/lib/types/site';

export default function SettingsPage() {
  const content = useIntlayer('settings-page');
  const { configured, config, enabled, loading, updating, error, updateSettings, refetch } = useVLogsConfig();
  const { config: zmqConfig, loading: zmqLoading, error: zmqError, refetch: zmqRefetch } = useZeromqConfig();
  const { toast } = useToast();

  // Site Management State
  const [sites, setSites] = useAtom(sitesAtom);
  const [selectedSiteId, setSelectedSiteId] = useAtom(selectedSiteIdAtom);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [formData, setFormData] = useState({ name: '', siteUrl: '' });
  const [siteError, setSiteError] = useState<string>('');
  const [isListEditing, setIsListEditing] = useState(false);

  // --- Site Management Handlers ---

  const handleStartEdit = (site: Site) => {
    setEditingId(site.id);
    setFormData({ name: site.name, siteUrl: site.siteUrl });
    setSiteError('');
  };

  const handleCancelEdit = () => {
    setEditingId(null);
    setFormData({ name: '', siteUrl: '' });
    setSiteError('');
  };

  const handleSaveSite = () => {
    // Validate
    if (!formData.name.trim()) {
      setSiteError(content.sites.errorSiteNameRequired.value);
      return;
    }
    if (!formData.siteUrl.trim()) {
      setSiteError(content.sites.errorSiteUrlRequired.value);
      return;
    }

    // Validate URL format
    try {
      new URL(formData.siteUrl);
    } catch {
      setSiteError(content.sites.errorInvalidUrl.value);
      return;
    }

    if (editingId && editingId !== 'new') {
      // Update existing site
      setSites((prev) =>
        prev.map((site) =>
          site.id === editingId
            ? { ...site, name: formData.name.trim(), siteUrl: formData.siteUrl.trim() }
            : site
        )
      );
    } else {
      // Add new site
      const newSite: Site = {
        id: crypto.randomUUID(),
        name: formData.name.trim(),
        siteUrl: formData.siteUrl.trim(),
      };
      setSites((prev) => [...prev, newSite]);
      // Do not automatically select the newly created site
      // setSelectedSiteId(newSite.id);
    }

    // Reset form
    handleCancelEdit();
  };

  const handleDeleteSite = (site: Site) => {
    if (sites.length === 1) {
      setSiteError(content.sites.errorCannotDeleteLast.value);
      return;
    }

    const confirmMessage = content.sites.confirmDelete.value.replace('{siteName}', site.name);
    if (window.confirm(confirmMessage)) {
      setSites((prev) => prev.filter((s) => s.id !== site.id));

      // If deleted site was selected, select the first available site
      if (selectedSiteId === site.id) {
        const remainingSites = sites.filter((s) => s.id !== site.id);
        if (remainingSites.length > 0) {
          setSelectedSiteId(remainingSites[0].id);
        }
      }
      setSiteError('');
    }
  };

  const toggleListEditing = () => {
    setIsListEditing(!isListEditing);
    // Reset any individual edit mode
    if (!isListEditing && editingId) {
      setEditingId(null);
    }
  };

  const handleStartAddSite = () => {
    setEditingId('new');
    setFormData({ name: '', siteUrl: '' });
    setSiteError('');
  };

  // --- VictoriaLogs Handlers ---

  const handleUpdateVLogs = async (updates: { enabled?: boolean; log_level?: string }) => {
    // Optimistic check: if toggling enable, use updates.enabled, else use current enabled state
    const isEnabling = updates.enabled !== undefined ? updates.enabled : enabled;

    const success = await updateSettings(updates);

    if (success) {
      toast({
        title: content.toast.toggleSuccess.value,
        description: isEnabling
          ? content.toast.enabledDescription.value
          : content.toast.disabledDescription.value,
      });
    } else {
      toast({
        title: content.toast.toggleError.value,
        description: error || content.toast.toggleErrorDescription.value,
        variant: 'destructive',
      });
    }
  };

  if (loading && zmqLoading) {
    return (
      <div className="min-h-screen bg-background flex items-center justify-center">
        <Typography variant="large">{content.loading}</Typography>
      </div>
    );
  }

  return (
    <div className="h-full bg-background relative overflow-hidden flex flex-col">
      {/* Main Content */}
      <div className="relative z-10 flex flex-col overflow-y-auto h-full">
        <div className="w-[95%] mx-auto p-4 h-full flex flex-col space-y-6">
          {/* Page Title */}
          <div>
            <Typography variant="h3" className="mb-2">{content.title}</Typography>
            <Muted>{content.description}</Muted>
          </div>

          {/* === Unified Server Connections Card === */}
          <Card>
            <CardHeader>
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <Globe className="h-5 w-5" />
                  <CardTitle>{content.sites.sitesTitle}</CardTitle>
                </div>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={toggleListEditing}
                  disabled={editingId === 'new'}
                  className="gap-2"
                >
                  {isListEditing ? (
                    <>
                      <CheckCircle2 className="h-4 w-4" />
                      {content.sites.done}
                    </>
                  ) : (
                    <>
                      <Settings2 className="h-4 w-4" />
                      {content.sites.manageSites}
                    </>
                  )}
                </Button>
              </div>
              <CardDescription>{content.sites.infoMessage}</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              {/* Site Error */}
              {siteError && (
                <Alert variant="destructive">
                  <AlertCircle className="h-4 w-4" />
                  <AlertDescription>{siteError}</AlertDescription>
                </Alert>
              )}

              {/* Site List */}
              <div className="space-y-3">
                {sites.map((site) => (
                  <div
                    key={site.id}
                    className={`p-3 rounded-lg border transition-colors ${site.id === selectedSiteId
                      ? 'border-blue-500 bg-blue-50 dark:bg-blue-950/30'
                      : 'border-border bg-card'
                      }`}
                  >
                    {editingId === site.id ? (
                      // Edit Form for individual item
                      <div className="space-y-3">
                        <div className="grid gap-3">
                          <div>
                            <Label htmlFor={`name-${site.id}`} className="text-xs mb-1 block">
                              {content.sites.siteName}
                            </Label>
                            <Input
                              id={`name-${site.id}`}
                              value={formData.name}
                              onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                              placeholder={content.sites.siteNamePlaceholder.value}
                              className="h-8"
                            />
                          </div>
                          <div>
                            <Label htmlFor={`url-${site.id}`} className="text-xs mb-1 block">
                              {content.sites.siteUrl}
                            </Label>
                            <Input
                              id={`url-${site.id}`}
                              value={formData.siteUrl}
                              onChange={(e) => setFormData({ ...formData, siteUrl: e.target.value })}
                              placeholder={content.sites.siteUrlPlaceholder.value}
                              className="h-8"
                            />
                          </div>
                        </div>
                        <div className="flex gap-2 justify-end">
                          <Button
                            type="button"
                            size="sm"
                            variant="ghost"
                            onClick={handleCancelEdit}
                            className="h-8 text-xs"
                          >
                            {content.sites.cancel}
                          </Button>
                          <Button type="button" size="sm" onClick={handleSaveSite} className="h-8 text-xs">
                            {content.sites.save}
                          </Button>
                        </div>
                      </div>
                    ) : (
                      // Display Mode
                      <div className="flex items-center justify-between gap-3">
                        <div className="flex-1 min-w-0">
                          <div className="flex items-center gap-2">
                            <h4 className="font-semibold text-sm">{site.name}</h4>
                            {site.id === selectedSiteId && (
                              <span className="text-[10px] px-1.5 py-0.5 rounded-full bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-300 font-medium border border-blue-200 dark:border-blue-800">
                                {content.sites.selected}
                              </span>
                            )}
                          </div>
                          <p className="text-xs text-muted-foreground mt-0.5 truncate">
                            {site.siteUrl}
                          </p>
                        </div>

                        <div className="flex gap-1 flex-shrink-0 items-center">
                          {isListEditing ? (
                            // Edit Mode Controls
                            <>
                              <Button
                                type="button"
                                size="sm"
                                variant="ghost"
                                onClick={() => handleStartEdit(site)}
                                className="h-8 w-8 p-0"
                              >
                                <Edit2 className="h-3.5 w-3.5" />
                              </Button>
                              <Button
                                type="button"
                                size="sm"
                                variant="ghost"
                                onClick={() => handleDeleteSite(site)}
                                disabled={sites.length === 1}
                                className="h-8 w-8 p-0 text-red-600 hover:text-red-700 hover:bg-red-50 dark:hover:bg-red-950/30 disabled:opacity-50"
                              >
                                <Trash2 className="h-3.5 w-3.5" />
                              </Button>
                            </>
                          ) : (
                            // Select Mode Controls
                            site.id !== selectedSiteId && (
                              <Button
                                size="sm"
                                variant="outline"
                                onClick={() => setSelectedSiteId(site.id)}
                                className="h-8 text-xs"
                              >
                                {content.sites.connect}
                              </Button>
                            )
                          )}
                        </div>
                      </div>
                    )}
                  </div>
                ))}

                {/* Add New Site - only visible in List Edit Mode or when adding */}
                {(isListEditing || editingId === 'new') && (
                  editingId === 'new' ? (
                    <div className="p-3 rounded-lg border border-border bg-card">
                      <div className="space-y-3">
                        <h4 className="font-semibold text-sm">{content.sites.addNewSite}</h4>
                        <div className="grid gap-3">
                          <div>
                            <Label htmlFor="name-new" className="text-xs mb-1 block">
                              {content.sites.siteName}
                            </Label>
                            <Input
                              id="name-new"
                              value={formData.name}
                              onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                              placeholder={content.sites.siteNamePlaceholder.value}
                              className="h-8"
                            />
                          </div>
                          <div>
                            <Label htmlFor="url-new" className="text-xs mb-1 block">
                              {content.sites.siteUrl}
                            </Label>
                            <Input
                              id="url-new"
                              value={formData.siteUrl}
                              onChange={(e) => setFormData({ ...formData, siteUrl: e.target.value })}
                              placeholder={content.sites.siteUrlPlaceholder.value}
                              className="h-8"
                            />
                          </div>
                        </div>
                        <div className="flex gap-2 justify-end">
                          <Button
                            type="button"
                            size="sm"
                            variant="ghost"
                            onClick={handleCancelEdit}
                            className="h-8 text-xs"
                          >
                            {content.sites.cancel}
                          </Button>
                          <Button type="button" size="sm" onClick={handleSaveSite} className="h-8 text-xs">
                            {content.sites.add}
                          </Button>
                        </div>
                      </div>
                    </div>
                  ) : (
                    <Button
                      variant="outline"
                      className="w-full border-dashed"
                      onClick={handleStartAddSite}
                    >
                      <Plus className="mr-2 h-4 w-4" />
                      {content.sites.addNewSite}
                    </Button>
                  )
                )}
              </div>
            </CardContent>
          </Card>

          {/* === Section 3: VictoriaLogs Settings === */}
          {configured ? (
            <Card>
              <CardHeader>
                <div className="flex items-center gap-2">
                  <Activity className="h-5 w-5" />
                  <CardTitle>{content.vlogs.title}</CardTitle>
                </div>
                <CardDescription>{content.vlogs.description}</CardDescription>
              </CardHeader>
              <CardContent className="space-y-6">
                {/* VLogs Error Display */}
                {error && (
                  <Alert variant="destructive">
                    <AlertCircle className="h-4 w-4" />
                    <AlertTitle>{content.errorTitle}</AlertTitle>
                    <AlertDescription>
                      {error}
                      {error.includes('fetch') && (
                        <span className="block mt-1 text-xs opacity-90">
                          {content.sites.errorInvalidUrl.value.split('(')[0]}
                        </span>
                      )}
                    </AlertDescription>
                  </Alert>
                )}

                {/* Enabled Toggle - only editable field */}
                <div className="flex items-center justify-between">
                  <div className="space-y-0.5">
                    <Label htmlFor="vlogs-enabled">{content.vlogs.enabled}</Label>
                    <p className="text-sm text-muted-foreground">
                      {content.vlogs.enabledDescription}
                    </p>
                  </div>
                  <Switch
                    id="vlogs-enabled"
                    checked={enabled}
                    onCheckedChange={(checked) => handleUpdateVLogs({ enabled: checked })}
                    disabled={updating}
                  />
                </div>

                {/* Log Level Selector */}
                <div className="space-y-2">
                  <Label htmlFor="vlogs-loglevel">{content.vlogs.logLevel}</Label>
                  <div className="flex items-center gap-4">
                    <Select
                      value={config?.log_level || 'INFO'}
                      onValueChange={(value) => handleUpdateVLogs({ log_level: value })}
                      disabled={!enabled || updating}
                    >
                      <SelectTrigger id="vlogs-loglevel" className="w-[180px]">
                        <SelectValue placeholder="Select log level" />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="DEBUG">{content.vlogs.levelDebug}</SelectItem>
                        <SelectItem value="INFO">{content.vlogs.levelInfo}</SelectItem>
                        <SelectItem value="WARN">{content.vlogs.levelWarn}</SelectItem>
                        <SelectItem value="ERROR">{content.vlogs.levelError}</SelectItem>
                      </SelectContent>
                    </Select>
                    <p className="text-sm text-muted-foreground flex-1">
                      {content.vlogs.logLevelDescription}
                    </p>
                  </div>
                </div>

                {/* Read-only info alert */}
                <Alert>
                  <Info className="h-4 w-4" />
                  <AlertTitle>{content.vlogs.readOnlyTitle}</AlertTitle>
                  <AlertDescription>
                    {content.vlogs.readOnlyDescription}
                  </AlertDescription>
                </Alert>

                <div className="grid gap-4 md:grid-cols-2">
                  {/* Host URL - read-only */}
                  <div className="space-y-2">
                    <Label htmlFor="vlogs-host">{content.vlogs.host}</Label>
                    <Input
                      id="vlogs-host"
                      type="url"
                      value={config?.host || ''}
                      disabled
                      className="bg-muted"
                    />
                    <p className="text-xs text-muted-foreground">
                      {content.vlogs.hostDescription}
                    </p>
                  </div>

                  {/* Batch Size - read-only */}
                  <div className="space-y-2">
                    <Label htmlFor="vlogs-batch-size">{content.vlogs.batchSize}</Label>
                    <Input
                      id="vlogs-batch-size"
                      type="number"
                      value={config?.batch_size || 0}
                      disabled
                      className="bg-muted"
                    />
                    <p className="text-xs text-muted-foreground">
                      {content.vlogs.batchSizeDescription}
                    </p>
                  </div>

                  {/* Flush Interval - read-only */}
                  <div className="space-y-2">
                    <Label htmlFor="vlogs-flush-interval">{content.vlogs.flushInterval}</Label>
                    <Input
                      id="vlogs-flush-interval"
                      type="number"
                      value={config?.flush_interval_secs || 0}
                      disabled
                      className="bg-muted"
                    />
                    <p className="text-xs text-muted-foreground">
                      {content.vlogs.flushIntervalDescription}
                    </p>
                  </div>

                  {/* Source - read-only */}
                  <div className="space-y-2">
                    <Label htmlFor="vlogs-source">{content.vlogs.source}</Label>
                    <Input
                      id="vlogs-source"
                      type="text"
                      value={config?.source || ''}
                      disabled
                      className="bg-muted"
                    />
                    <p className="text-xs text-muted-foreground">
                      {content.vlogs.sourceDescription}
                    </p>
                  </div>
                </div>

                {/* Status Indicator */}
                {enabled && config?.host && (
                  <Alert>
                    <CheckCircle2 className="h-4 w-4" />
                    <AlertTitle>{content.vlogs.statusActive}</AlertTitle>
                    <AlertDescription>
                      {content.vlogs.statusActiveDescription}
                    </AlertDescription>
                  </Alert>
                )}
              </CardContent>
            </Card>
          ) : (
            // Not Configured State
            <Card>
              <CardHeader>
                <div className="flex items-center gap-2">
                  <Activity className="h-5 w-5 text-muted-foreground" />
                  <CardTitle className="text-muted-foreground">{content.vlogs.title}</CardTitle>
                </div>
                <CardDescription>{content.notConfigured.description}</CardDescription>
              </CardHeader>
              <CardContent>
                {/* Show error even if not configured, as it might be why it appears not configured */}
                {error && (
                  <Alert variant="destructive" className="mb-4">
                    <AlertCircle className="h-4 w-4" />
                    <AlertTitle>{content.errorTitle}</AlertTitle>
                    <AlertDescription>{error}</AlertDescription>
                  </Alert>
                )}

                <Alert className="mb-4">
                  <Info className="h-4 w-4" />
                  <AlertTitle>{content.notConfigured.title}</AlertTitle>
                  <AlertDescription>
                    {content.notConfigured.hint}
                  </AlertDescription>
                </Alert>
                <pre className="p-4 rounded-md bg-muted text-sm font-mono overflow-x-auto">
                  {`[victoria_logs]
enabled = true
endpoint = "http://localhost:9428/insert/jsonline"
batch_size = 100
flush_interval_secs = 5
source = "sankey-copier"`}
                </pre>
              </CardContent>
            </Card>
          )}

          {/* === Section 4: ZeroMQ Settings === */}
          <Card>
            <CardHeader>
              <div className="flex items-center gap-2">
                <Radio className="h-5 w-5" />
                <CardTitle>{content.zeromq.title}</CardTitle>
              </div>
              <CardDescription>{content.zeromq.description}</CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
              {/* ZeroMQ Error Display */}
              {zmqError && (
                <Alert variant="destructive">
                  <AlertCircle className="h-4 w-4" />
                  <AlertTitle>{content.errorTitle}</AlertTitle>
                  <AlertDescription>{zmqError}</AlertDescription>
                </Alert>
              )}

              {/* Port mode indicator */}
              <div className="flex items-center gap-2">
                {zmqConfig.is_dynamic ? (
                  <Alert>
                    <CheckCircle2 className="h-4 w-4" />
                    <AlertTitle>{content.zeromq.isDynamic}</AlertTitle>
                    <AlertDescription>
                      {content.zeromq.isDynamicDescription}
                      {zmqConfig.generated_at && (
                        <span className="block mt-1 text-xs">
                          {content.zeromq.generatedAt}: {new Date(zmqConfig.generated_at).toLocaleString()}
                        </span>
                      )}
                    </AlertDescription>
                  </Alert>
                ) : (
                  <Alert>
                    <Info className="h-4 w-4" />
                    <AlertTitle>{content.zeromq.isFixed}</AlertTitle>
                    <AlertDescription>
                      {content.zeromq.isFixedDescription}
                    </AlertDescription>
                  </Alert>
                )}
              </div>

              <div className="grid gap-4 md:grid-cols-2">
                {/* Receiver Port - read-only */}
                <div className="space-y-2">
                  <Label htmlFor="zmq-receiver-port-main">{content.zeromq.receiverPort}</Label>
                  <Input
                    id="zmq-receiver-port-main"
                    type="number"
                    value={zmqConfig.receiver_port}
                    disabled
                    className="bg-muted"
                  />
                  <p className="text-xs text-muted-foreground">
                    {content.zeromq.receiverPortDescription}
                  </p>
                </div>

                {/* Sender Port - read-only (unified PUB for trades and configs) */}
                <div className="space-y-2">
                  <Label htmlFor="zmq-sender-port-main">{content.zeromq.senderPort}</Label>
                  <Input
                    id="zmq-sender-port-main"
                    type="number"
                    value={zmqConfig.sender_port}
                    disabled
                    className="bg-muted"
                  />
                  <p className="text-xs text-muted-foreground">
                    {content.zeromq.senderPortDescription}
                  </p>
                </div>
              </div>

              <div className="mt-4">
                <Alert>
                  <Info className="h-4 w-4" />
                  <AlertTitle>{content.zeromq.readOnlyTitle}</AlertTitle>
                  <AlertDescription>
                    {content.zeromq.readOnlyDescription}
                  </AlertDescription>
                </Alert>
              </div>
            </CardContent>
          </Card>

          {/* Refresh Button - Fixed at bottomright or end of content */}
          <div className="flex justify-end gap-2 pb-8">
            <Button
              variant="outline"
              onClick={() => { refetch(); zmqRefetch(); }}
              disabled={loading || updating || zmqLoading}
            >
              <RefreshCw className="mr-2 h-4 w-4" />
              {content.buttons.refresh}
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
