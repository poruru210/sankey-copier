'use client';

// AppSidebar component using shadcn Sidebar
// Combines navigation, logo, and controls (Site selector, Language toggle, Theme toggle)
// Replaces the previous Header + Sidebar combination

import * as React from 'react';
import Image from 'next/image';
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { useIntlayer } from 'next-intlayer';
import { Network, Settings, Globe } from 'lucide-react';

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
} from '@/components/ui/sidebar';
import { SiteSelector } from './SiteSelector';
import { LanguageToggle } from './LanguageToggle';
import { ThemeToggle } from './ThemeToggle';

export function AppSidebar({ ...props }: React.ComponentProps<typeof Sidebar>) {
  const content = useIntlayer('sidebar');
  const pathname = usePathname();
  const locale = pathname.split('/')[1] || 'en';

  // Navigation items
  const navItems = [
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
      href: `/${locale}/sites`,
      icon: Globe,
      label: content.sites,
      active: pathname.includes('/sites'),
    },
  ];

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
              <SiteSelector />
              <div className="flex items-center gap-1 group-data-[collapsible=icon]:flex-col">
                <LanguageToggle />
                <ThemeToggle />
              </div>
            </div>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarFooter>

      <SidebarRail />
    </Sidebar>
  );
}
