'use client';

// Settings page - global system settings including VictoriaLogs configuration
// Settings are applied to all connected EAs via ZMQ broadcast

import { useState, useEffect } from 'react';
import { useIntlayer } from 'next-intlayer';
import { Save, RefreshCw, Activity, AlertCircle, CheckCircle2 } from 'lucide-react';
import { useVLogsSettings, type VLogsGlobalSettings } from '@/hooks/useVLogsSettings';
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
  const { settings, loading, saving, error, saveSettings, refetch } = useVLogsSettings();
  const { toast } = useToast();

  // Local form state
  const [formData, setFormData] = useState<VLogsGlobalSettings>({
    enabled: false,
    endpoint: '',
    batch_size: 100,
    flush_interval_secs: 5,
  });
  const [isDirty, setIsDirty] = useState(false);

  // Sync form with loaded settings
  useEffect(() => {
    setFormData(settings);
    setIsDirty(false);
  }, [settings]);

  // Check if form has unsaved changes
  useEffect(() => {
    const hasChanges =
      formData.enabled !== settings.enabled ||
      formData.endpoint !== settings.endpoint ||
      formData.batch_size !== settings.batch_size ||
      formData.flush_interval_secs !== settings.flush_interval_secs;
    setIsDirty(hasChanges);
  }, [formData, settings]);

  // Handle form changes
  const handleChange = (field: keyof VLogsGlobalSettings, value: boolean | string | number) => {
    setFormData((prev) => ({ ...prev, [field]: value }));
  };

  // Handle save
  const handleSave = async () => {
    const success = await saveSettings(formData);
    if (success) {
      toast({
        title: String(content.toast.saveSuccess),
        description: String(content.toast.saveSuccessDescription),
      });
    } else {
      toast({
        title: String(content.toast.saveError),
        description: error || String(content.toast.saveErrorDescription),
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
              {/* Enabled Toggle */}
              <div className="flex items-center justify-between">
                <div className="space-y-0.5">
                  <Label htmlFor="vlogs-enabled">{content.vlogs.enabled}</Label>
                  <p className="text-sm text-muted-foreground">
                    {content.vlogs.enabledDescription}
                  </p>
                </div>
                <Switch
                  id="vlogs-enabled"
                  checked={formData.enabled}
                  onCheckedChange={(checked) => handleChange('enabled', checked)}
                />
              </div>

              {/* Endpoint URL */}
              <div className="space-y-2">
                <Label htmlFor="vlogs-endpoint">{content.vlogs.endpoint}</Label>
                <Input
                  id="vlogs-endpoint"
                  type="url"
                  placeholder="http://localhost:9428/insert/jsonline"
                  value={formData.endpoint}
                  onChange={(e) => handleChange('endpoint', e.target.value)}
                  disabled={!formData.enabled}
                />
                <p className="text-sm text-muted-foreground">
                  {content.vlogs.endpointDescription}
                </p>
              </div>

              {/* Batch Size */}
              <div className="space-y-2">
                <Label htmlFor="vlogs-batch-size">{content.vlogs.batchSize}</Label>
                <Input
                  id="vlogs-batch-size"
                  type="number"
                  min={1}
                  max={10000}
                  value={formData.batch_size}
                  onChange={(e) => handleChange('batch_size', parseInt(e.target.value) || 1)}
                  disabled={!formData.enabled}
                />
                <p className="text-sm text-muted-foreground">
                  {content.vlogs.batchSizeDescription}
                </p>
              </div>

              {/* Flush Interval */}
              <div className="space-y-2">
                <Label htmlFor="vlogs-flush-interval">{content.vlogs.flushInterval}</Label>
                <Input
                  id="vlogs-flush-interval"
                  type="number"
                  min={1}
                  max={3600}
                  value={formData.flush_interval_secs}
                  onChange={(e) => handleChange('flush_interval_secs', parseInt(e.target.value) || 1)}
                  disabled={!formData.enabled}
                />
                <p className="text-sm text-muted-foreground">
                  {content.vlogs.flushIntervalDescription}
                </p>
              </div>

              {/* Status Indicator */}
              {formData.enabled && formData.endpoint && (
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

          {/* Action Buttons */}
          <div className="flex justify-end gap-2">
            <Button
              variant="outline"
              onClick={refetch}
              disabled={loading || saving}
            >
              <RefreshCw className="mr-2 h-4 w-4" />
              {content.buttons.refresh}
            </Button>
            <Button
              onClick={handleSave}
              disabled={!isDirty || saving}
            >
              <Save className="mr-2 h-4 w-4" />
              {saving ? content.buttons.saving : content.buttons.save}
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
