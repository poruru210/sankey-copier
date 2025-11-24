'use client';

// web-ui/app/[locale]/trade-groups/page.tsx
//
// TradeGroups list page - displays all Master accounts and their configuration settings.
// Allows navigation to individual TradeGroup detail pages for editing settings.

import { useEffect } from 'react';
import { useIntlayer } from 'next-intlayer';
import { useRouter } from 'next/navigation';
import { ParticlesBackground } from '@/components/ParticlesBackground';
import { useTradeGroups } from '@/hooks/useTradeGroups';
import { useSidebar } from '@/lib/contexts/sidebar-context';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { AlertCircle, Edit, Loader2, RefreshCw } from 'lucide-react';
import { cn } from '@/lib/utils';

export default function TradeGroupsPage() {
  const content = useIntlayer('trade-groups-page');
  const { tradeGroups, loading, error, fetchTradeGroups } = useTradeGroups();
  const { isOpen: isSidebarOpen, isMobile, serverLogHeight } = useSidebar();
  const router = useRouter();

  useEffect(() => {
    fetchTradeGroups();
  }, [fetchTradeGroups]);

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

  const handleEdit = (masterAccount: string) => {
    router.push(`/trade-groups/${encodeURIComponent(masterAccount)}`);
  };

  if (loading && tradeGroups.length === 0) {
    return (
      <div className="min-h-screen bg-background flex items-center justify-center">
        <div className="flex flex-col items-center gap-4">
          <Loader2 className="h-8 w-8 animate-spin" />
          <div className="text-xl">{content.loadingTradeGroups}</div>
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
              <h1 className="text-2xl md:text-xl font-bold mb-1">{content.title}</h1>
              <p className="text-sm text-muted-foreground">
                {content.description}
              </p>
            </div>

            {/* Action Buttons */}
            <div className="mb-6 flex gap-3">
              <Button
                onClick={fetchTradeGroups}
                disabled={loading}
                variant="outline"
                className="gap-2 min-h-[44px] md:min-h-0"
              >
                <RefreshCw className={`h-4 w-4 ${loading ? 'animate-spin' : ''}`} />
                更新
              </Button>
            </div>

            {/* Error Display */}
            {error && (
              <div className="bg-destructive/10 border border-destructive text-destructive px-4 py-3 rounded-lg mb-6 flex items-center gap-2">
                <AlertCircle className="h-5 w-5" />
                {error}
              </div>
            )}

            {/* TradeGroups Table */}
            {tradeGroups.length === 0 ? (
              <Card>
                <CardContent className="py-12 text-center">
                  <p className="text-lg text-muted-foreground">
                    {content.noTradeGroupsFound}
                  </p>
                  <p className="text-sm text-muted-foreground mt-2">
                    {content.noTradeGroupsDescription}
                  </p>
                </CardContent>
              </Card>
            ) : (
              <div className="rounded-lg border bg-card overflow-x-auto">
                <Table>
                  <TableHeader>
                    <TableRow className="h-12 md:h-9">
                      <TableHead className="py-2 text-xs">{content.masterAccount}</TableHead>
                      <TableHead className="py-2 text-xs">{content.symbolPrefix}</TableHead>
                      <TableHead className="py-2 text-xs">{content.symbolSuffix}</TableHead>
                      <TableHead className="py-2 text-xs hidden md:table-cell">{content.configVersion}</TableHead>
                      <TableHead className="py-2 text-xs hidden md:table-cell">{content.updatedAt}</TableHead>
                      <TableHead className="py-2 text-xs w-[100px]">{content.actions}</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {tradeGroups.map((tradeGroup) => (
                      <TableRow
                        key={tradeGroup.id}
                        className="h-14 md:h-10"
                      >
                        <TableCell className="font-medium py-2 md:py-1">
                          <span className="text-xs font-mono">{tradeGroup.id}</span>
                        </TableCell>
                        <TableCell className="py-2 md:py-1">
                          {tradeGroup.master_settings.symbol_prefix ? (
                            <span className="text-xs font-mono">{tradeGroup.master_settings.symbol_prefix}</span>
                          ) : (
                            <span className="text-xs text-muted-foreground">{content.notSet}</span>
                          )}
                        </TableCell>
                        <TableCell className="py-2 md:py-1">
                          {tradeGroup.master_settings.symbol_suffix ? (
                            <span className="text-xs font-mono">{tradeGroup.master_settings.symbol_suffix}</span>
                          ) : (
                            <span className="text-xs text-muted-foreground">{content.notSet}</span>
                          )}
                        </TableCell>
                        <TableCell className="py-2 md:py-1 hidden md:table-cell">
                          <span className="text-xs">{tradeGroup.master_settings.config_version}</span>
                        </TableCell>
                        <TableCell className="py-2 md:py-1 hidden md:table-cell">
                          <span className="text-xs text-muted-foreground">
                            {formatDate(tradeGroup.updated_at)}
                          </span>
                        </TableCell>
                        <TableCell className="py-2 md:py-1">
                          <Button
                            onClick={() => handleEdit(tradeGroup.id)}
                            variant="outline"
                            size="sm"
                            className="gap-1 h-8 text-xs"
                          >
                            <Edit className="h-3 w-3" />
                            {content.edit}
                          </Button>
                        </TableCell>
                      </TableRow>
                    ))}
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
