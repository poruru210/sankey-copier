'use client';

import { usePathname, useRouter } from 'next/navigation';
import { Button } from '@/components/ui/button';
import { Languages } from 'lucide-react';

export function LanguageToggle() {
  const pathname = usePathname();
  const router = useRouter();
  const currentLocale = pathname.split('/')[1];

  const toggleLanguage = () => {
    const newLocale = currentLocale === 'en' ? 'ja' : 'en';
    const newPath = pathname.replace(`/${currentLocale}`, `/${newLocale}`);
    router.push(newPath);
  };

  return (
    <Button variant="ghost" size="icon" onClick={toggleLanguage} className="h-9 w-9">
      <Languages className="h-5 w-5" />
      <span className="sr-only">
        {currentLocale === 'en' ? 'Switch to Japanese' : 'Switch to English'}
      </span>
    </Button>
  );
}
