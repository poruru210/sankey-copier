'use client';

import { usePathname, useRouter } from 'next/navigation';
import { Button } from '@/components/ui/button';
import { Languages } from 'lucide-react';

export function LanguageSwitcher() {
  const pathname = usePathname();
  const router = useRouter();
  const currentLocale = pathname.split('/')[1];

  const toggleLanguage = () => {
    const newLocale = currentLocale === 'en' ? 'ja' : 'en';
    const newPath = pathname.replace(`/${currentLocale}`, `/${newLocale}`);
    router.push(newPath);
  };

  return (
    <Button variant="outline" size="sm" onClick={toggleLanguage}>
      <Languages className="h-4 w-4 mr-2" />
      {currentLocale === 'en' ? '日本語' : 'English'}
    </Button>
  );
}
