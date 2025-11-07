'use client';

import { useState, useEffect } from 'react';
import { MasterAccountSidebar } from './MasterAccountSidebar';
import { Sheet, SheetContent } from './ui/sheet';
import type { CopySettings, EaConnection } from '@/types';

interface MasterAccountSidebarContainerProps {
  connections: EaConnection[];
  settings: CopySettings[];
  selectedMaster: string | 'all';
  onSelectMaster: (masterId: string | 'all') => void;
  isMobileDrawerOpen?: boolean;
  onCloseMobileDrawer?: () => void;
}

export function MasterAccountSidebarContainer({
  connections,
  settings,
  selectedMaster,
  onSelectMaster,
  isMobileDrawerOpen = false,
  onCloseMobileDrawer,
}: MasterAccountSidebarContainerProps) {
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
    if (isMobile && onCloseMobileDrawer) {
      onCloseMobileDrawer();
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

  // Mobile: Drawer only (button is in Header)
  return (
    <Sheet
      open={isMobileDrawerOpen}
      onOpenChange={(open) => {
        if (!open && onCloseMobileDrawer) {
          onCloseMobileDrawer();
        }
      }}
      side="left"
    >
      <SheetContent>
        <MasterAccountSidebar
          connections={connections}
          settings={settings}
          selectedMaster={selectedMaster}
          onSelectMaster={handleSelectMaster}
        />
      </SheetContent>
    </Sheet>
  );
}
