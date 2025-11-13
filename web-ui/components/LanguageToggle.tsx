'use client';

import { useLocale } from 'next-intlayer';
import { usePathname, useRouter } from 'next/navigation';
import { Button } from '@/components/ui/button';
import { Languages } from 'lucide-react';

export function LanguageToggle() {
  const { locale, setLocale, pathWithoutLocale } = useLocale();
  const router = useRouter();

  const toggleLanguage = () => {
    const newLocale = locale === 'en' ? 'ja' : 'en';
    // Update locale in client context
    setLocale(newLocale);
    // Update URL for prefix-all routing mode
    router.push(`/${newLocale}${pathWithoutLocale}`);
  };

  return (
    <Button variant="ghost" size="icon" onClick={toggleLanguage} className="h-9 w-9">
      <Languages className="h-5 w-5" />
      <span className="sr-only">
        {locale === 'en' ? 'Switch to Japanese' : 'Switch to English'}
      </span>
    </Button>
  );
}
