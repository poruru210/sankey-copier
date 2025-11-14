'use client';

import { Header } from './Header';
import { Sidebar } from './Sidebar';
import { ServerLog } from './ServerLog';

interface LayoutWrapperProps {
  children: React.ReactNode;
}

// Layout wrapper component with Header, Sidebar and ServerLog
// These components are rendered once at layout level to prevent remounting on page navigation
// Sidebar is fixed, so main content needs proper spacing
// ServerLog is available globally across all pages
export function LayoutWrapper({ children }: LayoutWrapperProps) {
  return (
    <>
      <Header />
      <Sidebar />
      {children}
      <ServerLog />
    </>
  );
}
