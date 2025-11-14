// Root page for Tauri Desktop App
// Redirects to default locale using client-side navigation
// This ensures index.html is generated for static export

'use client';

import { useEffect } from 'react';
import { useRouter } from 'next/navigation';

export default function RootPage() {
  const router = useRouter();
  
  useEffect(() => {
    // Get browser language (defaults to 'ja')
    const browserLang = navigator.language.toLowerCase();
    const locale = browserLang.startsWith('en') ? 'en' : 'ja';
    
    // Redirect to locale-specific page
    router.replace(`/${locale}`);
  }, [router]);
  
  // Show nothing while redirecting
  return null;
}
