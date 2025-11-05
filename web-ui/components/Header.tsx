'use client';

import { LanguageToggle } from './LanguageToggle';
import { ThemeToggle } from './ThemeToggle';

export function Header() {
  return (
    <header className="sticky top-0 z-50 w-full border-b bg-background">
      <div className="container mx-auto flex h-14 max-w-[1600px] items-center justify-between px-6">
        {/* App Name */}
        <div className="flex items-center gap-2">
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
