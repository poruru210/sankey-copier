'use client';

import { useState, useEffect } from 'react';
import { Menu } from 'lucide-react';
import { MasterAccountSidebar } from './MasterAccountSidebar';
import { Sheet, SheetContent } from './ui/sheet';
import { Button } from './ui/button';
import type { CopySettings, EaConnection } from '@/types';

interface MasterAccountSidebarContainerProps {
  connections: EaConnection[];
  settings: CopySettings[];
  selectedMaster: string | 'all';
  onSelectMaster: (masterId: string | 'all') => void;
}

export function MasterAccountSidebarContainer({
  connections,
  settings,
  selectedMaster,
  onSelectMaster,
}: MasterAccountSidebarContainerProps) {
  const [isMobileDrawerOpen, setIsMobileDrawerOpen] = useState(false);
  const [isMobile, setIsMobile] = useState(false);

  // Detect mobile screen size
  useEffect(() => {
    const checkMobile = () => {
      setIsMobile(window.innerWidth < 1024); // lg breakpoint
    };

    checkMobile();
    window.addEventListener('resize', checkMobile);
    return () => window.removeEventListener('resize', checkMobile);
  }, []);

  // Close drawer when selection changes on mobile
  const handleSelectMaster = (masterId: string | 'all') => {
    onSelectMaster(masterId);
    if (isMobile) {
      setIsMobileDrawerOpen(false);
    }
  };

  // Desktop: Fixed sidebar
  if (!isMobile) {
    return (
      <div className="w-60 flex-shrink-0">
        <MasterAccountSidebar
          connections={connections}
          settings={settings}
          selectedMaster={selectedMaster}
          onSelectMaster={onSelectMaster}
        />
      </div>
    );
  }

  // Mobile: Hamburger button + Drawer
  return (
    <>
      {/* Mobile hamburger button */}
      <div className="mb-4">
        <Button
          variant="outline"
          size="sm"
          onClick={() => setIsMobileDrawerOpen(true)}
          className="flex items-center gap-2"
        >
          <Menu className="h-4 w-4" />
          <span>Filter Accounts</span>
        </Button>
      </div>

      {/* Mobile drawer */}
      <Sheet open={isMobileDrawerOpen} onOpenChange={setIsMobileDrawerOpen} side="left">
        <SheetContent>
          <MasterAccountSidebar
            connections={connections}
            settings={settings}
            selectedMaster={selectedMaster}
            onSelectMaster={handleSelectMaster}
          />
        </SheetContent>
      </Sheet>
    </>
  );
}
