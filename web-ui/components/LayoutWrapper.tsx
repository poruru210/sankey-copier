'use client';

import { Sidebar } from './Sidebar';
import { ServerLog } from './ServerLog';

interface LayoutWrapperProps {
  children: React.ReactNode;
}

// Layout wrapper component with Sidebar and ServerLog
// Header is rendered separately at the page level
// Sidebar is fixed, so main content needs proper spacing
// ServerLog is available globally across all pages
export function LayoutWrapper({ children }: LayoutWrapperProps) {
  return (
    <>
      <Sidebar />
      {children}
      <ServerLog />
    </>
  );
}
