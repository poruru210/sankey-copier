'use client';

import { Menu, Settings } from 'lucide-react';
import Link from 'next/link';
import Image from 'next/image';
import { usePathname } from 'next/navigation';
import { Button } from './ui/button';
import { LanguageToggle } from './LanguageToggle';
import { ThemeToggle } from './ThemeToggle';
import { SiteSelector } from './SiteSelector';

interface HeaderProps {
  isMobile?: boolean;
  onOpenMobileFilter?: () => void;
}

export function Header({ isMobile, onOpenMobileFilter }: HeaderProps) {
  const pathname = usePathname();

  // Extract locale from pathname (e.g., /en/installations -> en)
  const locale = pathname.split('/')[1] || 'en';

  return (
    <header className="sticky top-0 z-50 w-full border-b bg-background">
      <div className="container mx-auto flex h-14 max-w-[1600px] items-center justify-between px-6">
        {/* App Name + Mobile Filter Button */}
        <div className="flex items-center gap-6">
          {/* Mobile Filter Button */}
          {isMobile && onOpenMobileFilter && (
            <Button
              variant="ghost"
              size="sm"
              onClick={onOpenMobileFilter}
              className="flex items-center gap-2 lg:hidden"
              aria-label="Open filter menu"
            >
              <Menu className="h-5 w-5" />
            </Button>
          )}

          <Link href={`/${locale}`} className="flex items-center gap-2">
            <Image
              src="/logo.png"
              alt="SANKEY Copier Logo"
              width={32}
              height={32}
              className="object-contain"
            />
            <h1 className="text-lg font-semibold text-foreground cursor-pointer hover:text-primary transition-colors">
              SANKEY Copier
            </h1>
          </Link>

          {/* Navigation */}
          <nav className="hidden md:flex items-center gap-1">
            <Link href={`/${locale}`}>
              <Button
                variant={pathname === `/${locale}` ? 'default' : 'ghost'}
                size="sm"
              >
                Connections
              </Button>
            </Link>
            <Link href={`/${locale}/installations`}>
              <Button
                variant={pathname.includes('/installations') ? 'default' : 'ghost'}
                size="sm"
                className="gap-2"
              >
                <Settings className="h-4 w-4" />
                Installations
              </Button>
            </Link>
          </nav>
        </div>

        {/* Controls */}
        <div className="flex items-center gap-2">
          <SiteSelector />
          <LanguageToggle />
          <ThemeToggle />
        </div>
      </div>
    </header>
  );
}
