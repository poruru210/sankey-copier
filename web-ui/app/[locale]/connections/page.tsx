'use client';

import { useEffect } from 'react';
import { preconnect } from 'react-dom';
import { useIntlayer } from 'next-intlayer';
import { ConnectionsViewReactFlow } from '@/components/ConnectionsViewReactFlow';
import { ParticlesBackground } from '@/components/ParticlesBackground';
import { useSankeyCopier } from '@/hooks/useSankeyCopier';
import { useSiteContext } from '@/lib/contexts/site-context';
import { useSidebar } from '@/lib/contexts/sidebar-context';
import { cn } from '@/lib/utils';

export default function Home() {
  const content = useIntlayer('connections-page');
  const { selectedSite } = useSiteContext();
  const { isOpen: isSidebarOpen, isMobile, serverLogHeight } = useSidebar();
  const {
    settings,
    connections,
    loading,
    error,
    wsMessages,
    toggleEnabled,
    createSetting,
    updateSetting,
    deleteSetting,
  } = useSankeyCopier();

  // Preconnect to API server for faster initial requests
  useEffect(() => {
    if (selectedSite?.siteUrl) {
      preconnect(selectedSite.siteUrl);
    }
  }, [selectedSite]);

  if (loading && settings.length === 0) {
    return (
      <div className="min-h-screen bg-background flex items-center justify-center">
        <div className="text-xl">{content.loading}</div>
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
          <div className="w-[95%] mx-auto p-4 h-full flex flex-col">
          {/* Page Title */}
          <div className="mb-4">
            <h1 className="text-xl font-bold mb-1">{content.title}</h1>
            <p className="text-sm text-muted-foreground">
              {content.description}
            </p>
          </div>

          {/* Error Display */}
          {error && (
            <div className="bg-destructive/10 border border-destructive text-destructive px-4 py-3 rounded-lg mb-6">
              {error}
            </div>
          )}

          {/* Copy Connections */}
          <div className="flex-1 min-h-0">
            <ConnectionsViewReactFlow
              connections={connections}
              settings={settings}
              onToggle={toggleEnabled}
              onCreate={createSetting}
              onUpdate={updateSetting}
              onDelete={deleteSetting}
            />
          </div>
          </div>
        </div>
      </div>
    </div>
  );
}
