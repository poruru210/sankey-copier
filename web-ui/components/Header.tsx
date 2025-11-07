'use client';

import { Menu } from 'lucide-react';
import { Button } from './ui/button';
import { LanguageToggle } from './LanguageToggle';
import { ThemeToggle } from './ThemeToggle';

interface HeaderProps {
  isMobile?: boolean;
  onOpenMobileFilter?: () => void;
}

export function Header({ isMobile, onOpenMobileFilter }: HeaderProps) {
  return (
    <header className="sticky top-0 z-50 w-full border-b bg-background">
      <div className="container mx-auto flex h-14 max-w-[1600px] items-center justify-between px-6">
        {/* App Name + Mobile Filter Button */}
        <div className="flex items-center gap-3">
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

          <h1 className="text-lg font-semibold text-foreground">SANKEY Copier</h1>
        </div>

        {/* Controls */}
        <div className="flex items-center gap-2">
          <LanguageToggle />
          <ThemeToggle />
        </div>
      </div>
    </header>
  );
}
