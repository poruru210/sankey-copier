'use client';

// Settings page - VictoriaLogs configuration display and toggle
// Config values are read from config.toml (read-only), only enabled state can be toggled
// This page is only accessible when VictoriaLogs is configured in config.toml

import { useIntlayer } from 'next-intlayer';
import { RefreshCw, Activity, AlertCircle, CheckCircle2, Info, Settings2 } from 'lucide-react';
import { useVLogsConfig } from '@/hooks/useVLogsConfig';
import { useServerLogContext } from '@/lib/contexts/sidebar-context';
import { Typography, Muted } from '@/components/ui/typography';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { useToast } from '@/hooks/use-toast';

export default function SettingsPage() {
  const content = useIntlayer('settings-page');
  const { serverLogHeight } = useServerLogContext();
  const { configured, config, enabled, loading, toggling, error, toggleEnabled, refetch } = useVLogsConfig();
  const { toast } = useToast();

  // Handle toggle
  const handleToggle = async (checked: boolean) => {
    const success = await toggleEnabled(checked);
    if (success) {
      toast({
        title: String(content.toast.toggleSuccess),
        description: checked
          ? String(content.toast.enabledDescription)
          : String(content.toast.disabledDescription),
      });
    } else {
      toast({
        title: String(content.toast.toggleError),
        description: error || String(content.toast.toggleErrorDescription),
        variant: 'destructive',
      });
    }
  };

  if (loading) {
    return (
      <div className="min-h-screen bg-background flex items-center justify-center">
        <Typography variant="large">{content.loading}</Typography>
      </div>
    );
  }

  // Not configured - show info message
  if (!configured) {
    return (
      <div className="h-full bg-background relative overflow-hidden flex flex-col">
        <div
          className="relative z-10 flex flex-col overflow-y-auto"
          style={{
            height: `calc(100% - ${serverLogHeight}px)`,
          }}
        >
          <div className="w-[95%] max-w-4xl mx-auto p-4 h-full flex flex-col">
            {/* Page Title */}
            <div className="mb-6">
              <Typography variant="h3" className="mb-2">{content.title}</Typography>
              <Muted>{content.description}</Muted>
            </div>

            {/* Not Configured Alert */}
            <Alert className="mb-6">
              <Info className="h-4 w-4" />
              <AlertTitle>{content.notConfigured.title}</AlertTitle>
              <AlertDescription>
                {content.notConfigured.description}
              </AlertDescription>
            </Alert>

            <Card>
              <CardHeader>
                <div className="flex items-center gap-2">
                  <Settings2 className="h-5 w-5 text-muted-foreground" />
                  <CardTitle className="text-muted-foreground">{content.vlogs.title}</CardTitle>
                </div>
                <CardDescription>{content.notConfigured.hint}</CardDescription>
              </CardHeader>
              <CardContent>
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
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="h-full bg-background relative overflow-hidden flex flex-col">
      {/* Main Content */}
      <div
        className="relative z-10 flex flex-col overflow-y-auto"
        style={{
          height: `calc(100% - ${serverLogHeight}px)`,
        }}
      >
        <div className="w-[95%] max-w-4xl mx-auto p-4 h-full flex flex-col">
          {/* Page Title */}
          <div className="mb-6">
            <Typography variant="h3" className="mb-2">{content.title}</Typography>
            <Muted>{content.description}</Muted>
          </div>

          {/* Error Display */}
          {error && (
            <Alert variant="destructive" className="mb-6">
              <AlertCircle className="h-4 w-4" />
              <AlertTitle>{content.errorTitle}</AlertTitle>
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          )}

          {/* VictoriaLogs Settings Card */}
          <Card className="mb-6">
            <CardHeader>
              <div className="flex items-center gap-2">
                <Activity className="h-5 w-5" />
                <CardTitle>{content.vlogs.title}</CardTitle>
              </div>
              <CardDescription>{content.vlogs.description}</CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
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
                  onCheckedChange={handleToggle}
                  disabled={toggling}
                />
              </div>

              {/* Read-only info alert */}
              <Alert>
                <Info className="h-4 w-4" />
                <AlertTitle>{content.vlogs.readOnlyTitle}</AlertTitle>
                <AlertDescription>
                  {content.vlogs.readOnlyDescription}
                </AlertDescription>
              </Alert>

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
                <p className="text-sm text-muted-foreground">
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
                <p className="text-sm text-muted-foreground">
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
                <p className="text-sm text-muted-foreground">
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
                <p className="text-sm text-muted-foreground">
                  {content.vlogs.sourceDescription}
                </p>
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

          {/* Refresh Button */}
          <div className="flex justify-end gap-2">
            <Button
              variant="outline"
              onClick={refetch}
              disabled={loading || toggling}
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
