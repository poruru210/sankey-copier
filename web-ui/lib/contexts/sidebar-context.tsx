'use client';

// ServerLog context for sharing ServerLog state across components
// Sidebar UI state is now managed by shadcn SidebarProvider (components/ui/sidebar.tsx)
// This context only handles ServerLog open/close state

import { createContext, useContext, useState, ReactNode } from 'react';

interface ServerLogContextType {
  serverLogExpanded: boolean;
  setServerLogExpanded: (expanded: boolean) => void;
}

const ServerLogContext = createContext<ServerLogContextType | undefined>(undefined);

export function ServerLogProvider({ children }: { children: ReactNode }) {
  const [serverLogExpanded, setServerLogExpanded] = useState(false);

  return (
    <ServerLogContext.Provider
      value={{
        serverLogExpanded,
        setServerLogExpanded,
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
