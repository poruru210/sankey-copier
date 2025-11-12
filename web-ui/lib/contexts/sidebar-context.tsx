'use client';

import { createContext, useContext, useState, useEffect, ReactNode } from 'react';

// Context for sharing sidebar and ServerLog state across components
// Allows Header, ServerLog, and pages to adjust positioning and padding
interface SidebarContextType {
  isOpen: boolean;
  isMobile: boolean;
  setIsOpen: (open: boolean) => void;
  serverLogExpanded: boolean;
  setServerLogExpanded: (expanded: boolean) => void;
  serverLogHeight: number;
  setServerLogHeight: (height: number) => void;
}

const SidebarContext = createContext<SidebarContextType | undefined>(undefined);

export function SidebarProvider({ children }: { children: ReactNode }) {
  const [isOpen, setIsOpen] = useState(true);
  const [isMobile, setIsMobile] = useState(false);
  const [isMounted, setIsMounted] = useState(false);
  const [serverLogExpanded, setServerLogExpanded] = useState(false);
  const [serverLogHeight, setServerLogHeight] = useState(40);

  // Hydration fix
  useEffect(() => {
    setIsMounted(true);
  }, []);

  // Load sidebar state from localStorage on mount
  useEffect(() => {
    if (!isMounted) return;

    const savedState = localStorage.getItem('sidebar-open');
    if (savedState !== null) {
      setIsOpen(savedState === 'true');
    }
  }, [isMounted]);

  // Save sidebar state to localStorage when changed (PC only)
  useEffect(() => {
    if (!isMounted || isMobile) return;
    localStorage.setItem('sidebar-open', String(isOpen));
  }, [isOpen, isMobile, isMounted]);

  // Detect mobile viewport
  useEffect(() => {
    if (!isMounted) return;

    const checkMobile = () => {
      const mobile = window.innerWidth < 1024; // lg breakpoint
      setIsMobile(mobile);
      // On mobile, always close sidebar by default
      if (mobile) {
        setIsOpen(false);
      }
    };

    checkMobile();
    window.addEventListener('resize', checkMobile);
    return () => window.removeEventListener('resize', checkMobile);
  }, [isMounted]);

  return (
    <SidebarContext.Provider
      value={{
        isOpen,
        isMobile,
        setIsOpen,
        serverLogExpanded,
        setServerLogExpanded,
        serverLogHeight,
        setServerLogHeight,
      }}
    >
      {children}
    </SidebarContext.Provider>
  );
}

export function useSidebar() {
  const context = useContext(SidebarContext);
  if (context === undefined) {
    throw new Error('useSidebar must be used within a SidebarProvider');
  }
  return context;
}
