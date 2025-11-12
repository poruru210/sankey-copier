'use client';

import Image from 'next/image';
import { SiteSelector } from './SiteSelector';
import { LanguageToggle } from './LanguageToggle';
import { ThemeToggle } from './ThemeToggle';
import { useSidebar } from '@/lib/contexts/sidebar-context';
import { cn } from '@/lib/utils';

// Header component with logo and controls
// Full-width header with logo section matching sidebar width
// Navigation moved to Sidebar component
// Site management moved to Sites page
export function Header() {
  const { isOpen, isMobile } = useSidebar();

  return (
    <header className="sticky top-0 z-50 w-full border-b bg-background">
      <div className="flex h-14 items-center">
        {/* Logo section - matches sidebar width */}
        {!isMobile && (
          <div
            className={cn(
              'flex items-center justify-center h-full border-r flex-shrink-0 transition-all duration-300',
              isOpen ? 'w-64 gap-2 px-6' : 'w-16 px-2'
            )}
          >
            <Image
              src="/logo.png"
              alt="SANKEY Copier Logo"
              width={32}
              height={32}
              className="object-contain flex-shrink-0"
            />
            {isOpen && (
              <h1 className="text-lg font-semibold text-foreground whitespace-nowrap">
                SANKEY Copier
              </h1>
            )}
          </div>
        )}

        {/* Main header content */}
        <div className="flex items-center flex-1 px-6">
          {/* Mobile logo */}
          {isMobile && (
            <div className="flex items-center gap-2 flex-shrink-0">
              <Image
                src="/logo.png"
                alt="SANKEY Copier Logo"
                width={32}
                height={32}
                className="object-contain"
              />
              <h1 className="text-lg font-semibold text-foreground whitespace-nowrap">
                SANKEY Copier
              </h1>
            </div>
          )}

          {/* Spacer */}
          <div className="flex-1" />

          {/* Controls - Right aligned */}
          <div className="flex items-center gap-2 flex-shrink-0">
            <SiteSelector />
            <LanguageToggle />
            <ThemeToggle />
          </div>
        </div>
      </div>
    </header>
  );
}
