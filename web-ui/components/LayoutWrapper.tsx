'use client';

// Layout wrapper component with shadcn Sidebar and ServerLog
// Uses SidebarProvider for sidebar state and SidebarInset for main content area
// ServerLog is rendered outside SidebarInset to span full width at the bottom

import { SidebarProvider, SidebarInset, SidebarTrigger } from '@/components/ui/sidebar';
import { AppSidebar } from './AppSidebar';
import { ServerLog } from './ServerLog';
import { ServerLogProvider } from '@/lib/contexts/sidebar-context';
import { Separator } from '@/components/ui/separator';

interface LayoutWrapperProps {
  children: React.ReactNode;
}

export function LayoutWrapper({ children }: LayoutWrapperProps) {
  return (
    <ServerLogProvider>
      <SidebarProvider>
        <AppSidebar />
        <SidebarInset>
          <header className="flex h-14 shrink-0 items-center gap-2 border-b px-4">
            <SidebarTrigger className="-ml-1" />
            <Separator orientation="vertical" className="mr-2 h-4" />
            <span className="text-sm font-medium">SANKEY Copier</span>
          </header>
          <div className="flex-1 overflow-hidden">
            {children}
          </div>
        </SidebarInset>
        <ServerLog />
      </SidebarProvider>
    </ServerLogProvider>
  );
}
