'use client';

import { ConnectionsView } from '@/components/ConnectionsView';
import { ActivityLog } from '@/components/ActivityLog';
import { Header } from '@/components/Header';
import { ParticlesBackground } from '@/components/ParticlesBackground';
import { useForexCopier } from '@/hooks/useForexCopier';

export default function Home() {
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
  } = useForexCopier();

  if (loading && settings.length === 0) {
    return (
      <div className="min-h-screen bg-background flex items-center justify-center">
        <div className="text-xl">Loading...</div>
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
        <div className="container mx-auto p-6 max-w-[1600px]">
          {/* Error Display */}
          {error && (
            <div className="bg-destructive/10 border border-destructive text-destructive px-4 py-3 rounded-lg mb-6">
              {error}
            </div>
          )}

          {/* Copy Connections */}
          <ConnectionsView
            connections={connections}
            settings={settings}
            onToggle={toggleEnabled}
            onCreate={createSetting}
            onUpdate={updateSetting}
            onDelete={deleteSetting}
          />

          {/* Real-time Activity */}
          <ActivityLog messages={wsMessages} />
        </div>
      </div>
    </div>
  );
}
