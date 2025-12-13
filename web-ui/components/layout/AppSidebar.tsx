'use client';

// AppSidebar component using shadcn Sidebar
// Combines navigation, logo, and controls (Site selector, Language toggle, Theme toggle)
// Settings nav is conditionally shown based on VictoriaLogs configuration

import * as React from 'react';
import Image from 'next/image';
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { useIntlayer } from 'next-intlayer';
import { Network, Settings, Globe, Cog } from 'lucide-react';

import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarRail,
  useSidebar,
} from '@/components/ui/sidebar';
import { LanguageToggle } from './LanguageToggle';
import { ThemeToggle } from './ThemeToggle';
import { useVLogsConfig } from '@/hooks/useVLogsConfig';
import { useServerLogContext } from '@/lib/contexts/sidebar-context';
import { Button } from '@/components/ui/button';
import { SquareTerminal } from 'lucide-react';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';

function ServerLogToggle() {
  const { setServerLogExpanded } = useServerLogContext();
  const { state } = useSidebar();
  const isCollapsed = state === 'collapsed';

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            variant="ghost"
            size="icon"
            onClick={() => setServerLogExpanded(true)}
            className="h-9 w-9"
          >
            <SquareTerminal className="h-[1.2rem] w-[1.2rem] transition-all" />
            <span className="sr-only">Toggle Server Logs</span>
          </Button>
        </TooltipTrigger>
        <TooltipContent side="right">
          <p>Server Logs</p>
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}

export function AppSidebar({ ...props }: React.ComponentProps<typeof Sidebar>) {
  const content = useIntlayer('sidebar');
  const pathname = usePathname();
  const locale = pathname.split('/')[1] || 'en';

  // Check if VictoriaLogs is configured to show/hide Settings nav
  const { configured: vlogsConfigured, loading: vlogsLoading } = useVLogsConfig();

  // Navigation items - Settings is conditionally shown based on VictoriaLogs config
  const navItems = React.useMemo(() => {
    const items = [
      {
        href: `/${locale}/connections`,
        icon: Network,
        label: content.connections,
        active: pathname === `/${locale}/connections` || pathname === `/${locale}`,
      },
      {
        href: `/${locale}/installations`,
        icon: Settings,
        label: content.installations,
        active: pathname.includes('/installations'),
      },
      {
        href: `/${locale}/settings`,
        icon: Cog,
        label: content.settings,
        active: pathname.includes('/settings'),
      },
    ];

    return items;
  }, [locale, pathname, content]);

  return (
    <Sidebar collapsible="icon" {...props}>
      {/* Header with logo */}
      <SidebarHeader>
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton
              size="lg"
              className="data-[state=open]:bg-sidebar-accent data-[state=open]:text-sidebar-accent-foreground"
              asChild
            >
              <Link href={`/${locale}/connections`}>
                <div className="flex aspect-square size-8 items-center justify-center rounded-lg">
                  <Image
                    src="/logo.png"
                    alt="SANKEY Copier Logo"
                    width={32}
                    height={32}
                    priority
                    className="object-contain"
                  />
                </div>
                <div className="grid flex-1 text-left text-sm leading-tight">
                  <span className="truncate font-semibold">SANKEY Copier</span>
                </div>
              </Link>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarHeader>

      {/* Main navigation */}
      <SidebarContent>
        <SidebarGroup>
          <SidebarGroupContent>
            <SidebarMenu>
              {navItems.map((item) => (
                <SidebarMenuItem key={item.href}>
                  <SidebarMenuButton
                    asChild
                    isActive={item.active}
                    tooltip={String(item.label)}
                  >
                    <Link href={item.href}>
                      <item.icon />
                      <span>{item.label}</span>
                    </Link>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              ))}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>

      {/* Footer with controls */}
      <SidebarFooter>
        <SidebarMenu>
          <SidebarMenuItem>
            <div className="flex items-center gap-1 px-2 py-1 group-data-[collapsible=icon]:flex-col group-data-[collapsible=icon]:gap-2">
              <div className="flex items-center gap-1 group-data-[collapsible=icon]:flex-col">
                <LanguageToggle />
                <ThemeToggle />
                <ServerLogToggle />
              </div>
            </div>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarFooter>

      <SidebarRail />
    </Sidebar>
  );
}
