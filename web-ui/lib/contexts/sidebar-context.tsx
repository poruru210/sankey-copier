'use client';

// ServerLog context for sharing ServerLog state across components
// Sidebar UI state is now managed by shadcn SidebarProvider (components/ui/sidebar.tsx)
// This context only handles ServerLog expand/collapse and height for page layout adjustment

import { createContext, useContext, useState, ReactNode } from 'react';

interface ServerLogContextType {
  serverLogExpanded: boolean;
  setServerLogExpanded: (expanded: boolean) => void;
  serverLogHeight: number;
  setServerLogHeight: (height: number) => void;
}

const ServerLogContext = createContext<ServerLogContextType | undefined>(undefined);

export function ServerLogProvider({ children }: { children: ReactNode }) {
  const [serverLogExpanded, setServerLogExpanded] = useState(false);
  const [serverLogHeight, setServerLogHeight] = useState(40);

  return (
    <ServerLogContext.Provider
      value={{
        serverLogExpanded,
        setServerLogExpanded,
        serverLogHeight,
        setServerLogHeight,
      }}
    >
      {children}
    </ServerLogContext.Provider>
  );
}

export function useServerLogContext() {
  const context = useContext(ServerLogContext);
  if (context === undefined) {
    throw new Error('useServerLogContext must be used within a ServerLogProvider');
  }
  return context;
}
