import type { Metadata } from 'next';
import { IntlayerClientProvider } from 'next-intlayer';
import { Inter } from 'next/font/google';
import { ThemeProvider } from '@/components/ThemeProvider';
import { SiteProvider } from '@/lib/contexts/site-context';
import { Toaster } from '@/components/ui/toaster';
import '../globals.css';

const inter = Inter({ subsets: ['latin'] });

export const metadata: Metadata = {
  title: 'SANKEY SANKEY Copier',
  description: 'MT4/MT5 Trade Copier with low latency and remote control',
};

export default async function RootLayout({
  children,
  params,
}: {
  children: React.ReactNode;
  params: Promise<{ locale: string }>;
}) {
  const { locale } = await params;

  return (
    <html lang={locale} suppressHydrationWarning>
      <body className={inter.className}>
        <ThemeProvider
          attribute="class"
          defaultTheme="system"
          enableSystem
          disableTransitionOnChange
        >
          <SiteProvider>
            <IntlayerClientProvider locale={locale}>
              {children}
              <Toaster />
            </IntlayerClientProvider>
          </SiteProvider>
        </ThemeProvider>
      </body>
    </html>
  );
}
