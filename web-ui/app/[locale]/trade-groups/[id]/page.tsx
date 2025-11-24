'use client';

// web-ui/app/[locale]/trade-groups/[id]/page.tsx
//
// TradeGroup detail page - edit Master account configuration settings.
// Displays a form to update symbol_prefix and symbol_suffix for a specific Master account.

import { useEffect, useState } from 'react';
import { useParams, useRouter } from 'next/navigation';
import { useIntlayer } from 'next-intlayer';
import { useAtomValue } from 'jotai';
import { ParticlesBackground } from '@/components/ParticlesBackground';
import { useSidebar } from '@/lib/contexts/sidebar-context';
import { apiClientAtom } from '@/lib/atoms/site';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { AlertCircle, ArrowLeft, CheckCircle, Loader2, Save } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { TradeGroup, MasterSettings } from '@/types';

export default function TradeGroupDetailPage() {
  const params = useParams();
  const router = useRouter();
  const content = useIntlayer('trade-group-detail-page');
  const apiClient = useAtomValue(apiClientAtom);
  const { isOpen: isSidebarOpen, isMobile, serverLogHeight } = useSidebar();

  const masterAccount = decodeURIComponent(params.id as string);

  const [tradeGroup, setTradeGroup] = useState<TradeGroup | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null);

  const [formData, setFormData] = useState({
    symbol_prefix: '',
    symbol_suffix: '',
  });

  useEffect(() => {
    const fetchTradeGroup = async () => {
      setLoading(true);
      setError(null);

      try {
        const data = await apiClient.getTradeGroup(masterAccount);
        setTradeGroup(data);
        setFormData({
          symbol_prefix: data.master_settings.symbol_prefix || '',
          symbol_suffix: data.master_settings.symbol_suffix || '',
        });
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : content.loadFailed.value;
        setError(errorMessage);
        console.error('Error fetching trade group:', err);
      } finally {
        setLoading(false);
      }
    };

    fetchTradeGroup();
  }, [apiClient, masterAccount, content.loadFailed]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setSaving(true);
    setMessage(null);

    try {
      const settings: MasterSettings = {
        symbol_prefix: formData.symbol_prefix || null,
        symbol_suffix: formData.symbol_suffix || null,
        config_version: tradeGroup?.master_settings.config_version || 0,
      };

      await apiClient.updateTradeGroupSettings(masterAccount, settings);

      setMessage({ type: 'success', text: content.saveSuccess.value });

      // Navigate back to list after 1.5 seconds
      setTimeout(() => {
        router.push('/trade-groups');
      }, 1500);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : content.saveFailed.value;
      setMessage({ type: 'error', text: errorMessage });
      console.error('Error updating trade group settings:', err);
    } finally {
      setSaving(false);
    }
  };

  const handleCancel = () => {
    router.push('/trade-groups');
  };

  const formatDate = (dateString: string) => {
    const date = new Date(dateString);
    return date.toLocaleString('ja-JP', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  if (loading) {
    return (
      <div className="min-h-screen bg-background flex items-center justify-center">
        <div className="flex flex-col items-center gap-4">
          <Loader2 className="h-8 w-8 animate-spin" />
          <div className="text-xl">{content.loadingTradeGroup}</div>
        </div>
      </div>
    );
  }

  if (error || !tradeGroup) {
    return (
      <div className="min-h-screen bg-background flex items-center justify-center">
        <Card className="w-full max-w-md">
          <CardHeader>
            <CardTitle className="flex items-center gap-2 text-destructive">
              <AlertCircle className="h-5 w-5" />
              {content.notFound}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-sm text-muted-foreground mb-4">
              {error || content.loadFailed.value}
            </p>
            <Button onClick={handleCancel} variant="outline">
              <ArrowLeft className="h-4 w-4 mr-2" />
              {content.backToList}
            </Button>
          </CardContent>
        </Card>
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
          <div className="w-[95%] max-w-2xl mx-auto p-4">
            {/* Back Button */}
            <Button
              onClick={handleCancel}
              variant="ghost"
              className="mb-4 gap-2"
            >
              <ArrowLeft className="h-4 w-4" />
              {content.backToList}
            </Button>

            {/* Page Title */}
            <div className="mb-6">
              <h1 className="text-2xl md:text-xl font-bold mb-1">{content.title}</h1>
              <p className="text-sm text-muted-foreground">
                {content.description}
              </p>
            </div>

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

            {/* Settings Form */}
            <Card>
              <CardHeader>
                <CardTitle>{tradeGroup.id}</CardTitle>
                <CardDescription>
                  {content.configVersionLabel}: {tradeGroup.master_settings.config_version} |{' '}
                  {content.lastUpdatedLabel}: {formatDate(tradeGroup.updated_at)}
                </CardDescription>
              </CardHeader>
              <CardContent>
                <form onSubmit={handleSubmit} className="space-y-6">
                  {/* Symbol Prefix */}
                  <div className="space-y-2">
                    <Label htmlFor="symbol_prefix">{content.symbolPrefixLabel}</Label>
                    <Input
                      id="symbol_prefix"
                      type="text"
                      placeholder={content.symbolPrefixPlaceholder.value}
                      value={formData.symbol_prefix}
                      onChange={(e) => setFormData({ ...formData, symbol_prefix: e.target.value })}
                      disabled={saving}
                    />
                    <p className="text-xs text-muted-foreground">
                      {content.symbolPrefixDescription}
                    </p>
                  </div>

                  {/* Symbol Suffix */}
                  <div className="space-y-2">
                    <Label htmlFor="symbol_suffix">{content.symbolSuffixLabel}</Label>
                    <Input
                      id="symbol_suffix"
                      type="text"
                      placeholder={content.symbolSuffixPlaceholder.value}
                      value={formData.symbol_suffix}
                      onChange={(e) => setFormData({ ...formData, symbol_suffix: e.target.value })}
                      disabled={saving}
                    />
                    <p className="text-xs text-muted-foreground">
                      {content.symbolSuffixDescription}
                    </p>
                  </div>

                  {/* Action Buttons */}
                  <div className="flex gap-3 pt-4">
                    <Button
                      type="submit"
                      disabled={saving}
                      className="gap-2"
                    >
                      {saving ? (
                        <>
                          <Loader2 className="h-4 w-4 animate-spin" />
                          {content.saving}
                        </>
                      ) : (
                        <>
                          <Save className="h-4 w-4" />
                          {content.save}
                        </>
                      )}
                    </Button>
                    <Button
                      type="button"
                      variant="outline"
                      onClick={handleCancel}
                      disabled={saving}
                    >
                      {content.cancel}
                    </Button>
                  </div>
                </form>
              </CardContent>
            </Card>
          </div>
        </div>
      </div>
    </div>
  );
}
