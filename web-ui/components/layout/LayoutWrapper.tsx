'use client';

// Layout wrapper component with shadcn Sidebar and ServerLog
// Uses SidebarProvider for sidebar state and SidebarInset for main content area
// Header includes SidebarTrigger and dynamic breadcrumb navigation
// ServerLog is rendered outside SidebarInset to span full width at the bottom

import { SidebarProvider, SidebarInset, SidebarTrigger } from '@/components/ui/sidebar';
import { AppSidebar } from './AppSidebar';
import { AppBreadcrumb } from './AppBreadcrumb';
import { ServerLog } from '@/components/features/server-log/ServerLog';
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
          <header className="flex h-16 shrink-0 items-center gap-2 border-b px-4">
            <SidebarTrigger className="-ml-1" />
            <Separator orientation="vertical" className="mr-2 h-4" />
            <AppBreadcrumb />
          </header>
          <div className="flex-1 overflow-hidden pt-2">
            {children}
          </div>
        </SidebarInset>
        <ServerLog />
      </SidebarProvider>
    </ServerLogProvider>
  );
}
