// Layout for trade-group detail page
// Required to provide generateStaticParams for static export

import type { ReactNode } from 'react';

// Generate static params for static export
// Returns placeholder paths for each locale; actual IDs are resolved at runtime via client-side navigation
export function generateStaticParams() {
  return [
    { locale: 'en', id: '_placeholder' },
    { locale: 'ja', id: '_placeholder' },
  ];
}

export default function TradeGroupDetailLayout({
  children,
}: {
  children: ReactNode;
}) {
  return children;
}
