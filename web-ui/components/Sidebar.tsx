'use client';

import { useState, useEffect } from 'react';
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { useIntlayer } from 'next-intlayer';
import { cn } from '@/lib/utils';
import { ChevronLeft, ChevronRight, Network, Settings, Globe, Menu, X } from 'lucide-react';
import { Sheet, SheetContent, SheetHeader, SheetTitle, SheetClose } from './ui/sheet';
import { useSidebar } from '@/lib/contexts/sidebar-context';

// Sidebar component with collapsible functionality
// PC: Fixed sidebar (default open), Mobile: Overlay drawer
// State is shared via SidebarContext for Header and ServerLog positioning
export function Sidebar() {
  const content = useIntlayer('sidebar');
  const pathname = usePathname();
  const locale = pathname.split('/')[1] || 'en';
  const { isOpen, isMobile, setIsOpen } = useSidebar();
  const [isMounted, setIsMounted] = useState(false);

  // Hydration fix: Only render after mount
  useEffect(() => {
    setIsMounted(true);
  }, []);

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

  // Prevent hydration mismatch
  if (!isMounted) {
    return null;
  }

  // Mobile: Render as overlay drawer (Sheet)
  if (isMobile) {
    return (
      <>
        {/* Mobile menu toggle button */}
        <button
          onClick={() => setIsOpen(true)}
          className="fixed top-3 left-4 z-40 lg:hidden p-2 rounded-md bg-background border border-border hover:bg-accent transition-colors"
          aria-label={content.openMenu.value}
        >
          <Menu className="h-5 w-5" />
        </button>

        <Sheet open={isOpen} onOpenChange={setIsOpen} side="left">
          <SheetContent>
            <SheetHeader>
              <SheetTitle>{content.menu}</SheetTitle>
              <SheetClose onClose={() => setIsOpen(false)} />
            </SheetHeader>

            <nav className="flex flex-col gap-2 mt-4" aria-label="Main navigation">
              {navItems.map((item) => {
                const Icon = item.icon;
                return (
                  <Link
                    key={item.href}
                    href={item.href}
                    onClick={() => setIsOpen(false)}
                    className={cn(
                      'flex items-center gap-3 px-4 py-3 rounded-md transition-colors',
                      'hover:bg-accent hover:text-accent-foreground',
                      'focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2',
                      item.active && 'bg-accent text-accent-foreground font-medium'
                    )}
                  >
                    <Icon className="h-5 w-5 flex-shrink-0" />
                    <span className="text-sm">{item.label}</span>
                  </Link>
                );
              })}
            </nav>
          </SheetContent>
        </Sheet>
      </>
    );
  }

  // Desktop: Fixed sidebar with collapse functionality
  return (
    <aside
      className={cn(
        'fixed left-0 top-14 bottom-0 z-30 bg-card border-r border-border transition-all duration-300',
        isOpen ? 'w-64' : 'w-16'
      )}
      aria-label="Main navigation"
    >
      {/* Collapse toggle button */}
      <button
        onClick={() => setIsOpen(!isOpen)}
        className={cn(
          'absolute -right-3 top-6 z-40 p-1 rounded-full bg-background border border-border',
          'hover:bg-accent transition-colors shadow-sm',
          'focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2'
        )}
        aria-label={isOpen ? content.collapseSidebar.value : content.expandSidebar.value}
      >
        {isOpen ? (
          <ChevronLeft className="h-4 w-4" />
        ) : (
          <ChevronRight className="h-4 w-4" />
        )}
      </button>

      {/* Navigation */}
      <nav className="flex flex-col gap-2 p-3" aria-label="Main navigation">
        {navItems.map((item) => {
          const Icon = item.icon;
          return (
            <Link
              key={item.href}
              href={item.href}
              className={cn(
                'flex items-center gap-3 px-3 py-3 rounded-md transition-colors',
                'hover:bg-accent hover:text-accent-foreground',
                'focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2',
                item.active && 'bg-accent text-accent-foreground font-medium',
                !isOpen && 'justify-center'
              )}
              title={!isOpen ? item.label : undefined}
            >
              <Icon className="h-5 w-5 flex-shrink-0" />
              {isOpen && <span className="text-sm">{item.label}</span>}
            </Link>
          );
        })}
      </nav>
    </aside>
  );
}
